mod orderbook;
mod orderbook_processing;
mod read_trd_ord_only;
mod structure;
mod tsc;
mod contStruct;

use contStruct::FeedFilePath;

use std::collections::HashMap;
use std::sync::Arc;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;

use pyo3::exceptions::{PyIndexError, PyRuntimeError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyModule};

pub use orderbook::{OrderBookManager, PriceLevel, Side};
pub use structure::{Message, OrderPacket, PeekStructure, TradePacket};
pub use read_trd_ord_only::read_trd_ord_only;

fn format_message(message: &Message) -> String {
    match message {
        Message::Order(order_packet) => unsafe {
            let seq_no = std::ptr::addr_of!(order_packet.hdr.seq_no).read_unaligned();
            let msg_len = std::ptr::addr_of!(order_packet.hdr.msg_len).read_unaligned();
            let msg_type = std::ptr::addr_of!(order_packet.ord.msg_type).read_unaligned();
            let exch_ts = std::ptr::addr_of!(order_packet.ord.exch_ts).read_unaligned();
            let local_ts = std::ptr::addr_of!(order_packet.local_ts).read_unaligned();
            let order_id = std::ptr::addr_of!(order_packet.ord.order_id).read_unaligned();
            let token = std::ptr::addr_of!(order_packet.ord.token).read_unaligned();
            let order_type = std::ptr::addr_of!(order_packet.ord.order_type).read_unaligned();
            let price = std::ptr::addr_of!(order_packet.ord.price).read_unaligned();
            let quantity = std::ptr::addr_of!(order_packet.ord.quantity).read_unaligned();
            let flags = std::ptr::addr_of!(order_packet.flags).read_unaligned();

            format!(
                "Order Message: SeqNo {}, MsgLen {}, MsgType '{}', ExchTs {}, LocalTs {}, OrderId {}, Token {}, Side '{}', Price {}, Quantity {}, Missed {}",
                seq_no,
                msg_len,
                msg_type as char,
                exch_ts,
                local_ts,
                order_id,
                token,
                order_type as char,
                price,
                quantity,
                if flags { 1 } else { 0 }
            )
        },

        Message::Trade(trade_packet) => unsafe {
            let seq_no = std::ptr::addr_of!(trade_packet.hdr.seq_no).read_unaligned();
            let msg_len = std::ptr::addr_of!(trade_packet.hdr.msg_len).read_unaligned();
            let msg_type = std::ptr::addr_of!(trade_packet.trd.msg_type).read_unaligned();
            let exch_ts = std::ptr::addr_of!(trade_packet.trd.exch_ts).read_unaligned();
            let local_ts = std::ptr::addr_of!(trade_packet.local_ts).read_unaligned();
            let buy_order_id = std::ptr::addr_of!(trade_packet.trd.buy_order_id).read_unaligned();
            let sell_order_id = std::ptr::addr_of!(trade_packet.trd.sell_order_id).read_unaligned();
            let token = std::ptr::addr_of!(trade_packet.trd.token).read_unaligned();
            let trade_price = std::ptr::addr_of!(trade_packet.trd.trade_price).read_unaligned();
            let trade_quantity = std::ptr::addr_of!(trade_packet.trd.trade_quantity).read_unaligned();
            let flags = std::ptr::addr_of!(trade_packet.flags).read_unaligned();

            format!(
                "Trade Message: SeqNo {}, MsgLen {}, MsgType '{}', ExchTs {}, LocalTs {}, BuyOrderId {}, SellOrderId {}, Token {}, Price {}, Quantity {}, Missed {}",
                seq_no,
                msg_len,
                msg_type as char,
                exch_ts,
                local_ts,
                buy_order_id,
                sell_order_id,
                token,
                trade_price,
                trade_quantity,
                if flags { 1 } else { 0 }
            )
        },
    }
}


fn message_to_py_dict(py: Python<'_>, message: &Message) -> PyResult<Py<PyAny>> {
    let dict = PyDict::new_bound(py);

    match message {
        Message::Order(order_packet) => unsafe {
            let seq_no = std::ptr::addr_of!(order_packet.hdr.seq_no).read_unaligned();
            let msg_len = std::ptr::addr_of!(order_packet.hdr.msg_len).read_unaligned();
            let stream_id = std::ptr::addr_of!(order_packet.hdr.stream_id).read_unaligned();
            let msg_type = std::ptr::addr_of!(order_packet.ord.msg_type).read_unaligned();
            let exch_ts = std::ptr::addr_of!(order_packet.ord.exch_ts).read_unaligned();
            let local_ts = std::ptr::addr_of!(order_packet.local_ts).read_unaligned();
            let order_id = std::ptr::addr_of!(order_packet.ord.order_id).read_unaligned();
            let token = std::ptr::addr_of!(order_packet.ord.token).read_unaligned();
            let order_type = std::ptr::addr_of!(order_packet.ord.order_type).read_unaligned();
            let price = std::ptr::addr_of!(order_packet.ord.price).read_unaligned();
            let quantity = std::ptr::addr_of!(order_packet.ord.quantity).read_unaligned();
            let flags = std::ptr::addr_of!(order_packet.flags).read_unaligned();

            dict.set_item("message_kind", "order")?;
            dict.set_item("seq_no", seq_no)?;
            dict.set_item("msg_len", msg_len)?;
            dict.set_item("stream_id", stream_id)?;
            dict.set_item("msg_type", (msg_type as char).to_string())?;
            dict.set_item("exch_ts", exch_ts)?;
            dict.set_item("local_ts", local_ts)?;
            dict.set_item("order_id", order_id)?;
            dict.set_item("token", token)?;
            dict.set_item("order_type", (order_type as char).to_string())?;
            dict.set_item("price", price)?;
            dict.set_item("quantity", quantity)?;
            dict.set_item("flags", flags)?;
            dict.set_item("token_symbol", py.None())?;
            dict.set_item("strike_price", py.None())?;
            dict.set_item("option_type", py.None())?;
        },

        Message::Trade(trade_packet) => unsafe {
            let seq_no = std::ptr::addr_of!(trade_packet.hdr.seq_no).read_unaligned();
            let msg_len = std::ptr::addr_of!(trade_packet.hdr.msg_len).read_unaligned();
            let stream_id = std::ptr::addr_of!(trade_packet.hdr.stream_id).read_unaligned();
            let msg_type = std::ptr::addr_of!(trade_packet.trd.msg_type).read_unaligned();
            let exch_ts = std::ptr::addr_of!(trade_packet.trd.exch_ts).read_unaligned();
            let local_ts = std::ptr::addr_of!(trade_packet.local_ts).read_unaligned();
            let buy_order_id = std::ptr::addr_of!(trade_packet.trd.buy_order_id).read_unaligned();
            let sell_order_id = std::ptr::addr_of!(trade_packet.trd.sell_order_id).read_unaligned();
            let token = std::ptr::addr_of!(trade_packet.trd.token).read_unaligned();
            let trade_price = std::ptr::addr_of!(trade_packet.trd.trade_price).read_unaligned();
            let trade_quantity = std::ptr::addr_of!(trade_packet.trd.trade_quantity).read_unaligned();
            let flags = std::ptr::addr_of!(trade_packet.flags).read_unaligned();

            dict.set_item("message_kind", "trade")?;
            dict.set_item("seq_no", seq_no)?;
            dict.set_item("msg_len", msg_len)?;
            dict.set_item("stream_id", stream_id)?;
            dict.set_item("msg_type", (msg_type as char).to_string())?;
            dict.set_item("exch_ts", exch_ts)?;
            dict.set_item("local_ts", local_ts)?;
            dict.set_item("buy_order_id", buy_order_id)?;
            dict.set_item("sell_order_id", sell_order_id)?;
            dict.set_item("token", token)?;
            dict.set_item("trade_price", trade_price)?;
            dict.set_item("trade_quantity", trade_quantity)?;
            dict.set_item("flags", flags)?;
            dict.set_item("token_symbol", py.None())?;
            dict.set_item("strike_price", py.None())?;
            dict.set_item("option_type", py.None())?;
        },
    }

    Ok(dict.into_any().unbind())
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct CachedMessage {
    #[pyo3(get)]
    message_kind: String,
    #[pyo3(get)]
    seq_no: u32,
    #[pyo3(get)]
    msg_len: u16,
    #[pyo3(get)]
    stream_id: u16,
    #[pyo3(get)]
    msg_type: String,
    #[pyo3(get)]
    exch_ts: u64,
    #[pyo3(get)]
    local_ts: u64,
    #[pyo3(get)]
    flags: bool,
    #[pyo3(get)]
    token: i64,
    #[pyo3(get)]
    order_type: Option<String>,
    #[pyo3(get)]
    order_id: Option<u64>,
    #[pyo3(get)]
    price: Option<i64>,
    #[pyo3(get)]
    quantity: Option<i64>,
    #[pyo3(get)]
    buy_order_id: Option<u64>,
    #[pyo3(get)]
    sell_order_id: Option<u64>,
    #[pyo3(get)]
    trade_price: Option<i64>,
    #[pyo3(get)]
    trade_quantity: Option<i64>,
}

fn message_to_cached_message(message: &Message) -> CachedMessage {
    match message {
        Message::Order(order_packet) => unsafe {
            let seq_no = std::ptr::addr_of!(order_packet.hdr.seq_no).read_unaligned();
            let msg_len = std::ptr::addr_of!(order_packet.hdr.msg_len).read_unaligned();
            let stream_id = std::ptr::addr_of!(order_packet.hdr.stream_id).read_unaligned();
            let msg_type = std::ptr::addr_of!(order_packet.ord.msg_type).read_unaligned();
            let exch_ts = std::ptr::addr_of!(order_packet.ord.exch_ts).read_unaligned();
            let local_ts = std::ptr::addr_of!(order_packet.local_ts).read_unaligned();
            let order_id = std::ptr::addr_of!(order_packet.ord.order_id).read_unaligned();
            let token = std::ptr::addr_of!(order_packet.ord.token).read_unaligned();
            let order_type = std::ptr::addr_of!(order_packet.ord.order_type).read_unaligned();
            let price = std::ptr::addr_of!(order_packet.ord.price).read_unaligned();
            let quantity = std::ptr::addr_of!(order_packet.ord.quantity).read_unaligned();
            let flags = std::ptr::addr_of!(order_packet.flags).read_unaligned();

            CachedMessage {
                message_kind: "order".to_string(),
                seq_no,
                msg_len,
                stream_id,
                msg_type: (msg_type as char).to_string(),
                exch_ts,
                local_ts,
                flags,
                token: token as i64,
                order_type: Some((order_type as char).to_string()),
                order_id: Some(order_id),
                price: Some(price as i64),
                quantity: Some(quantity as i64),
                buy_order_id: None,
                sell_order_id: None,
                trade_price: None,
                trade_quantity: None,
            }
        },
        Message::Trade(trade_packet) => unsafe {
            let seq_no = std::ptr::addr_of!(trade_packet.hdr.seq_no).read_unaligned();
            let msg_len = std::ptr::addr_of!(trade_packet.hdr.msg_len).read_unaligned();
            let stream_id = std::ptr::addr_of!(trade_packet.hdr.stream_id).read_unaligned();
            let msg_type = std::ptr::addr_of!(trade_packet.trd.msg_type).read_unaligned();
            let exch_ts = std::ptr::addr_of!(trade_packet.trd.exch_ts).read_unaligned();
            let local_ts = std::ptr::addr_of!(trade_packet.local_ts).read_unaligned();
            let buy_order_id = std::ptr::addr_of!(trade_packet.trd.buy_order_id).read_unaligned();
            let sell_order_id = std::ptr::addr_of!(trade_packet.trd.sell_order_id).read_unaligned();
            let token = std::ptr::addr_of!(trade_packet.trd.token).read_unaligned();
            let trade_price = std::ptr::addr_of!(trade_packet.trd.trade_price).read_unaligned();
            let trade_quantity = std::ptr::addr_of!(trade_packet.trd.trade_quantity).read_unaligned();
            let flags = std::ptr::addr_of!(trade_packet.flags).read_unaligned();

            CachedMessage {
                message_kind: "trade".to_string(),
                seq_no,
                msg_len,
                stream_id,
                msg_type: (msg_type as char).to_string(),
                exch_ts,
                local_ts,
                flags,
                token: token as i64,
                order_type: None,
                order_id: None,
                price: None,
                quantity: None,
                buy_order_id: Some(buy_order_id),
                sell_order_id: Some(sell_order_id),
                trade_price: Some(trade_price as i64),
                trade_quantity: Some(trade_quantity as i64),
            }
        },
    }
}

fn format_snapshot_row(
    local_ts: u64,
    exch_ts: u64,
    mid_price: u32,
    mut bids: Vec<(u32, u64)>,
    mut asks: Vec<(u32, u64)>,
) -> String {
    while bids.len() < 5 {
        bids.push((0, 0));
    }
    while asks.len() < 5 {
        asks.push((0, 0));
    }

    format!(
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
    )
}
// ─── Symbol master data types ────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ContractInfo {
    symbol:      String,   // TckrSymb   — e.g. "NIFTY"
    name:        String,   // StockNm    — e.g. "NIFTY2660921350CE"
    option_type: String,   // OptnTp     — "CE" | "PE" | "XX"
    strike:      i64,      // StrkPric / 100 (rupees; -1 for futures)
    expiry:      String,   // formatted from XpryDt unix timestamp
    lot_size:    u32,      // NewBrdLotQty
}

fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Convert a unix-seconds timestamp to a human-readable date string like "26-May-2026".
fn unix_ts_to_date(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let mut d = ts / 86_400;
    let mut yr = 1970i64;
    loop {
        let diy = if is_leap_year(yr) { 366 } else { 365 };
        if d < diy { break; }
        d -= diy;
        yr += 1;
    }
    let dim: [i64; 12] = [
        31,
        if is_leap_year(yr) { 29 } else { 28 },
        31, 30, 31, 30, 31, 31, 30, 31, 30, 31,
    ];
    let mn = ["Jan","Feb","Mar","Apr","May","Jun",
               "Jul","Aug","Sep","Oct","Nov","Dec"];
    let mut mo = 0usize;
    for (i, &days) in dim.iter().enumerate() {
        if d < days { mo = i; break; }
        d -= days;
    }
    format!("{:02}-{}-{}", d + 1, mn[mo], yr)
}

fn parse_order_packet(bytes: &[u8]) -> OrderPacket {
    let mut packet: OrderPacket = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const _) };

    packet.hdr.msg_len = u16::from_le(packet.hdr.msg_len);
    packet.hdr.stream_id = u16::from_le(packet.hdr.stream_id);
    packet.hdr.seq_no = u32::from_le(packet.hdr.seq_no);

    packet.ord.exch_ts = u64::from_le(packet.ord.exch_ts);
    packet.ord.order_id = u64::from_le(packet.ord.order_id);
    packet.ord.token = u32::from_le(packet.ord.token);
    packet.ord.price = u32::from_le(packet.ord.price);
    packet.ord.quantity = u32::from_le(packet.ord.quantity);
    packet.local_ts = u64::from_le(packet.local_ts);

    packet
}

fn parse_trade_packet(bytes: &[u8]) -> TradePacket {
    let mut packet: TradePacket = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const _) };

    packet.hdr.msg_len = u16::from_le(packet.hdr.msg_len);
    packet.hdr.stream_id = u16::from_le(packet.hdr.stream_id);
    packet.hdr.seq_no = u32::from_le(packet.hdr.seq_no);

    packet.trd.exch_ts = u64::from_le(packet.trd.exch_ts);
    packet.trd.buy_order_id = u64::from_le(packet.trd.buy_order_id);
    packet.trd.sell_order_id = u64::from_le(packet.trd.sell_order_id);
    packet.trd.token = i32::from_le(packet.trd.token);
    packet.trd.trade_price = i32::from_le(packet.trd.trade_price);
    packet.trd.trade_quantity = i32::from_le(packet.trd.trade_quantity);
    packet.local_ts = u64::from_le(packet.local_ts);

    packet
}

fn validate_binary_header(file: &mut File) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(0))?;

    loop {
        let mut first = [0u8; 1];
        match file.read_exact(&mut first) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "file is empty or missing a complete message header",
                ));
            }
            Err(err) => return Err(err),
        }

        if first[0] == b' ' {
            continue;
        }

        let mut rest = [0u8; size_of::<PeekStructure>() - 1];
        match file.read_exact(&mut rest) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "truncated first message header",
                ));
            }
            Err(err) => return Err(err),
        }

        let mut peek_bytes = [0u8; size_of::<PeekStructure>()];
        peek_bytes[0] = first[0];
        peek_bytes[1..].copy_from_slice(&rest);
        let peek: PeekStructure = unsafe {
            std::ptr::read_unaligned(peek_bytes.as_ptr() as *const _)
        };

        match peek.msg_type {
            b'T' | b'N' | b'M' | b'X' => {
                file.seek(SeekFrom::Start(0))?;
                return Ok(());
            }
            invalid => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid first message type: {}", invalid),
                ));
            }
        }
    }
}
fn read_next_message_from_file(file: &mut File) -> std::io::Result<Option<Message>> {
    loop {
        let mut first = [0u8; 1];
        match file.read_exact(&mut first) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(err) => return Err(err),
        }

        if first[0] == b' ' {
            continue;
        }

        let mut rest = [0u8; size_of::<PeekStructure>() - 1];
        match file.read_exact(&mut rest) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "truncated message header",
                ));
            }
            Err(err) => return Err(err),
        }

        let mut peek_bytes = [0u8; size_of::<PeekStructure>()];
        peek_bytes[0] = first[0];
        peek_bytes[1..].copy_from_slice(&rest);
        let peek: PeekStructure = unsafe {
            std::ptr::read_unaligned(peek_bytes.as_ptr() as *const _)
        };

        let packet_size = match peek.msg_type {
            b'T' => size_of::<TradePacket>(),
            b'N' | b'M' | b'X' => size_of::<OrderPacket>(),
            _ => {
                // Unknown message type — use msg_len to skip over it
                let skip = (peek.global_header.msg_len as usize)
                    .saturating_sub(size_of::<PeekStructure>());
                let mut discard = vec![0u8; skip];
                file.read_exact(&mut discard)?;
                continue;
            }
        };

        let mut packet_bytes = vec![0u8; packet_size];
        packet_bytes[..size_of::<PeekStructure>()].copy_from_slice(&peek_bytes);

        let remaining = packet_size - size_of::<PeekStructure>();
        match file.read_exact(&mut packet_bytes[size_of::<PeekStructure>()..]) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "truncated message payload",
                ));
            }
            Err(err) => return Err(err),
        }

        if remaining == 0 {
            return Ok(None);
        }

        let msg = match peek.msg_type {
            b'T' => Message::Trade(parse_trade_packet(&packet_bytes)),
            b'N' | b'M' | b'X' => Message::Order(parse_order_packet(&packet_bytes)),
            _ => unreachable!(),
        };

        return Ok(Some(msg));
    }
}

fn count_messages_in_file(path: &str) -> std::io::Result<usize> {
    let mut file = File::open(path)?;
    validate_binary_header(&mut file)?;
    file.seek(SeekFrom::Start(0))?;
    let mut count = 0usize;

    while let Some(_msg) = read_next_message_from_file(&mut file)? {
        count += 1;
    }

    Ok(count)
}

