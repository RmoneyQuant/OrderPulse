use rustc_hash::FxHashMap;
use crate::structure::{OrderPacket, TradePacket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy)]
struct Order {
    side: Side,
    price: u32,
    qty: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct PriceLevel {
    pub price: u32,
    pub total_qty: u64,
}

const DEFAULT_PRICE_TICK_SIZE: u32 = 1;
const INITIAL_DPR_WINDOW_WIDTH: u32 = 20_000;

#[derive(Debug)]
pub(crate) struct OrderBook {
    orders: FxHashMap<u64, Order>,
    tick_size: u32,
    dpr_min: u32,
    dpr_max: u32,
    bid_levels: Vec<u64>,
    ask_levels: Vec<u64>,
}

impl OrderBook {
    #[inline]
    fn new(first_seen_price: u32) -> Self {
        let mut orders = FxHashMap::default();
        orders.reserve(100_000);

        let tick_size = DEFAULT_PRICE_TICK_SIZE;
        let initial_half_window = INITIAL_DPR_WINDOW_WIDTH / 2;

        let mut dpr_min = first_seen_price.saturating_sub(initial_half_window);
        let mut dpr_max = first_seen_price.saturating_add(initial_half_window);

        dpr_min = (dpr_min / tick_size) * tick_size;
        dpr_max = if dpr_max % tick_size == 0 {
            dpr_max
        } else {
            ((dpr_max / tick_size) + 1) * tick_size
        };

        if dpr_max < dpr_min {
            dpr_max = dpr_min;
        }

        let levels = ((dpr_max - dpr_min) / tick_size + 1) as usize;
        Self {
            orders,
            tick_size,
            dpr_min,
            dpr_max,
            bid_levels: vec![0u64; levels],
            ask_levels: vec![0u64; levels],
        }
    }

    #[inline(always)]
    fn in_range(&self, price: u32) -> bool {
        price >= self.dpr_min && price <= self.dpr_max
    }

    #[inline(always)]
    fn idx(&self, price: u32) -> usize {
        ((price - self.dpr_min) / self.tick_size) as usize
    }

    #[inline(always)]
    fn align_down_to_tick(&self, p: u32) -> u32 {
        (p / self.tick_size) * self.tick_size
    }

    #[inline(always)]
    fn round_up_to_10(p: u32) -> u32 {
        if p % 10 == 0 {
            p
        } else {
            p + (10 - (p % 10))
        }
    }

    fn load_new_dpr_settings(&mut self, price: u32) {
        let old_min = self.dpr_min;
        let mut new_min = self.dpr_min;
        let mut new_max = self.dpr_max;

        if price >= new_max {
            new_max = price.saturating_add(price / 4);
            new_max = Self::round_up_to_10(new_max);
            new_max = self.align_down_to_tick(new_max);
        } else if price < new_min {
            new_min = price.saturating_sub(price / 4);
            let diff = old_min.saturating_sub(new_min);
            new_min = new_min.saturating_sub(diff % 10);
            new_min = self.align_down_to_tick(new_min);
            if new_min == 0 {
                new_min = self.tick_size;
            }
        } else {
            return;
        }

        if new_max < new_min {
            new_max = new_min;
        }

        let new_levels = ((new_max - new_min) / self.tick_size + 1) as usize;
        let mut new_bid = vec![0u64; new_levels];
        let mut new_ask = vec![0u64; new_levels];

        for old_i in 0..self.bid_levels.len() {
            let abs_price = old_min + (old_i as u32 * self.tick_size);
            if abs_price < new_min || abs_price > new_max {
                continue;
            }
            let new_i = ((abs_price - new_min) / self.tick_size) as usize;
            new_bid[new_i] = self.bid_levels[old_i];
            new_ask[new_i] = self.ask_levels[old_i];
        }

        self.dpr_min = new_min;
        self.dpr_max = new_max;
        self.bid_levels = new_bid;
        self.ask_levels = new_ask;
    }

    #[inline(always)]
    fn ensure_price(&mut self, price: u32) {
        if !self.in_range(price) {
            self.load_new_dpr_settings(price);
        }
    }

    #[inline(always)]
    fn add_order(&mut self, order_id: u64, side: Side, price: u32, qty: u32) {
        if qty == 0 {
            return;
        }
        self.ensure_price(price);
        if !self.in_range(price) {
            return;
        }
        let i = self.idx(price);
        self.orders.insert(order_id, Order { side, price, qty });

        unsafe {
            match side {
                Side::Buy => *self.bid_levels.get_unchecked_mut(i) += qty as u64,
                Side::Sell => *self.ask_levels.get_unchecked_mut(i) += qty as u64,
            }
        }
    }

    #[inline(always)]
    fn reduce_order(&mut self, order_id: u64, qty: u32) {
        let (side, price, traded, remaining_qty) = {
            let order = match self.orders.get_mut(&order_id) {
                Some(o) => o,
                None => return,
            };
            let traded = qty.min(order.qty);
            order.qty -= traded;
            (order.side, order.price, traded, order.qty)
        };
        if traded > 0 && self.in_range(price) {
            let i = self.idx(price);
            unsafe {
                match side {
                    Side::Buy => *self.bid_levels.get_unchecked_mut(i) -= traded as u64,
                    Side::Sell => *self.ask_levels.get_unchecked_mut(i) -= traded as u64,
                }
            }
        }
        if remaining_qty == 0 {
            self.orders.remove(&order_id);
        }
    }

