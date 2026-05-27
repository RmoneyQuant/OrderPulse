# OrderPulse / `fastreader`

A high-performance Python library for reading NSE binary order/trade feed files and building orderbook snapshots.

`fastreader` is written in Rust and exposed to Python through PyO3. Heavy binary parsing and orderbook processing runs in Rust, while Python users get a clean and simple API.

---

## What this library does

- Reads NSE binary feed files.
- Extracts order and trade messages.
- Supports both RAM-based and streaming-based reading.
- Builds token-wise orderbook state.
- Provides best bid, best ask, spread, mid price, top levels, and full depth.
- Gives Python-friendly dictionaries, strings, and CSV-style snapshot rows.

---

## Architecture

```text
Binary Feed File
      |
      v
Rust Binary Parser
      |
      +--> MessageCacheReader       loads all messages into RAM
      |
      +--> StreamingBinaryLoader    reads one message at a time from disk
      |
      v
Decoded Order / Trade Messages
      |
      v
OrderbookBuilder
      |
      +--> apply_filter()
      +--> build_from_list()
      +--> build_from_source()
      +--> orderbook_add_msg()
      |
      v
Snapshot / Full Depth / CSV Row
```

---

## Classes

| Class | Role | Best use case |
|---|---|---|
| `MessageCacheReader` | Loads the full file into memory | Backtesting, repeated analysis, small or medium files |
| `StreamingBinaryLoader` | Reads messages sequentially from disk | Very large files, Jupyter usage, low-memory processing |
| `OrderbookBuilder` | Builds and queries the orderbook | Snapshot generation and market-depth analysis |

---

## Installation / Import

After building and installing the wheel:

```python
from fastreader import MessageCacheReader, StreamingBinaryLoader, OrderbookBuilder
```

You can also import path and contract-master helpers:

```python
from fastreader import FeedPathBuilder, SymbolMaster
```

Build locally with maturin:

```bash
maturin develop --release
```

Or build a wheel:

```bash
maturin build --release
```

---

# FeedPathBuilder

`FeedPathBuilder` builds NSE feed binary file paths for both `NSE_FO` and
`NSE_CM` streams.

```python
from fastreader import FeedPathBuilder

builder = FeedPathBuilder()

fo_path = builder.build(
    segment="NSE_FO",
    stream_id=1,
    day=27,
    month=5,
    year=2026,
)

cm_path = builder.build(
    segment="NSE_CM",
    stream_id=2,
    day=27,
    month=5,
    year=2026,
)
```

Default base path:

```text
/nas/50.30
```

Expected outputs:

```text
/nas/50.30/NSE_FO/Feed_FO_StreamID_1_27_05_2026.bin
/nas/50.30/NSE_CM/Feed_CM_StreamID_2_27_05_2026.bin
```

Custom base path:

```python
fo_path = builder.build(
    segment="NSE_FO",
    stream_id=1,
    day=27,
    month=5,
    year=2026,
    base_path="/mnt/data",
)

print(fo_path)
# /mnt/data/NSE_FO/Feed_FO_StreamID_1_27_05_2026.bin
```

`build_and_verify()` behavior:

```python
verified_path = builder.build_and_verify(
    segment="NSE_FO",
    stream_id=1,
    day=27,
    month=5,
    year=2026,
)
```

- Returns the path string if the file exists.
- Raises `RuntimeError` if the file does not exist.

---

# SymbolMaster

`SymbolMaster` loads NSE contract master CSV files and enriches decoded
messages using token metadata.

```python
from fastreader import SymbolMaster

sm = SymbolMaster()

count = sm.load_for_date(
    segment="NSE_FO",
    day=27,
    month=5,
    year=2026,
)
```

Default path pattern used by `load_for_date()`:

```text
/nas/50.30/CONTRACT/27_05_2026/NSE_FO_contract_27052026.csv
```

Explicit CSV path usage:

```python
count = sm.load("/nas/50.30/CONTRACT/27_05_2026/NSE_FO_contract_27052026.csv")
```