#[pyclass]
pub struct MessageCacheReader {
    file_path: Option<String>,
    messages: Arc<Vec<Message>>,
}

#[pymethods]
impl MessageCacheReader {
    #[new]
    pub fn new() -> Self {
        Self {
            file_path: None,
            messages: Arc::new(Vec::new()),
        }
    }

    pub fn load_to_cache(&mut self, file_path: String) -> PyResult<usize> {
        let messages = read_trd_ord_only::read_trd_ord_only(&file_path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;

        let count = messages.len();

        self.file_path = Some(file_path);
        self.messages = Arc::new(messages);

        Ok(count)
    }

    pub fn get_all_messages(&self) -> Vec<CachedMessage> {
        self.messages.iter().map(message_to_cached_message).collect()
    }

    #[getter]
    pub fn messages(&self) -> Vec<CachedMessage> {
    self.get_all_messages()
}

pub fn __len__(&self) -> usize {
    self.messages.len()
}

pub fn __getitem__(&self, index: isize) -> PyResult<CachedMessage> {
    let len = self.messages.len() as isize;

    if len == 0 {
        return Err(PyIndexError::new_err("message cache is empty"));
    }

    let resolved = if index < 0 { len + index } else { index };

    if resolved < 0 || resolved >= len {
        return Err(PyIndexError::new_err(format!(
            "message index {} out of range for cache of size {}",
            index, len
        )));
    }

    Ok(message_to_cached_message(&self.messages[resolved as usize]))
}


    pub fn get_order_message(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.messages
            .iter()
            .filter(|message| matches!(message, Message::Order(_)))
            .map(|message| message_to_py_dict(py, message))
            .collect()
    }

    pub fn get_trade_message(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.messages
            .iter()
            .filter(|message| matches!(message, Message::Trade(_)))
            .map(|message| message_to_py_dict(py, message))
            .collect()
    }

    pub fn get_all_trade_message(&self, py: Python<'_>) -> PyResult<Vec<Py<PyAny>>> {
        self.get_trade_message(py)
    }

    pub fn get_cache_summary(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new_bound(py);

        let total_messages = self.messages.len();

        let total_orders = self
            .messages
            .iter()
            .filter(|message| matches!(message, Message::Order(_)))
            .count();

        let total_trades = self
            .messages
            .iter()
            .filter(|message| matches!(message, Message::Trade(_)))
            .count();

        let memory_usage_bytes = total_messages * std::mem::size_of::<Message>();

        dict.set_item("file_source", self.file_path.clone())?;
        dict.set_item("total_messages", total_messages)?;
        dict.set_item("total_orders", total_orders)?;
        dict.set_item("total_trades", total_trades)?;
        dict.set_item("memory_usage_bytes", memory_usage_bytes)?;

        Ok(dict.into_any().unbind())
    }
}

#[pyclass]
pub struct StreamingBinaryLoader {
    file_path: Option<String>,
    file: Option<File>,
    symbol_master: Option<Arc<HashMap<u32, ContractInfo>>>,
}

impl StreamingBinaryLoader {
    fn get_next_message_raw(&mut self) -> PyResult<Option<Message>> {
        let Some(file) = self.file.as_mut() else {
            return Ok(None);
        };

        read_next_message_from_file(file)
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))
    }
}

#[pymethods]
impl StreamingBinaryLoader {
    #[new]
    pub fn new() -> Self {
        Self {
            file_path: None,
            file: None,
            symbol_master: None,
        }
    }

    #[pyo3(signature = (file_path, count_messages=true))]
    pub fn open_stream(&mut self, file_path: String, count_messages: bool) -> PyResult<usize> {
        let mut file = File::open(&file_path)
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;

        validate_binary_header(&mut file)
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;

        file.seek(SeekFrom::Start(0))
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;

        let count = if count_messages {
            count_messages_in_file(&file_path)
                .map_err(|err| PyRuntimeError::new_err(err.to_string()))?
        } else {
            0
        };

        self.file_path = Some(file_path);
        self.file = Some(file);

        Ok(count)
    }

    pub fn reset_cursor(&mut self) -> PyResult<()> {
        let Some(file) = self.file.as_mut() else {
            return Ok(());
        };

        file.seek(SeekFrom::Start(0))
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;
        Ok(())
    }

    pub fn get_next_msg(&mut self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let Some(message) = self.get_next_message_raw()? else {
            return Ok(None);
        };
        let py_obj = message_to_py_dict(py, &message)?;
        if let Some(ref master) = self.symbol_master {
            let bound = py_obj.bind(py);
            let dict = bound.downcast::<PyDict>()?;
            let token: u32 = dict
                .get_item("token")?
                .and_then(|v| {
                    v.extract::<u32>().ok()
                        .or_else(|| v.extract::<i32>().ok().map(|n| n as u32))
                })
                .unwrap_or(0);
            if let Some(info) = master.get(&token) {
                dict.set_item("token_symbol", &info.symbol)?;
                dict.set_item("strike_price", info.strike)?;
                dict.set_item("option_type",  &info.option_type)?;
                dict.set_item("expiry",       &info.expiry)?;
                dict.set_item("lot_size",     info.lot_size)?;
                dict.set_item("name",         &info.name)?;
            }
        }
        Ok(Some(py_obj))
    }

    pub fn is_end_of_msg(&mut self) -> PyResult<bool> {
        let Some(file) = self.file.as_mut() else {
            return Ok(true);
        };

        let current_pos = file
            .stream_position()
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;

        let result = read_next_message_from_file(file);

        file.seek(SeekFrom::Start(current_pos))
            .map_err(|err| PyRuntimeError::new_err(err.to_string()))?;

        match result {
            Ok(Some(_message)) => Ok(false),
            Ok(None) => Ok(true),
            Err(err) => Err(PyRuntimeError::new_err(err.to_string())),
        }
    }

    /// Attach a loaded SymbolMaster so that get_next_msg() auto-enriches every message.
    pub fn attach_symbol_master(&mut self, master: PyRef<'_, SymbolMaster>) {
        self.symbol_master = Some(Arc::new(master.contracts.clone()));
    }

    /// Remove the attached SymbolMaster (messages will have None symbol fields again).
    pub fn detach_symbol_master(&mut self) {
        self.symbol_master = None;
    }
}

#[pyclass]
pub struct OrderbookBuilder {
    manager: OrderBookManager,
    allowed_message_types: Option<Vec<u8>>,
}

impl OrderbookBuilder {
    fn should_process(&self, msg_type: u8) -> bool {
        match &self.allowed_message_types {
            Some(allowed) => allowed.contains(&msg_type),
            None => true,
        }
    }

    fn process_message(&mut self, message: &Message) -> bool {
        match message {
            Message::Order(order_packet) => {
                let msg_type = unsafe {
                    std::ptr::addr_of!(order_packet.ord.msg_type).read_unaligned()
                };

                if !self.should_process(msg_type) {
                    return false;
                }

                if matches!(msg_type, b'N' | b'M') {
                    let order_type = unsafe {
                        std::ptr::addr_of!(order_packet.ord.order_type).read_unaligned()
                    };

                    if !matches!(order_type, b'B' | b'S') {
                        return false;
                    }
                }

                self.manager.process_order_message(order_packet);
                true
            }

            Message::Trade(trade_packet) => {
                let msg_type = unsafe {
                    std::ptr::addr_of!(trade_packet.trd.msg_type).read_unaligned()
                };

                if !self.should_process(msg_type) {
                    return false;
                }

                self.manager.process_trade_message(trade_packet);
                true
            }
        }
    }

