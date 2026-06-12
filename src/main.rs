use std::collections::HashSet;
use std::io::Result;
use std::io::{Error, ErrorKind};
use std::mem::size_of;
use std::path::Path;
use std::time::Instant;

mod orderbook;
mod orderbook_processing;
mod read_trd_ord_only;
mod structure;
mod tsc;

use crate::orderbook::OrderBookManager;
use crate::read_trd_ord_only::read_trd_ord_only;
use crate::structure::{Message, OrderPacket, TradePacket};
use orderbook_processing::{clobber, cycle_end, cycle_start, Harness};

fn main() -> Result<()> {
    let _trade_size = size_of::<TradePacket>();
    let _order_size = size_of::<OrderPacket>();

    let args: Vec<String> = std::env::args().collect();
    const DEFAULT_PATH: &str = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_10_10_2026.bin";
    let path = if args.len() > 1 {
        &args[1]
    } else {
        DEFAULT_PATH
    };

    if !Path::new(path).exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!(
                "Input file not found: {}. Pass a valid file path as arg1. Default path: {}",
                path, DEFAULT_PATH
            ),
        ));
    }

    let filter_token: Option<u32> = if args.len() > 2 {
        Some(args[2].parse().map_err(|e| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Invalid token '{}': {}", args[2], e),
            )
        })?)
    } else {
        None
    };

    let max_messages: Option<usize> = if args.len() > 3 {
        Some(args[3].parse().map_err(|e| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Invalid max_messages '{}': {}", args[3], e),
            )
        })?)
    } else {
        None
    };

    let skip_bench = std::env::var("LOB_SKIP_BENCH")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    eprintln!("processing file: {}", path);
    if let Some(token) = filter_token {
        eprintln!("Filtering for token: {}", token);
    }
    if let Some(limit) = max_messages {
        eprintln!("Max messages to process: {}", limit);
    }
    if skip_bench {
        eprintln!("Benchmark phase disabled via LOB_SKIP_BENCH");
    }

    eprintln!("\nread_messages");
    let timer1 = Instant::now();
    let messages = read_trd_ord_only(path)?;
    let read_time = timer1.elapsed();
    eprintln!("Duration: {:.2?}", read_time);

    // Collect token statistics for diagnostics
    let mut order_tokens: HashSet<u32> = HashSet::new();
    let mut trade_tokens: HashSet<i32> = HashSet::new();
    for msg in &messages {
        match msg {
            Message::Order(order) => {
                order_tokens.insert(order.ord.token);
            }
            Message::Trade(trade) => {
                trade_tokens.insert(trade.trd.token);
            }
        }
    }

    eprintln!("\nfilter_messages");
    let timer2 = Instant::now();
    let mut filtered_messages: Vec<Message> = if let Some(filter) = filter_token {
        let mut out = Vec::with_capacity(messages.len() / 4);
        for msg in messages {
            match &msg {
                Message::Order(order) => {
                    if order.ord.token == filter {
                        out.push(msg);
                    }
                }
                Message::Trade(trade) => {
                    if trade.trd.token as u32 == filter {
                        out.push(msg);
                    }
                }
            }
        }
        out
    } else {
        messages
    };
    let filter_time = timer2.elapsed();

    if let Some(limit) = max_messages {
        if filtered_messages.len() > limit {
            filtered_messages.truncate(limit);
        }
    }

    eprintln!("Duration: {:.2?}", filter_time);
    eprintln!("Total filtered messages: {}", filtered_messages.len());

    if filtered_messages.is_empty() {
        eprintln!("No filtered messages to process. Exiting.");
        return Ok(());
    }

    eprintln!("\ninitialize_manager");
    let timer3 = Instant::now();
    let mut manager = OrderBookManager::new();
    let init_time = timer3.elapsed();
    eprintln!("initialize_manager duration: {:.2?}", init_time);

    println!(
        "local_ts,exch_ts,mid_price,\
bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,\
bid_price_1,bid_qty_1,ask_price_1,ask_qty_1,\
bid_price_2,bid_qty_2,ask_price_2,ask_qty_2,\
bid_price_3,bid_qty_3,ask_price_3,ask_qty_3,\
bid_price_4,bid_qty_4,ask_price_4,ask_qty_4"
    );

    eprintln!("\nprocess_all_messages");
    let timer4 = Instant::now();

    let mut message_count: u64 = 0;
    let mut order_count: u64 = 0;
    let mut trade_count: u64 = 0;

    for message in &filtered_messages {
        message_count += 1;
        match message {
            Message::Order(order) => {
                order_count += 1;
                manager.process_order_message(&order);

                let token = order.ord.token;
                if let Some((mid_price, mut bids, mut asks)) = manager.get_top_levels(token, 5) {
                    while bids.len() < 5 {
                        bids.push((0, 0));
                    }
                    while asks.len() < 5 {
                        asks.push((0, 0));
                    }
                    // Copy packed struct fields into aligned local variables
                    let local_ts = order.local_ts;
                    let exch_ts = order.ord.exch_ts;
                    println!(
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}, {}",
                        local_ts,
                        exch_ts,
                        mid_price,
                        bids[0].0,
                        bids[0].1,
                        asks[0].0,
                        asks[0].1,
                        bids[1].0,
                        bids[1].1,
                        asks[1].0,
                        asks[1].1,
                        bids[2].0,
                        bids[2].1,
                        asks[2].0,
                        asks[2].1,
                        bids[3].0,
                        bids[3].1,
                        asks[3].0,
                        asks[3].1,
                        bids[4].0,
                        bids[4].1,
                        asks[4].0,
                        asks[4].1
                    );
                }
            }
            Message::Trade(trade) => {
                trade_count += 1;
                manager.process_trade_message(&trade);

                let token = trade.trd.token as u32;
                if let Some((mid_price, mut bids, mut asks)) = manager.get_top_levels(token, 5) {
                    while bids.len() < 5 {
                        bids.push((0, 0));
                    }
                    while asks.len() < 5 {
                        asks.push((0, 0));
                    }
                    // Copy packed struct fields into aligned local variables
                    let local_ts = trade.local_ts;
                    let exch_ts = trade.trd.exch_ts;
                    println!(
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                        local_ts,
                        exch_ts,
                        mid_price,
                        bids[0].0,
                        bids[0].1,
                        asks[0].0,
                        asks[0].1,
                        bids[1].0,
                        bids[1].1,
                        asks[1].0,
                        asks[1].1,
                        bids[2].0,
                        bids[2].1,
                        asks[2].0,
                        asks[2].1,
                        bids[3].0,
                        bids[3].1,
                        asks[3].0,
                        asks[3].1,
                        bids[4].0,
                        bids[4].1,
                        asks[4].0,
                        asks[4].1
                    );
                }
            }
        }
    }

    let process_time = timer4.elapsed();
    eprintln!("Duration: {:.2?}", process_time);
    eprintln!(
        "Processed messages: total={}, orders={}, trades={}",
        message_count, order_count, trade_count
    );

    if skip_bench {
        return Ok(());
    }

    let mut harness = Harness::new();
    let filtered_messages_clone = filtered_messages.clone();
    harness.add_benchmark(
        "BM_RustOrderBook",
        filtered_messages_clone.len() as u64,
        move |iterations| -> u64 {
            let mut total_cycles: u64 = 0;
            for _ in 0..iterations {
                let mut book = OrderBookManager::new();
                let start = cycle_start();
                clobber();
                filtered_messages_clone
                    .iter()
                    .for_each(|message| match message {
                        Message::Order(order) => {
                            book.process_order_message(order);
                        }
                        Message::Trade(trade) => {
                            book.process_trade_message(trade);
                        }
                    });
                let end = cycle_end();
                total_cycles = total_cycles.saturating_add(end.saturating_sub(start));
            }
            total_cycles
        },
        21,
    );
    std::process::exit(harness.run());
}