Token lookup:

```python
info = sm.lookup(40434)
```

Lookup result includes these keys:

- `token`
- `found`
- `symbol`
- `name`
- `option_type`
- `strike`
- `expiry`
- `lot_size`

---

# Streaming Enrichment Example

`FeedPathBuilder`, `SymbolMaster`, and `StreamingBinaryLoader` can be used
together so streamed messages are enriched automatically.

```python
from fastreader import FeedPathBuilder, SymbolMaster, StreamingBinaryLoader

path_builder = FeedPathBuilder()

feed_path = path_builder.build(
    segment="NSE_FO",
    stream_id=1,
    day=27,
    month=5,
    year=2026,
)

symbol_master = SymbolMaster()
symbol_master.load_for_date(
    segment="NSE_FO",
    day=27,
    month=5,
    year=2026,
)

reader = StreamingBinaryLoader()
reader.open_stream(feed_path, count_messages=False)
reader.attach_symbol_master(symbol_master)

msg = reader.get_next_msg()
```

When a message token exists in the loaded symbol master, the message includes:

- `token_symbol`
- `strike_price`
- `option_type`
- `expiry`
- `lot_size`
- `name`

---

## Message types

| Message type | Meaning | Packet |
|---|---|---|
| `N` | New order | Order |
| `M` | Modify order | Order |
| `X` | Cancel/delete order | Order |
| `T` | Trade | Trade |

Order side values:

| Side | Meaning |
|---|---|
| `B` | Buy / bid |
| `S` | Sell / ask |

---

# Quick Start: Fast streaming for large files

Use this approach when the file is large and Jupyter becomes slow.

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

file_path = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
processed = builder.build_from_source(reader, limit=100000)

print("Processed:", processed)
print(builder.get_snapshot(token=1001, levels=5))
```

Expected output shape:

```text
Processed: 100000
{
    'token': 1001,
    'found': True,
    'mid_price': 1050,
    'best_bid': (1000, 40),
    'best_ask': (1100, 15),
    'spread': 100,
    'bids': [(1000, 40), (995, 20)],
    'asks': [(1100, 15), (1110, 25)]
}
```

Actual values depend on your binary file and token.

---

# 1. `MessageCacheReader`

`MessageCacheReader` loads all decoded messages into RAM.

Use it when you want to repeatedly inspect the same file or run backtests on a manageable file size.

Avoid it for very large files because RAM usage increases with message count.

## 1.1 Create reader

```python
from fastreader import MessageCacheReader

