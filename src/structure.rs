#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct StreamHeader {
    pub msg_len: u16,
    pub stream_id: u16,
    pub seq_no: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct PeekStructure {
    pub global_header: StreamHeader,
    pub msg_type: u8,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct OrderMessage {
    pub msg_type: u8,
    pub exch_ts: u64,
    pub order_id: u64,
    pub token: u32,
    pub order_type: u8,
    pub price: u32,
    pub quantity: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TradeMessage {
    pub msg_type: u8,
    pub exch_ts: u64,
    pub buy_order_id: u64,
    pub sell_order_id: u64,
    pub token: i32,
    pub trade_price: i32,
    pub trade_quantity: i32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct OrderPacket {
    pub hdr: StreamHeader,
    pub ord: OrderMessage,
    pub local_ts: u64,
    pub flags: bool,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct TradePacket {
    pub hdr: StreamHeader,
    pub trd: TradeMessage,
    pub local_ts: u64,
    pub flags: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    Order(OrderPacket),
    Trade(TradePacket),
}
