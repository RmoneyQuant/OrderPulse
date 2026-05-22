use std::fs::File;
use std::io::Result;
use std::mem::size_of;

use memmap2::Mmap;

use crate::structure::{Message, OrderPacket, PeekStructure, TradePacket};

#[inline]
pub fn read_trd_ord_only(path: &str) -> Result<Vec<Message>> {
    let file = File::open(path)?;
    let mmap = unsafe { Mmap::map(&file)? };
    let buf = &mmap[..];

    // Heuristic capacity to reduce reallocs.
    let estimated_msg_count = buf.len() / size_of::<OrderPacket>();
    let mut messages = Vec::with_capacity(estimated_msg_count);

    let mut i: usize = 0;
    while i < buf.len() {
        // skip spaces
        while i < buf.len() && buf[i] == b' ' {
            i += 1;
        }

        if i + size_of::<PeekStructure>() > buf.len() {
            break;
        }

        let peek_buf = &buf[i..i + size_of::<PeekStructure>()];
        let peek_struct: PeekStructure =
            unsafe { std::ptr::read_unaligned(peek_buf.as_ptr() as *const _) };

        match peek_struct.msg_type {
            b'T' => {
                if i + size_of::<TradePacket>() > buf.len() {
                    break;
                }

                let trade_buf = &buf[i..i + size_of::<TradePacket>()];
                let mut trade_packet: TradePacket =
                    unsafe { std::ptr::read_unaligned(trade_buf.as_ptr() as *const _) };

                // endian fixups
                trade_packet.hdr.msg_len = u16::from_le(trade_packet.hdr.msg_len);
                trade_packet.hdr.stream_id = u16::from_le(trade_packet.hdr.stream_id);
                trade_packet.hdr.seq_no = u32::from_le(trade_packet.hdr.seq_no);

                trade_packet.trd.exch_ts = u64::from_le(trade_packet.trd.exch_ts);
                trade_packet.trd.buy_order_id = u64::from_le(trade_packet.trd.buy_order_id);
                trade_packet.trd.sell_order_id = u64::from_le(trade_packet.trd.sell_order_id);
                trade_packet.trd.token = i32::from_le(trade_packet.trd.token);
                trade_packet.trd.trade_price = i32::from_le(trade_packet.trd.trade_price);
                trade_packet.trd.trade_quantity = i32::from_le(trade_packet.trd.trade_quantity);

                messages.push(Message::Trade(trade_packet));
                i += size_of::<TradePacket>();
            }
            b'N' | b'M' | b'X' => {
                if i + size_of::<OrderPacket>() > buf.len() {
                    break;
                }

                let order_buf = &buf[i..i + size_of::<OrderPacket>()];
                let mut order_packet: OrderPacket =
                    unsafe { std::ptr::read_unaligned(order_buf.as_ptr() as *const _) };

                // endian fixups
                order_packet.hdr.msg_len = u16::from_le(order_packet.hdr.msg_len);
                order_packet.hdr.stream_id = u16::from_le(order_packet.hdr.stream_id);
                order_packet.hdr.seq_no = u32::from_le(order_packet.hdr.seq_no);

                order_packet.ord.exch_ts = u64::from_le(order_packet.ord.exch_ts);
                order_packet.ord.order_id = u64::from_le(order_packet.ord.order_id);
                order_packet.ord.token = u32::from_le(order_packet.ord.token);
                order_packet.ord.price = u32::from_le(order_packet.ord.price);
                order_packet.ord.quantity = u32::from_le(order_packet.ord.quantity);

                messages.push(Message::Order(order_packet));
                i += size_of::<OrderPacket>();
            }
            _ => {
                // Unknown message type — use msg_len to skip the whole message
                let skip = u16::from_le(peek_struct.global_header.msg_len) as usize;
                i += if skip > 0 { skip } else { 1 };
            }
        }
    }

    Ok(messages)
}