    fn message_from_dict(&self, msg: &Bound<'_, PyDict>) -> PyResult<Message> {
        let msg_type_obj = msg
            .get_item("msg_type")?
            .ok_or_else(|| PyTypeError::new_err("missing key: msg_type"))?;

        let msg_type = if let Ok(s) = msg_type_obj.extract::<String>() {
            s.as_bytes()
                .first()
                .copied()
                .ok_or_else(|| PyTypeError::new_err("msg_type cannot be empty"))?
        } else {
            msg_type_obj.extract::<u8>()?
        };

        if msg_type == b'T' {
            let exch_ts = msg
                .get_item("exch_ts")?
                .map(|v| v.extract::<u64>())
                .transpose()?
                .unwrap_or(0);

            let buy_order_id = msg
                .get_item("buy_order_id")?
                .ok_or_else(|| PyTypeError::new_err("missing key: buy_order_id"))?
                .extract::<u64>()?;

            let sell_order_id = msg
                .get_item("sell_order_id")?
                .ok_or_else(|| PyTypeError::new_err("missing key: sell_order_id"))?
                .extract::<u64>()?;

            let token = msg
                .get_item("token")?
                .ok_or_else(|| PyTypeError::new_err("missing key: token"))?
                .extract::<i32>()?;

            let trade_price = msg
                .get_item("trade_price")?
                .map(|v| v.extract::<i32>())
                .transpose()?
                .unwrap_or(0);

            let trade_quantity = msg
                .get_item("trade_quantity")?
                .ok_or_else(|| PyTypeError::new_err("missing key: trade_quantity"))?
                .extract::<i32>()?;

            let local_ts = msg
                .get_item("local_ts")?
                .map(|v| v.extract::<u64>())
                .transpose()?
                .unwrap_or(0);

            let flags = msg
                .get_item("flags")?
                .map(|v| v.extract::<bool>())
                .transpose()?
                .unwrap_or(false);

            Ok(Message::Trade(TradePacket {
                hdr: structure::StreamHeader {
                    msg_len: 0,
                    stream_id: 0,
                    seq_no: 0,
                },
                trd: structure::TradeMessage {
                    msg_type,
                    exch_ts,
                    buy_order_id,
                    sell_order_id,
                    token,
                    trade_price,
                    trade_quantity,
                },
                local_ts,
                flags,
            }))
        } else if matches!(msg_type, b'N' | b'M' | b'X') {
            let exch_ts = msg
                .get_item("exch_ts")?
                .map(|v| v.extract::<u64>())
                .transpose()?
                .unwrap_or(0);

            let order_id = msg
                .get_item("order_id")?
                .ok_or_else(|| PyTypeError::new_err("missing key: order_id"))?
                .extract::<u64>()?;

            let token = msg
                .get_item("token")?
                .ok_or_else(|| PyTypeError::new_err("missing key: token"))?
                .extract::<u32>()?;

            let order_type_obj = msg
                .get_item("order_type")?
                .ok_or_else(|| PyTypeError::new_err("missing key: order_type"))?;

            let order_type = if let Ok(s) = order_type_obj.extract::<String>() {
                s.as_bytes()
                    .first()
                    .copied()
                    .ok_or_else(|| PyTypeError::new_err("order_type cannot be empty"))?
            } else {
                order_type_obj.extract::<u8>()?
            };

            let price = msg
                .get_item("price")?
                .ok_or_else(|| PyTypeError::new_err("missing key: price"))?
                .extract::<u32>()?;

            let quantity = msg
                .get_item("quantity")?
                .ok_or_else(|| PyTypeError::new_err("missing key: quantity"))?
                .extract::<u32>()?;

            let local_ts = msg
                .get_item("local_ts")?
                .map(|v| v.extract::<u64>())
                .transpose()?
                .unwrap_or(0);

            let flags = msg
                .get_item("flags")?
                .map(|v| v.extract::<bool>())
                .transpose()?
                .unwrap_or(false);

            Ok(Message::Order(OrderPacket {
                hdr: structure::StreamHeader {
                    msg_len: 0,
                    stream_id: 0,
                    seq_no: 0,
                },
                ord: structure::OrderMessage {
                    msg_type,
                    exch_ts,
                    order_id,
                    token,
                    order_type,
                    price,
                    quantity,
                },
                local_ts,
                flags,
            }))
        } else {
            Err(PyTypeError::new_err(format!(
                "unsupported msg_type: {}",
                msg_type as char
            )))
        }
    }
}
#[pymethods]
impl OrderbookBuilder {
    #[new]
    pub fn new() -> Self {
        Self {
            manager: OrderBookManager::new(),
            allowed_message_types: None,
        }
    }
    #[pyo3(signature = (logic_criteria=None))]
    pub fn apply_filter(&mut self, logic_criteria: Option<Vec<String>>) {
        self.allowed_message_types = logic_criteria.map(|items| {
            items
                .into_iter()
                .filter_map(|item| item.as_bytes().first().copied())
                .collect()
        });
    }

    /// Push one already-decoded message into the orderbook.
    ///
    /// The caller should read one message first by calling:
    ///     msg = reader.get_next_msg()
    ///
    /// Then pass that message here:
    ///     builder.orderbook_add_msg(msg)
    ///
    /// Returns:
    /// - Ok(true)  => message was accepted and applied to the orderbook
    /// - Ok(false) => message was valid but skipped by apply_filter() or business rules
    pub fn orderbook_add_msg(&mut self, msg: &Bound<'_, PyAny>) -> PyResult<bool> {
        let msg_dict = msg.downcast::<PyDict>().map_err(|_| {
            PyTypeError::new_err("orderbook_add_msg expects one message dict from get_next_msg()")
        })?;

        let message = self.message_from_dict(msg_dict)?;
        Ok(self.process_message(&message))
    }

    pub fn build_from_list(&mut self, source: &Bound<'_, PyAny>) -> PyResult<usize> {
        if let Ok(reader) = source.extract::<PyRef<'_, MessageCacheReader>>() {
            let mut count = 0usize;
            for message in reader.messages.iter() {
                if self.process_message(message) {
                    count += 1;
                }
            }
            return Ok(count);
        }

        let list = source.downcast::<PyList>().map_err(|_| {
            PyTypeError::new_err(
                "build_from_list expects MessageCacheReader or list[dict] decoded messages",
            )
        })?;

        let mut count = 0usize;
        for item in list.iter() {
            let msg = item.downcast::<PyDict>().map_err(|_| {
                PyTypeError::new_err("each list entry must be a dict")
            })?;

            let msg_type_obj = msg
                .get_item("msg_type")?
                .ok_or_else(|| PyTypeError::new_err("missing key: msg_type"))?;

            let msg_type = if let Ok(s) = msg_type_obj.extract::<String>() {
                s.as_bytes()
                    .first()
                    .copied()
                    .ok_or_else(|| PyTypeError::new_err("msg_type cannot be empty"))?
            } else {
                msg_type_obj.extract::<u8>()?
            };

            if msg_type == b'T' {
                let exch_ts = msg
                    .get_item("exch_ts")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?
                    .unwrap_or(0);
                let buy_order_id = msg
                    .get_item("buy_order_id")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: buy_order_id"))?
                    .extract::<u64>()?;
                let sell_order_id = msg
                    .get_item("sell_order_id")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: sell_order_id"))?
                    .extract::<u64>()?;
                let token = msg
                    .get_item("token")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: token"))?
                    .extract::<i32>()?;
                let trade_price = msg
                    .get_item("trade_price")?
                    .map(|v| v.extract::<i32>())
                    .transpose()?
                    .unwrap_or(0);
                let trade_quantity = msg
                    .get_item("trade_quantity")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: trade_quantity"))?
                    .extract::<i32>()?;
                let local_ts = msg
                    .get_item("local_ts")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?
                    .unwrap_or(0);
                let flags = msg
                    .get_item("flags")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(false);

                let trade_packet = TradePacket {
                    hdr: structure::StreamHeader {
                        msg_len: 0,
                        stream_id: 0,
                        seq_no: 0,
                    },
                    trd: structure::TradeMessage {
                        msg_type,
                        exch_ts,
                        buy_order_id,
                        sell_order_id,
                        token,
                        trade_price,
                        trade_quantity,
                    },
                    local_ts,
                    flags,
                };

                if self.process_message(&Message::Trade(trade_packet)) {
                    count += 1;
                }
            } else if matches!(msg_type, b'N' | b'M' | b'X') {
                let exch_ts = msg
                    .get_item("exch_ts")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?
                    .unwrap_or(0);
                let order_id = msg
                    .get_item("order_id")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: order_id"))?
                    .extract::<u64>()?;
                let token = msg
                    .get_item("token")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: token"))?
                    .extract::<u32>()?;
                let order_type_obj = msg
                    .get_item("order_type")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: order_type"))?;
                let order_type = if let Ok(s) = order_type_obj.extract::<String>() {
                    s.as_bytes()
                        .first()
                        .copied()
                        .ok_or_else(|| PyTypeError::new_err("order_type cannot be empty"))?
                } else {
                    order_type_obj.extract::<u8>()?
                };
                let price = msg
                    .get_item("price")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: price"))?
                    .extract::<u32>()?;
                let quantity = msg
                    .get_item("quantity")?
                    .ok_or_else(|| PyTypeError::new_err("missing key: quantity"))?
                    .extract::<u32>()?;
                let local_ts = msg
                    .get_item("local_ts")?
                    .map(|v| v.extract::<u64>())
                    .transpose()?
                    .unwrap_or(0);
                let flags = msg
                    .get_item("flags")?
                    .map(|v| v.extract::<bool>())
                    .transpose()?
                    .unwrap_or(false);

                let order_packet = OrderPacket {
                    hdr: structure::StreamHeader {
                        msg_len: 0,
                        stream_id: 0,
                        seq_no: 0,
                    },
                    ord: structure::OrderMessage {
                        msg_type,
                        exch_ts,
                        order_id,
                        token,
                        order_type,
                        price,
                        quantity,
                    },
                    local_ts,
                    flags,
                };

                if self.process_message(&Message::Order(order_packet)) {
                    count += 1;
                }
            } else {
                return Err(PyTypeError::new_err(format!(
                    "unsupported msg_type: {}",
                    msg_type as char
                )));
            }
        }

        Ok(count)
    }