reader = MessageCacheReader()
```

Expected output: no output. It creates an empty reader.

---

## 1.2 `load_to_cache(file_path)`

Loads all supported messages from a binary file into memory.

```python
count = reader.load_to_cache(file_path)
```

Parameters:

| Name | Type | Meaning |
|---|---|---|
| `file_path` | `str` | Full path of the binary file |

Returns:

| Type | Meaning |
|---|---|
| `int` | Number of messages loaded |

Example:

```python
reader = MessageCacheReader()
count = reader.load_to_cache("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")
print("Loaded messages:", count)
```

Expected output:

```text
Loaded messages: 1250000
```

---

## 1.3 `get_all_messages()`

Returns all cached order and trade messages as formatted strings.

```python
messages = reader.get_all_messages()
```

Example:

```python
messages = reader.get_all_messages()
print(messages[0])
print(messages[1])
```

Expected output:

```text
Order Message: SeqNo 42, MsgLen 10, MsgType 'N', ExchTs 100000, LocalTs 200000, OrderId 55, Token 1001, Side 'B', Price 500, Quantity 100, Missed 0
Trade Message: SeqNo 99, MsgLen 10, MsgType 'T', ExchTs 300000, LocalTs 400000, BuyOrderId 10, SellOrderId 20, Token 5000, Price 750, Quantity 30, Missed 1
```

---

## 1.4 `get_order_message()`

Returns only order messages.

```python
orders = reader.get_order_message()
```

Example:

```python
orders = reader.get_order_message()
print("Order messages:", len(orders))
print(orders[0])
```

Expected output:

```text
Order messages: 900000
Order Message: SeqNo 42, MsgLen 10, MsgType 'N', ExchTs 100000, LocalTs 200000, OrderId 55, Token 1001, Side 'B', Price 500, Quantity 100, Missed 0
```

Note: the current API name is `get_order_message()`, not `get_all_order_message()`.

---

## 1.5 `get_trade_message()`

Returns only trade messages.

```python
trades = reader.get_trade_message()
```

Example:

```python
trades = reader.get_trade_message()
print("Trade messages:", len(trades))
print(trades[0])
```

Expected output:

```text
Trade messages: 350000
Trade Message: SeqNo 99, MsgLen 10, MsgType 'T', ExchTs 300000, LocalTs 400000, BuyOrderId 10, SellOrderId 20, Token 5000, Price 750, Quantity 30, Missed 1
```

---

## 1.6 `get_all_trade_message()`

Alias for `get_trade_message()`.

```python
trades = reader.get_all_trade_message()
```

Example:

```python
trades = reader.get_all_trade_message()
print(trades[:2])
```

Expected output:

```text
[
  "Trade Message: SeqNo 99, MsgLen 10, MsgType 'T', ExchTs 300000, LocalTs 400000, BuyOrderId 10, SellOrderId 20, Token 5000, Price 750, Quantity 30, Missed 1"
]
```

---

## 1.7 `get_cache_summary()`

Returns a Python dictionary with cache statistics.

```python
summary = reader.get_cache_summary()
```

Returned keys:

| Key | Meaning |
|---|---|
| `file_source` | File path loaded into cache |
| `total_messages` | Total messages cached |
| `total_orders` | Total order messages |
| `total_trades` | Total trade messages |
| `memory_usage_bytes` | Estimated memory usage |

Example:

```python
summary = reader.get_cache_summary()
print(summary)
```

Expected output:

```python
{
    'file_source': '/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin',
    'total_messages': 1250000,
    'total_orders': 900000,
    'total_trades': 350000,
    'memory_usage_bytes': 80000000
}
```

---

# 2. `StreamingBinaryLoader`

`StreamingBinaryLoader` opens the binary file and reads messages one by one from disk.

This is the recommended class for large files.

## 2.1 Create loader

```python
from fastreader import StreamingBinaryLoader

reader = StreamingBinaryLoader()
```

Expected output: no output.

---

## 2.2 `open_stream(file_path, count_messages=True)`

Opens a binary file for sequential reading.

```python
count = reader.open_stream(file_path, count_messages=True)
```

Parameters:

| Name | Type | Default | Meaning |
|---|---|---|---|
| `file_path` | `str` | Required | Full binary file path |
| `count_messages` | `bool` | `True` | Whether to scan the file and count messages |

Returns:

| Case | Return |
|---|---|
| `count_messages=True` | Total message count |
| `count_messages=False` | `0` immediately |

For very large files, prefer:

```python
reader.open_stream(file_path, count_messages=False)
```

This opens faster because it skips the full count scan.

Example:

```python
reader = StreamingBinaryLoader()
count = reader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin", count_messages=False)
print("Count:", count)
```

Expected output:

```text
Count: 0
```

Important: `0` does not mean the file is empty. It means counting was skipped.

---

## 2.3 `get_next_msg()`

Reads the next message and returns a decoded Python dictionary.

```python
msg = reader.get_next_msg()
```

Returns:

| Return | Meaning |
|---|---|
| `dict` | Next decoded order/trade message |
| `None` | End of file |

Example:

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

print(reader.get_next_msg())
print(reader.get_next_msg())
print(reader.get_next_msg())
```

Expected output shape:

```python
{'message_kind': 'order', 'seq_no': 1, 'msg_type': 'N', ...}
{'message_kind': 'order', 'seq_no': 2, 'msg_type': 'N', ...}
{'message_kind': 'trade', 'seq_no': 3, 'msg_type': 'T', ...}
```

---