    #[inline(always)]
    fn cancel_order(&mut self, order_id: u64) {
        let order = match self.orders.remove(&order_id) {
            Some(o) => o,
            None => return,
        };
        if self.in_range(order.price) {
            let i = self.idx(order.price);
            unsafe {
                match order.side {
                    Side::Buy => *self.bid_levels.get_unchecked_mut(i) -= order.qty as u64,
                    Side::Sell => *self.ask_levels.get_unchecked_mut(i) -= order.qty as u64,
                }
            }
        }
    }

    #[inline(always)]
    fn modify_order(&mut self, order_id: u64, side: Side, new_price: u32, new_qty: u32) {
        self.cancel_order(order_id);
        self.add_order(order_id, side, new_price, new_qty);
    }

    #[inline(always)]
    pub(crate) fn bid_levels(&self) -> &[u64] {
        &self.bid_levels
    }

    #[inline(always)]
    pub(crate) fn ask_levels(&self) -> &[u64] {
        &self.ask_levels
    }
}

pub struct OrderBookManager {
    books: FxHashMap<u32, OrderBook>,
}

impl OrderBookManager {
    pub fn reset(&mut self) {
        self.books.clear();
    }

    #[inline]
    pub fn new() -> Self {
        let mut books = FxHashMap::default();
        books.reserve(20_000);
        Self { books }
    }

    #[inline(always)]
    fn book_mut(&mut self, token: u32, first_seen_price: u32) -> &mut OrderBook {
        self.books
            .entry(token)
            .or_insert_with(|| OrderBook::new(first_seen_price))
    }

    #[inline(always)]
    pub fn add_order(&mut self, token: u32, order_id: u64, side: Side, price: u32, qty: u32) {
         self.book_mut(token, price).add_order(order_id, side, price, qty);
    }

    #[inline(always)]
    pub fn reduce_order(&mut self, token: u32, order_id: u64, qty: u32) {
        if let Some(book) = self.books.get_mut(&token) {
            book.reduce_order(order_id, qty);
        }
    }

    #[inline(always)]
    pub fn cancel_order(&mut self, token: u32, order_id: u64) {
        if let Some(book) = self.books.get_mut(&token) {
            book.cancel_order(order_id);
        }
    }

    #[inline(always)]
    pub fn modify_order(
        &mut self,
        token: u32,
        order_id: u64,
        new_price: u32,
        new_qty: u32,
        side: Side,
    ) {
        self.book_mut(token, new_price)
            .modify_order(order_id, side, new_price, new_qty);
    }

    #[inline(always)]
    pub(crate) fn get_book(&self, token: u32) -> Option<&OrderBook> {
        self.books.get(&token)
    }

    pub fn active_tokens(&self) -> Vec<u32> {
        let mut tokens: Vec<u32> = self.books.keys().copied().collect();
        tokens.sort_unstable();
        tokens
    }



    #[inline(always)]
    pub fn process_order_message(&mut self, order: &OrderPacket) {
        let token = order.ord.token;
        let order_id = order.ord.order_id;
        let price = order.ord.price;
        let qty = order.ord.quantity;
        let order_type = order.ord.order_type;
        let msg_type = order.ord.msg_type;

        let side = if order_type == b'B' {
            Side::Buy
        } else {
            Side::Sell
        };

        match msg_type {
            b'N' => self.add_order(token, order_id, side, price, qty),
            b'M' => self.modify_order(token, order_id, price, qty, side),
            b'X' => self.cancel_order(token, order_id),
            _ => {}
        }
    }

    #[inline(always)]
    pub fn process_trade_message(&mut self, trade: &TradePacket) {
        let token = trade.trd.token as u32;
        let buy_order_id = trade.trd.buy_order_id;
        let sell_order_id = trade.trd.sell_order_id;
        let qty = trade.trd.trade_quantity.max(0) as u32;

        self.reduce_order(token, buy_order_id, qty);
        self.reduce_order(token, sell_order_id, qty);
    }

    /// Return `mid_price`, `Vec` of top N bid levels, `Vec` of top N ask levels, for a given token.
    /// Each `Vec` element is `(price, qty)`, ordered best-to-worst.
    pub fn get_top_levels(
        &self,
        token: u32,
        levels: usize,
    ) -> Option<(u32, Vec<(u32, u64)>, Vec<(u32, u64)>)> {
        let book = self.books.get(&token)?;
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        // Bids: iterate highest to lowest price (right to left)
        for (i, &qty) in book.bid_levels.iter().enumerate().rev() {
            if qty > 0 {
                let price = book.dpr_min + (i as u32) * book.tick_size;
                bids.push((price, qty));
            }
            if bids.len() >= levels {
                break;
            }
        }
        // Asks: iterate lowest to highest price (left to right)
        for (i, &qty) in book.ask_levels.iter().enumerate() {
            if qty > 0 {
                let price = book.dpr_min + (i as u32) * book.tick_size;
                asks.push((price, qty));
            }
            if asks.len() >= levels {
                break;
            }
        }
        let mid_price = if let (Some((b, _)), Some((a, _))) = (bids.first(), asks.first()) {
            (b + a) / 2
        } else {
            0
        };
        Some((mid_price, bids, asks))
    }

    /// Return all non-zero bid/ask levels for a token as full market depth.
    pub fn get_full_depth(&self, token: u32) -> Option<(Vec<(u32, u64)>, Vec<(u32, u64)>)> {
        let book = self.books.get(&token)?;
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        for (i, &qty) in book.bid_levels.iter().enumerate().rev() {
            if qty > 0 {
                let price = book.dpr_min + (i as u32) * book.tick_size;
                bids.push((price, qty));
            }
        }

        for (i, &qty) in book.ask_levels.iter().enumerate() {
            if qty > 0 {
                let price = book.dpr_min + (i as u32) * book.tick_size;
                asks.push((price, qty));
            }
        }

        Some((bids, asks))
    }
} 