#[pyo3(signature = (source, limit=None))]
    pub fn build_from_source(
        &mut self,
        source: &Bound<'_, PyAny>,
        limit: Option<usize>,
    ) -> PyResult<usize> {
        if source.extract::<PyRef<'_, MessageCacheReader>>().is_ok() {
            return self.build_from_list(source);
        }

        if let Ok(mut stream_reader) = source.extract::<PyRefMut<'_, StreamingBinaryLoader>>() {
            let max_messages = limit.unwrap_or(usize::MAX);
            let mut count = 0usize;

            while count < max_messages {
                let Some(message) = stream_reader.get_next_message_raw()? else {
                    break;
                };

                if self.process_message(&message) {
                    count += 1;
                }
            }

            return Ok(count);
        }

        Err(PyTypeError::new_err(
            "build_from_source expects MessageCacheReader or StreamingBinaryLoader",
        ))
    }
    pub fn get_active_tokens(&self) -> Vec<u32> {
        self.manager.active_tokens()
    }

    pub fn get_full_depth(&self, py: Python<'_>, token: u32) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("token", token)?;

        match self.manager.get_full_depth(token) {
            Some((bids, asks)) => {
                let best_bid = bids.first().cloned();
                let best_ask = asks.first().cloned();
                let spread = match (best_bid, best_ask) {
                    (Some((bid_price, _)), Some((ask_price, _))) => {
                        Some(ask_price.saturating_sub(bid_price))
                    }
                    _ => None,
                };

                dict.set_item("found", true)?;
                dict.set_item("best_bid", best_bid)?;
                dict.set_item("best_ask", best_ask)?;
                dict.set_item("spread", spread)?;
                dict.set_item("bids", PyList::new_bound(py, bids))?;
                dict.set_item("asks", PyList::new_bound(py, asks))?;
            }
            None => {
                dict.set_item("found", false)?;
                dict.set_item("best_bid", Option::<(u32, u64)>::None)?;
                dict.set_item("best_ask", Option::<(u32, u64)>::None)?;
                dict.set_item("spread", Option::<u32>::None)?;
                dict.set_item("bids", PyList::empty_bound(py))?;
                dict.set_item("asks", PyList::empty_bound(py))?;
            }
        }

        Ok(dict.into_any().unbind())
    }

    #[pyo3(signature = (token, levels=None))]
    pub fn get_snapshot(
        &self,
        py: Python<'_>,
        token: u32,
        levels: Option<usize>,
    ) -> PyResult<Py<PyAny>> {
        let levels = levels.unwrap_or(5);
        let dict = PyDict::new_bound(py);

        dict.set_item("token", token)?;

        match self.manager.get_top_levels(token, levels) {
            Some((mid_price, bids, asks)) => {
                let best_bid = bids.first().cloned();
                let best_ask = asks.first().cloned();

                let spread = match (best_bid, best_ask) {
                    (Some((bid_price, _)), Some((ask_price, _))) => {
                        Some(ask_price.saturating_sub(bid_price))
                    }
                    _ => None,
                };

                let py_bids = PyList::new_bound(py, bids);
                let py_asks = PyList::new_bound(py, asks);

                dict.set_item("found", true)?;
                dict.set_item("mid_price", mid_price)?;
                dict.set_item("best_bid", best_bid)?;
                dict.set_item("best_ask", best_ask)?;
                dict.set_item("spread", spread)?;
                dict.set_item("bids", py_bids)?;
                dict.set_item("asks", py_asks)?;
            }

            None => {
                let empty_bids = PyList::empty_bound(py);
                let empty_asks = PyList::empty_bound(py);

                dict.set_item("found", false)?;
                dict.set_item("mid_price", 0)?;
                dict.set_item("best_bid", Option::<(u32, u64)>::None)?;
                dict.set_item("best_ask", Option::<(u32, u64)>::None)?;
                dict.set_item("spread", Option::<u32>::None)?;
                dict.set_item("bids", empty_bids)?;
                dict.set_item("asks", empty_asks)?;
            }
        }

        Ok(dict.into_any().unbind())
    }
    pub fn snapshot_header(&self) -> String {
        "local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,ask_qty_1,bid_price_2,bid_qty_2,ask_price_2,ask_qty_2,bid_price_3,bid_qty_3,ask_price_3,ask_qty_3,bid_price_4,bid_qty_4,ask_price_4,ask_qty_4".to_string()
    }

    #[pyo3(signature = (token, levels=None))]
    pub fn get_snapshot_row(&self, token: u32, levels: Option<usize>) -> PyResult<String> {
        let levels = levels.unwrap_or(5);
        let local_ts = 0u64;
        let exch_ts = 0u64;

        if let Some((mid_price, bids, asks)) = self.manager.get_top_levels(token, levels) {
            Ok(format_snapshot_row(local_ts, exch_ts, mid_price, bids, asks))
        } else {
            Ok(format_snapshot_row(local_ts, exch_ts, 0, Vec::new(), Vec::new()))
        }
    }


}

// ─── SymbolMaster ─────────────────────────────────────────────────────────────

/// Loads the NSE FO/CM contract master CSV in Rust and provides fast token → symbol lookups.
///
/// Usage::
///
///     from fastreader import SymbolMaster, StreamingBinaryLoader
///
///     sm = SymbolMaster()
///     count = sm.load_for_date("NSE_FO", day=21, month=5, year=2026)
///     print(sm)           # SymbolMaster(contracts=95632)
///
///     info = sm.lookup(token=40434)
///     # {'token': 40434, 'found': True, 'symbol': 'FINNIFTY', 'strike': 21700, ...}
///
///     # Auto-enrich streaming messages:
///     reader = StreamingBinaryLoader()
///     reader.open_stream(path, count_messages=False)
///     reader.attach_symbol_master(sm)
///     msg = reader.get_next_msg()     # token_symbol, strike_price, option_type now populated
#[pyclass]
pub struct SymbolMaster {
    contracts: HashMap<u32, ContractInfo>,
}

#[pymethods]
impl SymbolMaster {
    #[new]
    pub fn new() -> Self {
        Self { contracts: HashMap::new() }
    }

    /// Load the contract master from an explicit CSV file path.
    /// Returns the number of contracts loaded.
    pub fn load(&mut self, csv_path: String) -> PyResult<usize> {
        use std::io::{BufRead, BufReader};

        let file = File::open(&csv_path)
            .map_err(|e| PyRuntimeError::new_err(format!("cannot open {csv_path}: {e}")))?;
        let mut rdr = BufReader::new(file);

        let mut header_line = String::new();
        rdr.read_line(&mut header_line)
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        let hdrs: Vec<&str> = header_line
            .trim_end_matches(|c| c == '\r' || c == '\n')
            .split(',')
            .collect();

        macro_rules! col {
            ($name:expr) => {
                hdrs.iter()
                    .position(|h| *h == $name)
                    .ok_or_else(|| PyRuntimeError::new_err(
                        format!("column '{}' not found in {csv_path}", $name)
                    ))?
            };
        }

        let i_token  = col!("FinInstrmId");
        let i_symbol = col!("TckrSymb");
        let i_expiry = col!("XpryDt");
        let i_strike = col!("StrkPric");
        let i_optype = col!("OptnTp");
        let i_name   = col!("StockNm");
        let i_lot    = hdrs.iter().position(|h| *h == "NewBrdLotQty")
            .or_else(|| hdrs.iter().position(|h| *h == "MinLot"));

        let min_cols = [i_token, i_symbol, i_expiry, i_strike, i_optype, i_name]
            .into_iter()
            .max()
            .unwrap_or(0);

        self.contracts.clear();
        let mut count = 0usize;

        for line in rdr.lines() {
            let line = line.map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            let f: Vec<&str> = line.split(',').collect();
            if f.len() <= min_cols { continue; }

            let Ok(token) = f[i_token].trim().parse::<u32>() else { continue; };
            let expiry_ts: i64  = f[i_expiry].trim().parse().unwrap_or(0);
            let raw_strike: i64 = f[i_strike].trim().parse().unwrap_or(-100);
            let lot: u32 = i_lot
                .and_then(|i| f.get(i))
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);

            self.contracts.insert(token, ContractInfo {
                symbol:      f[i_symbol].trim().to_string(),
                name:        f[i_name].trim().to_string(),
                option_type: f[i_optype].trim().to_string(),
                strike:      raw_strike / 100,
                expiry:      unix_ts_to_date(expiry_ts),
                lot_size:    lot,
            });
            count += 1;
        }