## 2.4 `get_next_msg()` decoded fields

`get_next_msg()` is the main streaming API and returns dictionaries suitable for
storing, filtering, iterating, indexing, and DataFrame conversion.

```python
msg = reader.get_next_msg()
```

Returns:

| Return | Meaning |
|---|---|
| `dict` | Next decoded message |
| `None` | End of file |

Example:

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

msg = reader.get_next_msg()
print(msg)
```

Expected order-message output:

```python
{
    'message_kind': 'order',
    'seq_no': 1,
    'msg_len': 10,
    'stream_id': 2,
    'msg_type': 'N',
    'exch_ts': 100001,
    'local_ts': 200001,
    'order_id': 10,
    'token': 200,
    'order_type': 'B',
    'price': 100,
    'quantity': 5,
    'flags': False
}
```

Expected trade-message output:

```python
{
    'message_kind': 'trade',
    'seq_no': 3,
    'msg_len': 10,
    'stream_id': 2,
    'msg_type': 'T',
    'exch_ts': 300003,
    'local_ts': 400003,
    'buy_order_id': 10,
    'sell_order_id': 20,
    'token': 200,
    'trade_price': 100,
    'trade_quantity': 5,
    'flags': False
}
```

---


## 2.5 `is_end_of_msg()`

Checks whether the next `get_next_msg()` call has reached the end of the file.

This function is useful when you want to check EOF status before reading the next decoded message.

```python
is_end = reader.is_end_of_msg()
```

Returns:

| Return | Meaning |
|---|---|
| `False` | More messages are available |
| `True` | No more messages are available / end of file reached |

Step-by-step example:

```python
from fastreader import StreamingBinaryLoader

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

# Step 1: Check before reading
print(reader.is_end_of_msg())

# Step 2: Read first message
msg = reader.get_next_msg()
print(msg)

# Step 3: Continue until EOF
while not reader.is_end_of_msg():
    msg = reader.get_next_msg()
    print(msg)

# Step 4: Check again after all messages are consumed
print(reader.is_end_of_msg())

# Step 5: Reading after EOF
msg = reader.get_next_msg()
print(msg)
```

Expected output shape:

```text
False
{'message_kind': 'order', 'seq_no': 1, 'msg_type': 'N', ...}
{'message_kind': 'order', 'seq_no': 2, 'msg_type': 'N', ...}
{'message_kind': 'trade', 'seq_no': 3, 'msg_type': 'T', ...}
True
None
```

Important: `is_end_of_msg()` only checks the next message availability. It preserves the current cursor position, so calling it does not consume a message.

---

## 2.6 `reset_cursor()`

Moves the stream cursor back to the beginning of the file.

```python
reader.reset_cursor()
```

Example:

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

first = reader.get_next_msg()
second = reader.get_next_msg()

reader.reset_cursor()
first_again = reader.get_next_msg()

print(first == first_again)
```

Expected output:

```text
True
```

---

# 3. `OrderbookBuilder`

`OrderbookBuilder` processes decoded messages and maintains orderbook state.

It can build from:

- `MessageCacheReader`
- `StreamingBinaryLoader`
- List of decoded Python dictionaries
- One message at a time using `orderbook_add_msg()`

---

## 3.1 Create builder

```python
from fastreader import OrderbookBuilder

builder = OrderbookBuilder()
```

Expected output: no output.

---

## 3.2 `apply_filter(logic_criteria=None)`

Filters which message types should be processed.

```python
builder.apply_filter(["N", "M", "X"])
```

Parameters:

| Value | Meaning |
|---|---|
| `None` | Process all supported messages |
| `["N"]` | Process only new orders |
| `["N", "M", "X"]` | Process order messages only |
| `["T"]` | Process trades only |

Example:

```python
builder = OrderbookBuilder()
builder.apply_filter(["N", "M", "X"])
```

Expected output: no output. The filter is stored inside builder.

Clear filter:

```python
builder.apply_filter(None)
```

---

## 3.3 `build_from_source(source, limit=None)`

Builds orderbook from a reader object.

Accepted sources:

- `MessageCacheReader`
- `StreamingBinaryLoader`

```python
processed = builder.build_from_source(source, limit=None)
```

Parameters:

| Name | Type | Meaning |
|---|---|---|
| `source` | reader object | Cache reader or streaming reader |
| `limit` | `int` or `None` | Maximum accepted messages to process from stream |

Example with streaming:

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
processed = builder.build_from_source(reader, limit=100000)

print("Processed:", processed)
```

Expected output:

```text
Processed: 100000
```

Example with cache:

```python
reader = MessageCacheReader()
reader.load_to_cache(file_path)

builder = OrderbookBuilder()
processed = builder.build_from_source(reader)

print("Processed:", processed)
```

Expected output:

```text
Processed: 1250000
```

---

## 3.4 `build_from_list(source)`

Builds the orderbook from either:

1. A `MessageCacheReader`
2. A Python `list[dict]` of decoded messages

```python
processed = builder.build_from_list(source)
```

Example with `MessageCacheReader`:

```python
reader = MessageCacheReader()
reader.load_to_cache(file_path)

builder = OrderbookBuilder()
processed = builder.build_from_list(reader)

print("Processed:", processed)
```

Expected output:

```text
Processed: 1250000
```

Example with list of dictionaries:

```python
messages = [
    {
        "msg_type": "N",
        "exch_ts": 100000,
        "order_id": 1,
        "token": 777,
        "order_type": "B",
        "price": 1000,
        "quantity": 40,
        "local_ts": 200000,
        "flags": False,
    },
    {
        "msg_type": "N",
        "exch_ts": 100001,
        "order_id": 2,
        "token": 777,
        "order_type": "S",
        "price": 1100,
        "quantity": 15,
        "local_ts": 200001,
        "flags": False,
    },
]

builder = OrderbookBuilder()
processed = builder.build_from_list(messages)
print("Processed:", processed)
```

Expected output:

```text
Processed: 2
```

---

## 3.5 `orderbook_add_msg(msg)`

Processes exactly one already-decoded message dictionary.

This function expects one message returned by `reader.get_next_msg()`.

```python
processed = builder.orderbook_add_msg(msg)
```

Returns:

| Return | Meaning |
|---|---|
| `True` | Message was accepted and applied |
| `False` | Message was valid but skipped by filter/business rules |

Correct usage:

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()

msg = reader.get_next_msg()
if msg is not None:
    processed = builder.orderbook_add_msg(msg)
    print("Processed:", processed)
```

Expected output:

```text
Processed: True
```

Loop usage:

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()

while True:
    msg = reader.get_next_msg()
    if msg is None:
        break
    builder.orderbook_add_msg(msg)