        Ok(count)
    }

    /// Convenience: build the standard NSE contract master path from date components and load.
    ///
    /// Path pattern: ``{base}/CONTRACT/{DD}_{MM}_{YYYY}/NSE_{FO|CM}_contract_{DD}{MM}{YYYY}.csv``
    #[pyo3(signature = (segment, day, month, year, base_path=None))]
    pub fn load_for_date(
        &mut self,
        segment: &str,
        day: u32,
        month: u32,
        year: u32,
        base_path: Option<&str>,
    ) -> PyResult<usize> {
        let seg = match segment.to_uppercase().as_str() {
            "NSE_FO" | "FO" => "FO",
            "NSE_CM" | "CM" => "CM",
            other => return Err(PyRuntimeError::new_err(
                format!("unknown segment '{other}' — expected NSE_FO, FO, NSE_CM, or CM")
            )),
        };
        let base     = base_path.unwrap_or("/nas/50.30");
        let folder   = format!("{day:02}_{month:02}_{year}");
        let filename = format!("NSE_{seg}_contract_{day:02}{month:02}{year}.csv");
        let path     = format!("{base}/CONTRACT/{folder}/{filename}");
        self.load(path)
    }

    /// Look up a single token. Returns a dict with ``found=True/False`` and contract metadata.
    ///
    /// Keys when found: token, found, symbol, name, option_type, strike, expiry, lot_size
    pub fn lookup(&self, py: Python<'_>, token: u32) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new_bound(py);
        dict.set_item("token", token)?;
        match self.contracts.get(&token) {
            Some(info) => {
                dict.set_item("found",       true)?;
                dict.set_item("symbol",      &info.symbol)?;
                dict.set_item("name",        &info.name)?;
                dict.set_item("option_type", &info.option_type)?;
                dict.set_item("strike",      info.strike)?;
                dict.set_item("expiry",      &info.expiry)?;
                dict.set_item("lot_size",    info.lot_size)?;
            }
            None => {
                dict.set_item("found",       false)?;
                dict.set_item("symbol",      py.None())?;
                dict.set_item("name",        py.None())?;
                dict.set_item("option_type", py.None())?;
                dict.set_item("strike",      py.None())?;
                dict.set_item("expiry",      py.None())?;
                dict.set_item("lot_size",    py.None())?;
            }
        }
        Ok(dict.into_any().unbind())
    }

    /// Enrich a message dict from ``get_next_msg()`` in place.
    ///
    /// Sets ``token_symbol``, ``strike_price``, ``option_type``, ``expiry``,
    /// ``lot_size``, and ``name`` on the dict. No-op if the token is not in
    /// the loaded master. Returns True if the token was found.
    pub fn enrich(&self, msg: &Bound<'_, PyAny>) -> PyResult<bool> {
        let dict = msg.downcast::<PyDict>().map_err(|_| {
            PyTypeError::new_err("enrich() expects a message dict from get_next_msg()")
        })?;
        let token: u32 = match dict.get_item("token")? {
            Some(v) => v
                .extract::<u32>()
                .or_else(|_| v.extract::<i32>().map(|n| n as u32))
                .unwrap_or(0),
            None => return Err(PyTypeError::new_err("msg dict missing 'token' key")),
        };
        if let Some(info) = self.contracts.get(&token) {
            dict.set_item("token_symbol", &info.symbol)?;
            dict.set_item("strike_price", info.strike)?;
            dict.set_item("option_type",  &info.option_type)?;
            dict.set_item("expiry",       &info.expiry)?;
            dict.set_item("lot_size",     info.lot_size)?;
            dict.set_item("name",         &info.name)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Number of contracts currently loaded.
    pub fn __len__(&self) -> usize {
        self.contracts.len()
    }

    pub fn __repr__(&self) -> String {
        format!("SymbolMaster(contracts={})", self.contracts.len())
    }
}

/// Python-accessible feed file path builder.
///
/// Usage::
///
///     builder = FeedPathBuilder()
///     path = builder.build("NSE_CM", stream_id=2, day=29, month=12, year=2025)
///     # → "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"
///
///     # With file-existence check:
///     path = builder.build_and_verify("NSE_CM", stream_id=2, day=29, month=12, year=2025)
///
///     # Custom base path:
///     path = builder.build("NSE_FO", stream_id=1, day=1, month=6, year=2026,
///                          base_path="/mnt/data")
#[pyclass]
pub struct FeedPathBuilder;

#[pymethods]
impl FeedPathBuilder {
    #[new]
    pub fn new() -> Self {
        Self
    }

    /// Build the file path from components.
    ///
    /// Args:
    ///     segment   (str): "NSE_CM", "CM", "NSE_FO", or "FO" (case-insensitive)
    ///     stream_id (int): stream identifier, must be > 0
    ///     day       (int): day of month (1-31)
    ///     month     (int): month (1-12)
    ///     year      (int): four-digit year (2000-2100)
    ///     base_path (str, optional): root directory; defaults to "/nas/50.30"
    ///
    /// Returns:
    ///     str: full file path
    #[pyo3(signature = (segment, stream_id, day, month, year, base_path=None))]
    pub fn build(
        &self,
        segment: &str,
        stream_id: u32,
        day: u32,
        month: u32,
        year: u32,
        base_path: Option<&str>,
    ) -> PyResult<String> {
        // ── BP-LIB-1: args entering Python → Rust FFI boundary ───────────
        #[cfg(debug_assertions)]
        eprintln!(
            "[FeedPathBuilder::build] segment={segment:?} stream_id={stream_id} \
             day={day:02} month={month:02} year={year:04} base_path={base_path:?}"
        );

        let result = FeedFilePath::build(segment, stream_id, day, month, year, base_path);

        // ── BP-LIB-2: result before it crosses back to Python ─────────────
        #[cfg(debug_assertions)]
        eprintln!("[FeedPathBuilder::build] result={result:?}");

        result.map_err(PyRuntimeError::new_err)
    }

    /// Same as `build`, but also verifies the file exists on disk.
    /// Raises RuntimeError if the path does not exist.
    #[pyo3(signature = (segment, stream_id, day, month, year, base_path=None))]
    pub fn build_and_verify(
        &self,
        segment: &str,
        stream_id: u32,
        day: u32,
        month: u32,
        year: u32,
        base_path: Option<&str>,
    ) -> PyResult<String> {
        // ── BP-LIB-3: args entering build_and_verify ─────────────────────
        #[cfg(debug_assertions)]
        eprintln!(
            "[FeedPathBuilder::build_and_verify] segment={segment:?} stream_id={stream_id} \
             day={day:02} month={month:02} year={year:04} base_path={base_path:?}"
        );

        let result = FeedFilePath::build_and_verify(segment, stream_id, day, month, year, base_path);

        // ── BP-LIB-4: result after disk check ────────────────────────────
        #[cfg(debug_assertions)]
        eprintln!("[FeedPathBuilder::build_and_verify] result={result:?}");

        result.map_err(PyRuntimeError::new_err)
    }

    pub fn __repr__(&self) -> &str {
        "FeedPathBuilder()"
    }
}

#[pymodule]
fn fastreader(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<CachedMessage>()?;
    m.add_class::<MessageCacheReader>()?;
    m.add_class::<StreamingBinaryLoader>()?;
    m.add_class::<OrderbookBuilder>()?;
    m.add_class::<FeedPathBuilder>()?;
    m.add_class::<SymbolMaster>()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure::{
        Message, OrderMessage, OrderPacket, StreamHeader, TradeMessage, TradePacket,
    };
    use std::mem::size_of;
    use std::path::PathBuf;

    // ── helpers ─────────────────────────────────────────────────────────────

    fn make_order_packet(
        msg_type: u8,
        order_type: u8,
        order_id: u64,
        token: u32,
        price: u32,
        qty: u32,
        flags: bool,
    ) -> OrderPacket {
        OrderPacket {
            hdr: StreamHeader { msg_len: 10, stream_id: 2, seq_no: 42 },
            ord: OrderMessage {
                msg_type,
                exch_ts: 100_000,
                order_id,
                token,
                order_type,
                price,
                quantity: qty,
            },
            local_ts: 200_000,
            flags,
        }
    }

    fn make_trade_packet(
        buy_id: u64,
        sell_id: u64,
        token: i32,
        price: i32,
        qty: i32,
        flags: bool,
    ) -> TradePacket {
        TradePacket {
            hdr: StreamHeader { msg_len: 10, stream_id: 2, seq_no: 99 },
            trd: TradeMessage {
                msg_type: b'T',
                exch_ts: 300_000,
                buy_order_id: buy_id,
                sell_order_id: sell_id,
                token,
                trade_price: price,
                trade_quantity: qty,
            },
            local_ts: 400_000,
            flags,
        }
    }

    fn write_tmp(label: &str, data: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("orderpulse_is_end_of_msg_{label}.bin"));
        std::fs::write(&path, data).unwrap();
        path
    }

    unsafe fn as_bytes<T: Sized>(val: &T) -> &[u8] {
        std::slice::from_raw_parts(val as *const T as *const u8, size_of::<T>())
    }

    // ── format_message ───────────────────────────────────────────────────────

    #[test]
    fn test_format_order_message_fields() {
        let pkt = make_order_packet(b'N', b'B', 55, 1001, 500, 100, false);
        let s = format_message(&Message::Order(pkt));
        assert!(s.starts_with("Order Message:"), "got: {s}");
        assert!(s.contains("SeqNo 42"));
        assert!(s.contains("MsgLen 10"));
        assert!(s.contains("OrderId 55"));
        assert!(s.contains("Token 1001"));
        assert!(s.contains("Price 500"));
        assert!(s.contains("Quantity 100"));
        assert!(s.contains("Missed 0"));
    }

    #[test]
    fn test_format_order_message_flags_true() {
        let pkt = make_order_packet(b'X', b'S', 1, 2, 3, 4, true);
        let s = format_message(&Message::Order(pkt));
        assert!(s.contains("Missed 1"));
    }

    #[test]
    fn test_format_trade_message_fields() {
        let pkt = make_trade_packet(10, 20, 5000, 750, 30, true);
        let s = format_message(&Message::Trade(pkt));
        assert!(s.starts_with("Trade Message:"), "got: {s}");
        assert!(s.contains("SeqNo 99"));
        assert!(s.contains("BuyOrderId 10"));
        assert!(s.contains("SellOrderId 20"));
        assert!(s.contains("Token 5000"));
        assert!(s.contains("Price 750"));
        assert!(s.contains("Quantity 30"));
        assert!(s.contains("Missed 1"));
    }

    #[test]
    fn test_format_trade_message_flags_false() {
        let pkt = make_trade_packet(1, 2, 100, 200, 10, false);
        let s = format_message(&Message::Trade(pkt));
        assert!(s.contains("Missed 0"));
    }

    // ── MessageCacheReader ───────────────────────────────────────────────────

    #[test]
    fn test_message_cache_reader_new_empty() {
        let reader = MessageCacheReader::new();
        assert!(reader.file_path.is_none());
        assert_eq!(reader.messages.len(), 0);
    }

    #[test]
    fn test_get_all_messages_empty_cache() {
        let reader = MessageCacheReader::new();
        let msgs = reader.get_all_messages();
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_load_to_cache_nonexistent_file_is_err() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut reader = MessageCacheReader::new();
            assert!(reader.load_to_cache("/no/such/file.bin".to_string()).is_err());
        });
    }

    #[test]
    fn test_get_cache_summary_empty_returns_zeros() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let reader = MessageCacheReader::new();
            let obj = reader.get_cache_summary(py).expect("summary should not fail");
            let dict = obj.bind(py);
            let total: usize = dict.get_item("total_messages").unwrap().extract().unwrap();
            let orders: usize = dict.get_item("total_orders").unwrap().extract().unwrap();
            let trades: usize = dict.get_item("total_trades").unwrap().extract().unwrap();
            let bytes: usize = dict.get_item("memory_usage_bytes").unwrap().extract().unwrap();
            assert_eq!(total, 0);
            assert_eq!(orders, 0);
            assert_eq!(trades, 0);
            assert_eq!(bytes, 0);
        });
    }

    // ── StreamingBinaryLoader ────────────────────────────────────────────────

    #[test]
    fn test_streaming_binary_loader_new_defaults() {
        let loader = StreamingBinaryLoader::new();
        assert!(loader.file_path.is_none());
        assert!(loader.file.is_none());
    }

    #[test]
    fn test_open_stream_nonexistent_file_is_err() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut loader = StreamingBinaryLoader::new();
            assert!(loader
                .open_stream("/no/such/file.bin".to_string(), true)
                .is_err());
        });
    }

    #[test]
    fn test_is_end_of_msg_returns_true_when_unopened() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut loader = StreamingBinaryLoader::new();
            assert!(loader.is_end_of_msg().unwrap());
        });
    }

    #[test]
    fn test_is_end_of_msg_peek_does_not_advance_cursor() {
        let pkt = make_order_packet(b'N', b'B', 10, 101, 500, 1, false);
        let path = write_tmp("peek_cursor", unsafe { as_bytes(&pkt) });

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut loader = StreamingBinaryLoader::new();
            loader
                .open_stream(path.to_str().unwrap().to_string(), false)
                .unwrap();

            assert!(!loader.is_end_of_msg().unwrap());

            match loader.get_next_message_raw().unwrap().expect("expected one message") {
                Message::Order(packet) => {
                    let order_id = unsafe {
                        std::ptr::addr_of!(packet.ord.order_id).read_unaligned()
                    };
                    assert_eq!(order_id, 10);
                }
                Message::Trade(_) => panic!("expected order message"),
            }
        });

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_is_end_of_msg_transitions_to_true_at_eof() {
        let p1 = make_order_packet(b'N', b'B', 1, 101, 500, 1, false);
        let p2 = make_order_packet(b'N', b'B', 2, 101, 501, 1, false);
        let mut buf = unsafe { as_bytes(&p1) }.to_vec();
        buf.extend_from_slice(unsafe { as_bytes(&p2) });
        let path = write_tmp("eof_transition", &buf);

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut loader = StreamingBinaryLoader::new();
            loader
                .open_stream(path.to_str().unwrap().to_string(), false)
                .unwrap();

            assert!(!loader.is_end_of_msg().unwrap());
            assert!(loader.get_next_message_raw().unwrap().is_some());
            assert!(!loader.is_end_of_msg().unwrap());
            assert!(loader.get_next_message_raw().unwrap().is_some());
            assert!(loader.is_end_of_msg().unwrap());
        });

        let _ = std::fs::remove_file(&path);
    }

    // ── OrderbookBuilder ─────────────────────────────────────────────────────

    #[test]
    fn test_orderbook_builder_new_no_filter() {
        let builder = OrderbookBuilder::new();
        assert!(builder.allowed_message_types.is_none());
    }

    #[test]
    fn test_apply_filter_sets_correct_bytes() {
        let mut builder = OrderbookBuilder::new();
        builder.apply_filter(Some(vec!["N".to_string(), "M".to_string(), "X".to_string()]));
        let allowed = builder.allowed_message_types.as_ref().unwrap();
        assert!(allowed.contains(&b'N'));
        assert!(allowed.contains(&b'M'));
        assert!(allowed.contains(&b'X'));
        assert!(!allowed.contains(&b'T'));
    }

    #[test]
    fn test_apply_filter_none_clears_filter() {
        let mut builder = OrderbookBuilder::new();
        builder.apply_filter(Some(vec!["N".to_string()]));
        builder.apply_filter(None);
        assert!(builder.allowed_message_types.is_none());
    }

    #[test]
    fn test_apply_filter_empty_string_produces_empty_vec() {
        let mut builder = OrderbookBuilder::new();
        builder.apply_filter(Some(vec!["".to_string()]));
        let allowed = builder.allowed_message_types.as_ref().unwrap();
        assert!(allowed.is_empty());
    }

    #[test]
    fn test_build_from_list_empty_reader_returns_zero() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let reader = pyo3::Py::new(py, MessageCacheReader::new()).unwrap();
            let mut builder = OrderbookBuilder::new();
            let count = builder.build_from_list(reader.bind(py).as_any()).unwrap();
            assert_eq!(count, 0);
        });
    }

    #[test]
    fn test_get_snapshot_unknown_token_not_found() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let builder = OrderbookBuilder::new();
            let obj = builder.get_snapshot(py, 99999, Some(5)).unwrap();
            let dict = obj.bind(py);
            let found: bool = dict.get_item("found").unwrap().extract().unwrap();
            assert!(!found);
        });
    }

    #[test]
    fn test_get_snapshot_none_levels_uses_default() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let builder = OrderbookBuilder::new();
            assert!(builder.get_snapshot(py, 12345, None).is_ok());
        });
    }

    #[test]
    fn test_get_snapshot_returns_token_and_found_keys() {
        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let builder = OrderbookBuilder::new();
            let snapshot = builder.get_snapshot(py, 99999, Some(5)).unwrap();
            let snapshot_dict = snapshot.bind(py);
            let _token: u32 = snapshot_dict.get_item("token").unwrap().extract().unwrap();
            let _found: bool = snapshot_dict.get_item("found").unwrap().extract().unwrap();
        });
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Data-extraction debug tests
// Each test writes a known binary payload → parses it → asserts every field.
// ═══════════════════════════════════════════════════════════════════════════
#[cfg(test)]
mod debug_tests {
    use super::*;
    use crate::structure::{
        Message, OrderMessage, OrderPacket, StreamHeader, TradeMessage, TradePacket,
    };
    use std::mem::size_of;
    use std::path::PathBuf;

    // ── helpers ──────────────────────────────────────────────────────────────

    /// Write bytes to a uniquely-named temp file; caller must remove it.
    fn write_tmp(label: &str, data: &[u8]) -> PathBuf {
        let path = std::env::temp_dir()
            .join(format!("orderpulse_debug_{}.bin", label));
        std::fs::write(&path, data).unwrap();
        path
    }

    /// Safe byte view of a packed struct.
    unsafe fn as_bytes<T: Sized>(val: &T) -> &[u8] {
        std::slice::from_raw_parts(val as *const T as *const u8, size_of::<T>())
    }

    fn make_order(seq: u32, oid: u64, token: u32, price: u32, qty: u32, side: u8) -> OrderPacket {
        OrderPacket {
            hdr: StreamHeader {
                msg_len: size_of::<OrderPacket>() as u16,
                stream_id: 1,
                seq_no: seq,
            },
            ord: OrderMessage {
                msg_type: b'N',
                exch_ts: 100_000 + seq as u64,
                order_id: oid,
                token,
                order_type: side,
                price,
                quantity: qty,
            },
            local_ts: 200_000 + seq as u64,
            flags: false,
        }
    }

    fn make_trade(seq: u32, buy_id: u64, sell_id: u64, token: i32, price: i32, qty: i32) -> TradePacket {
        TradePacket {
            hdr: StreamHeader {
                msg_len: size_of::<TradePacket>() as u16,
                stream_id: 1,
                seq_no: seq,
            },
            trd: TradeMessage {
                msg_type: b'T',
                exch_ts: 300_000 + seq as u64,
                buy_order_id: buy_id,
                sell_order_id: sell_id,
                token,
                trade_price: price,
                trade_quantity: qty,
            },
            local_ts: 400_000 + seq as u64,
            flags: false,
        }
    }

    // Unaligned-safe field readers for packed structs (1 or 2 levels deep)
    macro_rules! rd {
        ($pkt:expr, $f1:ident . $f2:ident) => {
            unsafe { std::ptr::addr_of!($pkt.$f1.$f2).read_unaligned() }
        };
        ($pkt:expr, $f1:ident) => {
            unsafe { std::ptr::addr_of!($pkt.$f1).read_unaligned() }
        };
    }

    // ── 1. OrderPacket round-trip: every field ────────────────────────────────