print(builder.get_snapshot(token=1001, levels=5))
```

Important: do not pass the `reader` object directly to `orderbook_add_msg()`. Pass one decoded message dictionary.

Wrong:

```python
builder.orderbook_add_msg(reader)   # wrong
```

Right:

```python
msg = reader.get_next_msg()
builder.orderbook_add_msg(msg)      # right
```

---

## 3.6 `get_snapshot(token, levels=None)`

Returns top bid/ask levels for one token.

```python
snapshot = builder.get_snapshot(token=1001, levels=5)
```

Parameters:

| Name | Type | Default | Meaning |
|---|---|---|---|
| `token` | `int` | Required | Instrument token |
| `levels` | `int` or `None` | `5` | Number of bid/ask levels |

Returns dictionary:

| Key | Meaning |
|---|---|
| `token` | Requested token |
| `found` | Whether orderbook exists for token |
| `mid_price` | `(best_bid + best_ask) / 2` when available |
| `best_bid` | Best bid tuple `(price, quantity)` |
| `best_ask` | Best ask tuple `(price, quantity)` |
| `spread` | `best_ask_price - best_bid_price` |
| `bids` | Top bid levels |
| `asks` | Top ask levels |

Example:

```python
snapshot = builder.get_snapshot(token=777, levels=5)
print(snapshot)
```

Expected output:

```python
{
    'token': 777,
    'found': True,
    'mid_price': 1050,
    'best_bid': (1000, 40),
    'best_ask': (1100, 15),
    'spread': 100,
    'bids': [(1000, 40)],
    'asks': [(1100, 15)]
}
```

If token is not found:

```python
{
    'token': 99999,
    'found': False,
    'mid_price': 0,
    'best_bid': None,
    'best_ask': None,
    'spread': None,
    'bids': [],
    'asks': []
}
```

---

## 3.7 `get_full_depth(token)`

Returns full available depth for one token.

```python
depth = builder.get_full_depth(token=1001)
```

Returns dictionary:

| Key | Meaning |
|---|---|
| `token` | Requested token |
| `found` | Whether token was found |
| `best_bid` | Best bid level |
| `best_ask` | Best ask level |
| `spread` | Ask minus bid |
| `bids` | All bid levels |
| `asks` | All ask levels |

Example:

```python
depth = builder.get_full_depth(token=777)
print(depth)
```

Expected output:

```python
{
    'token': 777,
    'found': True,
    'best_bid': (1000, 40),
    'best_ask': (1100, 15),
    'spread': 100,
    'bids': [(1000, 40), (995, 20), (990, 10)],
    'asks': [(1100, 15), (1110, 25), (1120, 30)]
}
```

---

## 3.9 `snapshot_header()`

Returns CSV header for snapshot rows.

```python
header = builder.snapshot_header()
```

Example:

```python
print(builder.snapshot_header())
```

Expected output:

```text
local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,ask_qty_1,bid_price_2,bid_qty_2,ask_price_2,ask_qty_2,bid_price_3,bid_qty_3,ask_price_3,ask_qty_3,bid_price_4,bid_qty_4,ask_price_4,ask_qty_4
```

---

## 3.10 `get_snapshot_row(token, levels=None)`

Returns one CSV-style row for a token snapshot.

```python
row = builder.get_snapshot_row(token=1001, levels=5)
```

Example:

```python
print(builder.snapshot_header())
print(builder.get_snapshot_row(token=777, levels=5))
```

Expected output:

```text
local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,ask_qty_1,bid_price_2,bid_qty_2,ask_price_2,ask_qty_2,bid_price_3,bid_qty_3,ask_price_3,ask_qty_3,bid_price_4,bid_qty_4,ask_price_4,ask_qty_4
0,0,1050,1000,40,1100,15,995,20,1110,25,990,10,1120,30,0,0,0,0,0,0,0,0
```

---

# Recommended Workflows

## Workflow A: Large file, fastest Jupyter usage

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

file_path = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
processed = builder.build_from_source(reader, limit=500000)

print("Processed:", processed)
print(builder.get_snapshot(token=1001, levels=5))
```

Why this is fast:

- File is not loaded into RAM.
- Message counting is skipped.
- Rust reads and processes messages directly.

---

## Workflow B: Load once, analyze many times

```python
from fastreader import MessageCacheReader, OrderbookBuilder

reader = MessageCacheReader()
count = reader.load_to_cache(file_path)

print(reader.get_cache_summary())

builder = OrderbookBuilder()
processed = builder.build_from_source(reader)

print(builder.get_snapshot(token=1001, levels=5))
```

Use this when the file fits comfortably in RAM.

---

## Workflow C: Process one message at a time

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()

while True:
    msg = reader.get_next_msg()
    if msg is None:
        break

    accepted = builder.orderbook_add_msg(msg)

print(builder.get_snapshot(token=1001, levels=5))
```

Use this when you want full control over each message.

---

## Workflow D: Only process order messages

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
builder.apply_filter(["N", "M", "X"])

processed = builder.build_from_source(reader)
print("Processed order messages:", processed)
```

---

## Workflow E: Only process new orders

```python
builder = OrderbookBuilder()
builder.apply_filter(["N"])
```

---

# Error Handling

## File does not exist