    #[test]
    fn debug_order_packet_all_fields_round_trip() {
        let orig = make_order(7, 99, 2001, 1_000, 50, b'B');
        let path = write_tmp("order_rt", unsafe { as_bytes(&orig) });

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut reader = MessageCacheReader::new();
            let count = reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            assert_eq!(count, 1);

            match reader.messages[0] {
                Message::Order(p) => {
                    assert_eq!(rd!(p, hdr.seq_no),       7,         "seq_no");
                    assert_eq!(rd!(p, hdr.stream_id),    1,         "stream_id");
                    assert_eq!(rd!(p, ord.msg_type),     b'N',      "msg_type");
                    assert_eq!(rd!(p, ord.order_id),     99,        "order_id");
                    assert_eq!(rd!(p, ord.token),        2001,      "token");
                    assert_eq!(rd!(p, ord.order_type),   b'B',      "order_type");
                    assert_eq!(rd!(p, ord.price),        1_000,     "price");
                    assert_eq!(rd!(p, ord.quantity),     50,        "quantity");
                    assert_eq!(rd!(p, ord.exch_ts),      100_007,   "exch_ts");
                    assert_eq!(rd!(p, local_ts),         200_007,   "local_ts");
                    assert!(!rd!(p, flags),                         "flags");
                }
                Message::Trade(_) => panic!("expected Order"),
            }
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 2. TradePacket round-trip: every field ────────────────────────────────

    #[test]
    fn debug_trade_packet_all_fields_round_trip() {
        let orig = make_trade(3, 11, 22, 5_555, 2_500, 10);
        let path = write_tmp("trade_rt", unsafe { as_bytes(&orig) });

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut reader = MessageCacheReader::new();
            let count = reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            assert_eq!(count, 1);

            match reader.messages[0] {
                Message::Trade(p) => {
                    assert_eq!(rd!(p, hdr.seq_no),           3,         "seq_no");
                    assert_eq!(rd!(p, trd.msg_type),         b'T',      "msg_type");
                    assert_eq!(rd!(p, trd.buy_order_id),     11,        "buy_order_id");
                    assert_eq!(rd!(p, trd.sell_order_id),    22,        "sell_order_id");
                    assert_eq!(rd!(p, trd.token),            5_555,     "token");
                    assert_eq!(rd!(p, trd.trade_price),      2_500,     "trade_price");
                    assert_eq!(rd!(p, trd.trade_quantity),   10,        "trade_quantity");
                    assert_eq!(rd!(p, trd.exch_ts),          300_003,   "exch_ts");
                    assert_eq!(rd!(p, local_ts),             400_003,   "local_ts");
                }
                Message::Order(_) => panic!("expected Trade"),
            }
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 3. Mixed file: message count + type distribution ─────────────────────

    #[test]
    fn debug_mixed_file_counts_correct() {
        let o1 = make_order(1, 1, 100, 500, 10, b'B');
        let o2 = make_order(2, 2, 100, 505, 20, b'S');
        let t1 = make_trade(3, 1, 2, 100, 500, 5);

        let mut buf = unsafe { as_bytes(&o1) }.to_vec();
        buf.extend_from_slice(unsafe { as_bytes(&o2) });
        buf.extend_from_slice(unsafe { as_bytes(&t1) });
        let path = write_tmp("mixed_counts", &buf);

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let mut reader = MessageCacheReader::new();
            assert_eq!(reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap(), 3);

            let obj = reader.get_cache_summary(py).unwrap();
            let d = obj.bind(py);
            let total:  usize = d.get_item("total_messages").unwrap().extract().unwrap();
            let orders: usize = d.get_item("total_orders").unwrap().extract().unwrap();
            let trades: usize = d.get_item("total_trades").unwrap().extract().unwrap();
            let bytes:  usize = d.get_item("memory_usage_bytes").unwrap().extract().unwrap();
            assert_eq!(total,  3);
            assert_eq!(orders, 2);
            assert_eq!(trades, 1);
            assert_eq!(bytes,  3 * size_of::<Message>());
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 4. get_all_messages: structured values match original packet ───────────

    #[test]
    fn debug_get_all_messages_order_struct_values() {
        let orig = make_order(5, 77, 3001, 900, 25, b'B');
        let path = write_tmp("get_all_msg", unsafe { as_bytes(&orig) });

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut reader = MessageCacheReader::new();
            reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            let msgs = reader.get_all_messages();
            assert_eq!(msgs.len(), 1);
            let msg = &msgs[0];
            assert_eq!(msg.message_kind, "order");
            assert_eq!(msg.seq_no, 5);
            assert_eq!(msg.order_id, Some(77));
            assert_eq!(msg.token, 3001);
            assert_eq!(msg.price, Some(900));
            assert_eq!(msg.quantity, Some(25));
            assert!(!msg.flags);
        });
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn debug_get_all_messages_trade_struct_values() {
        let orig = make_trade(9, 55, 66, 7777, 1_200, 8);
        let path = write_tmp("get_all_trd", unsafe { as_bytes(&orig) });

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut reader = MessageCacheReader::new();
            reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            let msgs = reader.get_all_messages();
            let msg = &msgs[0];
            assert_eq!(msg.message_kind, "trade");
            assert_eq!(msg.seq_no, 9);
            assert_eq!(msg.buy_order_id, Some(55));
            assert_eq!(msg.sell_order_id, Some(66));
            assert_eq!(msg.token, 7777);
            assert_eq!(msg.trade_price, Some(1_200));
            assert_eq!(msg.trade_quantity, Some(8));
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 5. open_stream count + sequential stream reads ───────────────────────

    #[test]
    fn debug_open_stream_sequential_reads() {
        let o1 = make_order(1, 10, 200, 100, 5, b'B');
        let o2 = make_order(2, 20, 200, 101, 8, b'B');
        let mut buf = unsafe { as_bytes(&o1) }.to_vec();
        buf.extend_from_slice(unsafe { as_bytes(&o2) });
        let path = write_tmp("open_stream", &buf);

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|_py| {
            let mut loader = StreamingBinaryLoader::new();
            let count = loader
                .open_stream(path.to_str().unwrap().to_string(), true)
                .unwrap();
            assert_eq!(count, 2, "open_stream should report 2 messages");

            // message 1 → order_id 10
            match loader.get_next_message_raw().unwrap().expect("msg 1") {
                Message::Order(p) => assert_eq!(rd!(p, ord.order_id), 10),
                _ => panic!("expected Order"),
            }
            // message 2 → order_id 20
            match loader.get_next_message_raw().unwrap().expect("msg 2") {
                Message::Order(p) => assert_eq!(rd!(p, ord.order_id), 20),
                _ => panic!("expected Order"),
            }
            // EOF
            assert!(loader.get_next_message_raw().unwrap().is_none(), "expected EOF");
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 6. build_from_list → bid level in get_snapshot ───────────────────────

    #[test]
    fn debug_build_from_list_bid_level_correct() {
        let buy = make_order(1, 1, 777, 1_000, 40, b'B');
        let path = write_tmp("bid_lvl", unsafe { as_bytes(&buy) });

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let mut reader = MessageCacheReader::new();
            reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            let py_reader = pyo3::Py::new(py, reader).unwrap();

            let mut builder = OrderbookBuilder::new();
            let processed = builder.build_from_list(py_reader.bind(py).as_any()).unwrap();
            assert_eq!(processed, 1);

            let obj = builder.get_snapshot(py, 777, Some(5)).unwrap();
            let d = obj.bind(py);
            assert!(d.get_item("found").unwrap().extract::<bool>().unwrap());
            let (bid_p, bid_q): (u32, u64) = d.get_item("best_bid").unwrap()
                .extract::<Option<(u32, u64)>>().unwrap().expect("best_bid Some");
            assert_eq!(bid_p, 1_000, "bid price");
            assert_eq!(bid_q, 40,    "bid qty");
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 7. build_from_list → ask level in get_snapshot ───────────────────────

    #[test]
    fn debug_build_from_list_ask_level_correct() {
        let sell = make_order(1, 1, 888, 2_000, 15, b'S');
        let path = write_tmp("ask_lvl", unsafe { as_bytes(&sell) });

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let mut reader = MessageCacheReader::new();
            reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            let py_reader = pyo3::Py::new(py, reader).unwrap();

            let mut builder = OrderbookBuilder::new();
            builder.build_from_list(py_reader.bind(py).as_any()).unwrap();

            let obj = builder.get_snapshot(py, 888, Some(5)).unwrap();
            let d = obj.bind(py);
            let (ask_p, ask_q): (u32, u64) = d.get_item("best_ask").unwrap()
                .extract::<Option<(u32, u64)>>().unwrap().expect("best_ask Some");
            assert_eq!(ask_p, 2_000, "ask price");
            assert_eq!(ask_q, 15,    "ask qty");
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 8. mid_price = (best_bid + best_ask) / 2 ─────────────────────────────

    #[test]
    fn debug_get_snapshot_mid_price_correct() {
        // bid=1000, ask=1100 → mid=1050
        let buy  = make_order(1, 1, 999, 1_000, 10, b'B');
        let sell = make_order(2, 2, 999, 1_100, 10, b'S');
        let mut buf = unsafe { as_bytes(&buy) }.to_vec();
        buf.extend_from_slice(unsafe { as_bytes(&sell) });
        let path = write_tmp("mid_price", &buf);

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let mut reader = MessageCacheReader::new();
            reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            let py_reader = pyo3::Py::new(py, reader).unwrap();

            let mut builder = OrderbookBuilder::new();
            builder.build_from_list(py_reader.bind(py).as_any()).unwrap();

            let obj = builder.get_snapshot(py, 999, Some(5)).unwrap();
            let d = obj.bind(py);
            let mid: u32 = d.get_item("mid_price").unwrap().extract().unwrap();
            assert_eq!(mid, 1_050, "mid_price should be (1000+1100)/2=1050");

            let spread: Option<u32> = d.get_item("spread").unwrap().extract().unwrap();
            assert_eq!(spread, Some(100), "spread should be ask-bid=100");
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 9. apply_filter blocks trades, passes orders ──────────────────────────

    #[test]
    fn debug_apply_filter_blocks_trades() {
        let o1 = make_order(1, 1, 111, 500, 10, b'B');
        let t1 = make_trade(2, 1, 2, 111, 500, 5);
        let mut buf = unsafe { as_bytes(&o1) }.to_vec();
        buf.extend_from_slice(unsafe { as_bytes(&t1) });
        let path = write_tmp("filter_blk", &buf);

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let mut reader = MessageCacheReader::new();
            reader.load_to_cache(path.to_str().unwrap().to_string()).unwrap();
            assert_eq!(reader.messages.len(), 2, "file has 2 messages");

            let py_reader = pyo3::Py::new(py, reader).unwrap();
            let mut builder = OrderbookBuilder::new();
            builder.apply_filter(Some(vec!["N".to_string()])); // only 'N' orders
            let processed = builder.build_from_list(py_reader.bind(py).as_any()).unwrap();
            assert_eq!(processed, 1, "only the 'N' order should pass the filter");
        });
        let _ = std::fs::remove_file(&path);
    }

    // ── 10. get_cache_summary file_source field ───────────────────────────────

    #[test]
    fn debug_get_cache_summary_file_source_path() {
        let o = make_order(1, 1, 1, 100, 1, b'B');
        let path = write_tmp("file_src", unsafe { as_bytes(&o) });
        let path_str = path.to_str().unwrap().to_string();

        pyo3::prepare_freethreaded_python();
        pyo3::Python::with_gil(|py| {
            let mut reader = MessageCacheReader::new();
            reader.load_to_cache(path_str.clone()).unwrap();

            let obj = reader.get_cache_summary(py).unwrap();
            let d = obj.bind(py);
            let src: Option<String> = d.get_item("file_source").unwrap().extract().unwrap();
            assert_eq!(src.as_deref(), Some(path_str.as_str()), "file_source should match path");
        });
        let _ = std::fs::remove_file(&path);
    }
}