```python
reader = StreamingBinaryLoader()
reader.open_stream("/wrong/path/file.bin")
```

Expected error:

```text
RuntimeError: No such file or directory
```

## Invalid binary file

If the first valid message type is not one of `T`, `N`, `M`, or `X`, the library raises an error.

Expected error shape:

```text
RuntimeError: invalid first message type: <value>
```

## Passing wrong object to `orderbook_add_msg()`

Wrong:

```python
builder.orderbook_add_msg(reader)
```

Expected error:

```text
TypeError: orderbook_add_msg expects one message dict from get_next_msg()
```

Right:

```python
msg = reader.get_next_msg()
builder.orderbook_add_msg(msg)
```

---

# Performance Tips

## For very large files

Use:

```python
reader.open_stream(file_path, count_messages=False)
```

Do not use `load_to_cache()` unless you have enough RAM.

## For fastest orderbook building

Use:

```python
builder.build_from_source(reader)
```

This keeps the processing path simple and Rust-heavy.

## For debugging first few messages

Use:

```python
print(reader.get_next_msg())
print(reader.get_next_msg())
print(reader.get_next_msg())
```

## For Python-level custom logic

Use:

```python
msg = reader.get_next_msg()
builder.orderbook_add_msg(msg)
```

This gives Python access to every decoded message.

---

# Complete Example

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

file_path = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"
token = 1001

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
builder.apply_filter(["N", "M", "X", "T"])

processed = builder.build_from_source(reader, limit=1000000)

print("Processed:", processed)
print("Snapshot:")
print(builder.get_snapshot(token=token, levels=5))

print("CSV:")
print(builder.snapshot_header())
print(builder.get_snapshot_row(token=token, levels=5))
```

Expected output shape:

```text
Processed: 1000000
Snapshot:
{'token': 1001, 'found': True, 'mid_price': 1050, 'best_bid': (1000, 40), 'best_ask': (1100, 15), 'spread': 100, 'bids': [(1000, 40)], 'asks': [(1100, 15)]}
CSV:
local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,bid_qty_1,...
0,0,1050,1000,40,1100,15,0,0,0,0,...
```

---

# API Summary

## `MessageCacheReader`

| Function | Description |
|---|---|
| `load_to_cache(file_path)` | Load full binary file into memory |
| `get_all_messages()` | Return all cached messages as strings |
| `get_order_message()` | Return only order messages |
| `get_trade_message()` | Return only trade messages |
| `get_all_trade_message()` | Alias for trade messages |
| `get_cache_summary()` | Return file, count, and memory summary |

## `StreamingBinaryLoader`

| Function | Description |
|---|---|
| `open_stream(file_path, count_messages=True)` | Open binary file for streaming |
| `get_next_msg()` | Return next message as Python dictionary |
| `is_end_of_msg()` | Check whether the next decoded message read is at EOF |
| `reset_cursor()` | Move cursor back to start of file |

## `OrderbookBuilder`

| Function | Description |
|---|---|
| `apply_filter(logic_criteria=None)` | Filter message types |
| `orderbook_add_msg(msg)` | Process one decoded message dictionary |
| `build_from_list(source)` | Build from cache reader or list of dict messages |
| `build_from_source(source, limit=None)` | Build from cache reader or stream reader |
| `get_snapshot(token, levels=None)` | Return top-N orderbook levels |
| `get_full_depth(token)` | Return full depth for token |
| `snapshot_header()` | Return CSV snapshot header |
| `get_snapshot_row(token, levels=None)` | Return CSV snapshot row |

---

# Notes for Python users

- Use `StreamingBinaryLoader` for huge files.
- Use `count_messages=False` when opening huge files in Jupyter.
- Use `MessageCacheReader` only when the file comfortably fits in RAM.
- Use `build_from_source()` for simple and fast orderbook building.
- Use `get_next_msg()` plus `orderbook_add_msg()` when you need custom Python logic per message.
- Use `get_snapshot()` for Python dictionary output.
- Use `snapshot_header()` and `get_snapshot_row()` for CSV-style output.

---


