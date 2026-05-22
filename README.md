# OrderPulse / `fastreader`

A high-performance Python library for reading NSE binary order/trade feed files and building real-time orderbook snapshots.

Written in Rust, exposed to Python via PyO3 — binary parsing and orderbook processing runs entirely in Rust, giving you a fast and simple Python API.

---

## What this library does

- Builds NSE feed file paths dynamically from segment, stream ID, and date.
- Reads NSE binary feed files (CM and FO segments).
- Extracts order and trade messages.
- Supports both RAM-based (cache) and streaming (low-memory) reading.
- Builds token-level orderbook state from decoded messages.
- Returns best bid, best ask, spread, mid price, top levels, full depth, and CSV rows.
- Loads the NSE contract master CSV in Rust and enriches every message with symbol, strike price, option type, expiry, and lot size — no Python CSV parsing.

---

## Architecture

```text
FeedPathBuilder
      |
      +--> build()              construct path string from segment + stream_id + date
      +--> build_and_verify()   same, but also checks the file exists on disk
      |
      v
Binary Feed File  (.bin)
      |
      v
Rust Binary Parser
      |
      +--> MessageCacheReader       load all messages into RAM at once
      +--> StreamingBinaryLoader    read one message at a time from disk
      |         |
      |         +--> attach_symbol_master()   auto-enrich every get_next_msg() call
      |
      v
Decoded Order / Trade Messages  ← token_symbol / strike_price / option_type / expiry populated
      |
      v
OrderbookBuilder
      |
      +--> apply_filter()           restrict which message types to process
      +--> build_from_source()      build from reader (recommended)
      +--> build_from_list()        build from cache reader or list[dict]
      +--> orderbook_add_msg()      feed one message at a time manually
      |
      v
Snapshots / Full Depth / CSV Rows

NSE Contract Master CSV
      |
      v
SymbolMaster  (Rust CSV parser — no Python overhead)
      |
      +--> load(csv_path)                  load from explicit path
      +--> load_for_date(seg, d, m, y)     build path automatically and load
      +--> lookup(token)                   single-token info dict
      +--> enrich(msg)                     populate symbol fields in a msg dict
```

---

## Classes

| Class | Purpose | When to use |
|---|---|---|
| `FeedPathBuilder` | Constructs NSE feed file paths | Avoid hardcoded path strings |
| `MessageCacheReader` | Loads entire file into RAM | Small/medium files, repeated analysis |
| `StreamingBinaryLoader` | Reads messages one by one from disk | Large files, Jupyter, low memory |
| `OrderbookBuilder` | Processes messages and queries orderbook state | Snapshot and depth analysis |
| `SymbolMaster` | Loads contract master CSV, looks up symbol info by token | Enriching messages with instrument metadata |

---

## Installation

Build and install locally with maturin:

```bash
maturin develop --release
```

Build a distributable wheel:

```bash
maturin build --release
```

Import in Python:

```python
from fastreader import (
    FeedPathBuilder, MessageCacheReader,
    StreamingBinaryLoader, OrderbookBuilder,
    SymbolMaster,
)
```

---

## Message Types

| Type | Meaning |
|---|---|
| `N` | New order |
| `M` | Modify order |
| `X` | Cancel / delete order |
| `T` | Trade |

Order side values:

| Side | Meaning |
|---|---|
| `B` | Buy (bid) |
| `S` | Sell (ask) |

---

# Quick Start

Copy and run this block to get a snapshot from your first NSE file:

```python
from fastreader import FeedPathBuilder, StreamingBinaryLoader, OrderbookBuilder

# 1. Build and verify the file path
b = FeedPathBuilder()
file_path = b.build_and_verify("NSE_FO", stream_id=1, day=21, month=5, year=2026)
print("File:", file_path)

# 2. Open the stream (fast — no full-file scan)
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

# 3. Process the first 500 000 messages into the orderbook
builder = OrderbookBuilder()
processed = builder.build_from_source(reader, limit=500_000)
print(f"Processed: {processed} messages")

# 4. Discover active tokens
tokens = builder.get_active_tokens()
print(f"Active tokens: {len(tokens)}  |  first 5: {tokens[:5]}")

# 5. Query a snapshot
snap = builder.get_snapshot(token=tokens[0], levels=5)
print(snap)
```

Expected terminal output:

```
File: /nas/50.30/NSE_FO/Feed_FO_StreamID_1_21_05_2026.bin
Processed: 500000 messages
Active tokens: 261  |  first 5: [36687, 40434, 42174, 42175, 42194]
{'token': 36687, 'found': True, 'mid_price': 408562, 'best_bid': (322585, 1740), 'best_ask': (494540, 120), 'spread': 171955, 'bids': [(322585, 1740), (322580, 120)], 'asks': [(494540, 120)]}
```

> **Note on `get_next_message()`:** this method returns a `(payload, is_end_of_stream)` tuple.
> `is_end_of_stream` is `True` only when the file is exhausted.
>
> ```python
> payload, is_end = reader.get_next_message()
> print(payload)   # "Order Message: SeqNo 1, MsgType 'N', ..."
> print(is_end)    # False
> ```

---

# 1. `FeedPathBuilder`

Constructs NSE binary feed file paths from components. Avoids hardcoding paths and keeps naming consistent with the NSE format.

Path format produced:

```
{base_path}/{FOLDER}/Feed_{SHORT}_StreamID_{id}_{DD}_{MM}_{YYYY}.bin
```

Default `base_path` is `/nas/50.30`.

| Segment input | Folder | Short name |
|---|---|---|
| `"NSE_CM"` or `"CM"` | `NSE_CM` | `CM` |
| `"NSE_FO"` or `"FO"` | `NSE_FO` | `FO` |

All segment values are case-insensitive.

---

## 1.1 Create

```python
from fastreader import FeedPathBuilder

b = FeedPathBuilder()
```

---

## 1.2 `build(segment, stream_id, day, month, year, base_path=None)`

Constructs and returns a file path string. Does **not** check whether the file exists on disk.

| Parameter | Type | Required | Description |
|---|---|---|---|
| `segment` | `str` | Yes | `"NSE_CM"`, `"CM"`, `"NSE_FO"`, or `"FO"` |
| `stream_id` | `int` | Yes | Stream identifier — must be > 0 |
| `day` | `int` | Yes | Day of month, 1–31 |
| `month` | `int` | Yes | Month, 1–12 |
| `year` | `int` | Yes | Four-digit year, 2000–2100 |
| `base_path` | `str` | No | Root directory; defaults to `/nas/50.30` |

Returns `str`. Raises `RuntimeError` for invalid inputs.

```python
b = FeedPathBuilder()

print(b.build("NSE_CM", stream_id=2, day=29, month=12, year=2025))
# /nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin

print(b.build("CM", stream_id=5, day=3, month=1, year=2025))
# /nas/50.30/NSE_CM/Feed_CM_StreamID_5_03_01_2025.bin

print(b.build("NSE_FO", stream_id=1, day=1, month=6, year=2026, base_path="/mnt/data"))
# /mnt/data/NSE_FO/Feed_FO_StreamID_1_01_06_2026.bin
```

**Validation errors:**

```python
b.build("INVALID", stream_id=1, day=1, month=1, year=2026)
# RuntimeError: unknown segment 'INVALID' — expected one of: NSE_CM, CM, NSE_FO, FO

b.build("NSE_CM", stream_id=0, day=1, month=1, year=2026)
# RuntimeError: stream_id must be > 0

b.build("NSE_CM", stream_id=1, day=1, month=13, year=2026)
# RuntimeError: invalid month 13 — must be 1–12
```

---

## 1.3 `build_and_verify(segment, stream_id, day, month, year, base_path=None)`

Same as `build()`, but also checks that the file exists on disk. Raises `RuntimeError` if it does not.

Same parameters as `build()`.

```python
b = FeedPathBuilder()

try:
    path = b.build_and_verify("NSE_CM", stream_id=2, day=29, month=12, year=2025)
    print("File ready:", path)
except RuntimeError as e:
    print("Not found:", e)
```

Expected output when file exists:

```
File ready: /nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
```

Expected output when file is missing:

```
Not found: file not found: /nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
```

---

# 2. `MessageCacheReader`

Loads all decoded messages from a binary file into RAM. Good for repeated analysis of the same file.

> **Tip:** For very large files use `StreamingBinaryLoader` instead to avoid high RAM usage.

---

## 2.1 Create

```python
from fastreader import MessageCacheReader

reader = MessageCacheReader()
```

---

## 2.2 `load_to_cache(file_path)`

Reads and decodes the entire binary file into memory.

| Parameter | Type | Description |
|---|---|---|
| `file_path` | `str` | Full path to the binary file |

Returns `int` — number of messages loaded.

```python
reader = MessageCacheReader()
count = reader.load_to_cache("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")
print("Loaded:", count)
```

Expected output:

```
Loaded: 1250000
```

---

## 2.3 `get_all_messages()`

Returns all cached messages (orders and trades) as formatted strings.

```python
messages = reader.get_all_messages()
print(messages[0])
print(messages[1])
```

Expected output:

```
Order Message: SeqNo 42, MsgLen 10, MsgType 'N', ExchTs 100000, LocalTs 200000, OrderId 55, Token 1001, Side 'B', Price 500, Quantity 100, Missed 0
Trade Message: SeqNo 99, MsgLen 10, MsgType 'T', ExchTs 300000, LocalTs 400000, BuyOrderId 10, SellOrderId 20, Token 5000, Price 750, Quantity 30, Missed 1
```

---

## 2.4 `get_order_message()`

Returns only order-type messages (`N`, `M`, `X`) as formatted strings.

```python
orders = reader.get_order_message()
print(f"Total orders: {len(orders)}")
print(orders[0])
```

Expected output:

```
Total orders: 900000
Order Message: SeqNo 42, MsgLen 10, MsgType 'N', ExchTs 100000, LocalTs 200000, OrderId 55, Token 1001, Side 'B', Price 500, Quantity 100, Missed 0
```

---

## 2.5 `get_trade_message()`

Returns only trade messages (`T`) as formatted strings.

```python
trades = reader.get_trade_message()
print(f"Total trades: {len(trades)}")
print(trades[0])
```

Expected output:

```
Total trades: 350000
Trade Message: SeqNo 99, MsgLen 10, MsgType 'T', ExchTs 300000, LocalTs 400000, BuyOrderId 10, SellOrderId 20, Token 5000, Price 750, Quantity 30, Missed 1
```

---

## 2.6 `get_all_trade_message()`

Alias for `get_trade_message()`. Identical behaviour.

```python
trades = reader.get_all_trade_message()
```

---

## 2.7 `get_cache_summary()`

Returns a dictionary with file and memory statistics.

| Key | Description |
|---|---|
| `file_source` | Path of the loaded file |
| `total_messages` | Total messages in cache |
| `total_orders` | Order messages count |
| `total_trades` | Trade messages count |
| `memory_usage_bytes` | Estimated RAM usage in bytes |

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

# 3. `StreamingBinaryLoader`

Reads messages one by one directly from disk without loading everything into RAM. The right choice for large files.

---

## 3.1 Create

```python
from fastreader import StreamingBinaryLoader

reader = StreamingBinaryLoader()
```

---

## 3.2 `open_stream(file_path, count_messages=True)`

Opens a binary file for sequential streaming.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `file_path` | `str` | Required | Full path to the binary file |
| `count_messages` | `bool` | `True` | Whether to scan the whole file to count messages |

Returns `int` — message count when `count_messages=True`, or `0` when `False`.

> **Tip:** For large files always use `count_messages=False`. Counting requires a full scan of the file.

```python
reader = StreamingBinaryLoader()

# Fast open — skip counting
count = reader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin", count_messages=False)
print(count)   # 0 — counting was skipped, not that the file is empty

# Slow open — includes full count
count = reader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin", count_messages=True)
print(count)   # 1250000
```

---

## 3.3 `get_next_message()`

Reads and returns the next message as a formatted string. Returns `"END"` at end of file.

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

print(reader.get_next_message())
print(reader.get_next_message())
print(reader.get_next_message())
```

Expected output:

```
Order Message: SeqNo 1, MsgLen 10, MsgType 'N', ExchTs 100001, LocalTs 200001, OrderId 10, Token 200, Side 'B', Price 100, Quantity 5, Missed 0
Order Message: SeqNo 2, MsgLen 10, MsgType 'M', ExchTs 100002, LocalTs 200002, OrderId 10, Token 200, Side 'B', Price 102, Quantity 5, Missed 0
Trade Message: SeqNo 3, MsgLen 10, MsgType 'T', ExchTs 300003, LocalTs 400003, BuyOrderId 10, SellOrderId 20, Token 200, Price 100, Quantity 5, Missed 0
```

At end of file: returns `"END"`.

---

## 3.4 `get_next_msg()`

Reads and returns the next message as a Python dictionary. Returns `None` at end of file.

Use this when you need to inspect individual fields or pass messages to `orderbook_add_msg()`.

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

msg = reader.get_next_msg()
print(msg)
```

Expected output for an order message:

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

Expected output for a trade message:

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

At end of file: returns `None`.

---

## 3.5 `reset_cursor()`

Moves the stream read position back to the start of the file so you can read it again.

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

first = reader.get_next_message()

reader.reset_cursor()

first_again = reader.get_next_message()

print(first == first_again)   # True
```

---

## 3.6 `attach_symbol_master(sm)`

Attaches a loaded `SymbolMaster` to the stream. After this call every `get_next_msg()` dict will have its symbol fields automatically populated.

| Parameter | Type | Description |
|---|---|---|
| `sm` | `SymbolMaster` | A loaded `SymbolMaster` instance |

The lookup runs entirely in Rust — no Python overhead per message.

```python
from fastreader import SymbolMaster, StreamingBinaryLoader

sm = SymbolMaster()
sm.load_for_date("NSE_FO", day=21, month=5, year=2026)

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)
reader.attach_symbol_master(sm)

msg = reader.get_next_msg()
print(msg['token_symbol'])   # e.g. 'FINNIFTY'
print(msg['strike_price'])   # e.g. 21700  (rupees)
print(msg['option_type'])    # e.g. 'CE'
print(msg['expiry'])         # e.g. '26-May-2026'
print(msg['name'])           # e.g. 'FINNIFTY26MAY21700CE'
print(msg['lot_size'])       # e.g. 40
```

When no match is found for a token the original `None` values remain unchanged.

---

## 3.7 `detach_symbol_master()`

Removes the attached `SymbolMaster`. After this call `get_next_msg()` returns `None` for symbol fields again.

```python
reader.detach_symbol_master()
```

---

# 4. `OrderbookBuilder`

Processes decoded messages and maintains live orderbook state per token. Query snapshots and depth after processing.

---

## 4.1 Create

```python
from fastreader import OrderbookBuilder

builder = OrderbookBuilder()
```

---

## 4.2 `apply_filter(logic_criteria=None)`

Restricts which message types are processed. By default, all types are processed.

| Value | Effect |
|---|---|
| `None` | Process all message types (default) |
| `["N", "M", "X"]` | Process only order messages |
| `["N"]` | Process only new orders |
| `["T"]` | Process only trades |

```python
builder = OrderbookBuilder()
builder.apply_filter(["N", "M", "X"])   # orders only
```

Clear the filter (process everything again):

```python
builder.apply_filter(None)
```

---

## 4.3 `build_from_source(source, limit=None)`

Builds the orderbook by reading from a `StreamingBinaryLoader` or `MessageCacheReader`.

This is the recommended method for most use cases.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `source` | reader object | Required | `StreamingBinaryLoader` or `MessageCacheReader` |
| `limit` | `int` or `None` | `None` (all) | Maximum number of messages to process |

Returns `int` — number of messages processed.

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

reader = StreamingBinaryLoader()
reader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin", count_messages=False)

builder = OrderbookBuilder()
processed = builder.build_from_source(reader, limit=500000)

print("Processed:", processed)
```

Expected output:

```
Processed: 500000
```

---

## 4.4 `build_from_list(source)`

Builds the orderbook from either a `MessageCacheReader` or a Python `list[dict]` of decoded message dictionaries.

Returns `int` — number of messages processed.

```python
# From MessageCacheReader
reader = MessageCacheReader()
reader.load_to_cache(file_path)

builder = OrderbookBuilder()
processed = builder.build_from_list(reader)
print("Processed:", processed)
# Processed: 1250000
```

```python
# From a list of message dicts
messages = [
    {
        "msg_type": "N", "exch_ts": 100000, "order_id": 1,
        "token": 777, "order_type": "B", "price": 1000,
        "quantity": 40, "local_ts": 200000, "flags": False,
    },
    {
        "msg_type": "N", "exch_ts": 100001, "order_id": 2,
        "token": 777, "order_type": "S", "price": 1100,
        "quantity": 15, "local_ts": 200001, "flags": False,
    },
]

builder = OrderbookBuilder()
processed = builder.build_from_list(messages)
print("Processed:", processed)
# Processed: 2
```

---

## 4.5 `orderbook_add_msg(msg)`

Processes exactly one decoded message dictionary returned by `reader.get_next_msg()`.

| Return | Meaning |
|---|---|
| `True` | Message accepted and applied to orderbook |
| `False` | Message skipped by filter or business rules |

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()

msg = reader.get_next_msg()
if msg is not None:
    result = builder.orderbook_add_msg(msg)
    print("Accepted:", result)
# Accepted: True
```

Loop example:

```python
while True:
    msg = reader.get_next_msg()
    if msg is None:
        break
    builder.orderbook_add_msg(msg)
```

> **Note:** Pass one message dict — not the reader object.
>
> Wrong: `builder.orderbook_add_msg(reader)` → `TypeError`
>
> Right: `builder.orderbook_add_msg(reader.get_next_msg())`

---

## 4.6 `get_active_tokens()`

Returns a sorted list of all token IDs seen during processing. Use this to discover which instruments are in your data before querying snapshots.

Returns `list[int]`.

```python
tokens = builder.get_active_tokens()
print(f"Active tokens ({len(tokens)} total):", tokens[:10])
```

Expected output:

```
Active tokens (261 total): [36687, 40434, 42174, 42175, 42194, 42195, 42219, 42244, 42258, 42259]
```

Find tokens that have live bid or ask depth:

```python
live = [t for t in builder.get_active_tokens()
        if builder.get_snapshot(token=t, levels=1).get('best_bid') is not None
        or builder.get_snapshot(token=t, levels=1).get('best_ask') is not None]

print(f"Tokens with depth: {len(live)}")
print(builder.get_snapshot(token=live[0], levels=5))
```

> **Always use `get_active_tokens()` first** when working with a new file — token numbers are NSE-specific and are not guessable.

---

## 4.7 `get_snapshot(token, levels=None)`

Returns the top bid and ask levels for a token.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `token` | `int` | Required | Instrument token |
| `levels` | `int` or `None` | `5` | Number of price levels to return |

Returned dictionary:

| Key | Type | Description |
|---|---|---|
| `token` | `int` | Requested token |
| `found` | `bool` | Whether the token has data |
| `mid_price` | `int` | `(best_bid_price + best_ask_price) / 2` (in paise) |
| `best_bid` | `(int, int)` or `None` | Best bid `(price, quantity)` |
| `best_ask` | `(int, int)` or `None` | Best ask `(price, quantity)` |
| `spread` | `int` or `None` | `best_ask_price − best_bid_price` |
| `bids` | `list[(int, int)]` | Top bid levels, best first |
| `asks` | `list[(int, int)]` | Top ask levels, best first |

All prices are in paise (1 rupee = 100 paise).

```python
snapshot = builder.get_snapshot(token=40434, levels=5)
print(snapshot)
```

Expected output (token found):

```python
{
    'token': 40434,
    'found': True,
    'mid_price': 408562,
    'best_bid': (322585, 1740),
    'best_ask': (494540, 120),
    'spread': 171955,
    'bids': [(322585, 1740), (322580, 120)],
    'asks': [(494540, 120)]
}
```

Expected output (token not found):

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

## 4.8 `get_orderbook_snapshot(token, levels=None)`

Alias for `get_snapshot()`. Identical behaviour and output.

```python
snapshot = builder.get_orderbook_snapshot(token=40434, levels=5)
```

---

## 4.9 `get_full_depth(token)`

Returns all available bid and ask levels for a token — no top-N limit.

| Parameter | Type | Description |
|---|---|---|
| `token` | `int` | Instrument token |

| Key | Description |
|---|---|
| `token` | Requested token |
| `found` | Whether the token has data |
| `best_bid` | Best bid level `(price, qty)` |
| `best_ask` | Best ask level `(price, qty)` |
| `spread` | Ask price − bid price |
| `bids` | All bid levels, best first |
| `asks` | All ask levels, best first |

```python
depth = builder.get_full_depth(token=40434)
print(depth)
```

Expected output:

```python
{
    'token': 40434,
    'found': True,
    'best_bid': (322585, 1740),
    'best_ask': (494540, 120),
    'spread': 171955,
    'bids': [(322585, 1740), (322580, 120), (322575, 300)],
    'asks': [(494540, 120), (495000, 250)]
}
```

---

## 4.10 `snapshot_header()`

Returns the CSV column header string for snapshot rows.

```python
print(builder.snapshot_header())
```

Expected output:

```
local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,ask_qty_1,bid_price_2,bid_qty_2,ask_price_2,ask_qty_2,bid_price_3,bid_qty_3,ask_price_3,ask_qty_3,bid_price_4,bid_qty_4,ask_price_4,ask_qty_4
```

---

## 4.11 `get_snapshot_row(token, levels=None)`

Returns one CSV-formatted data row for a token snapshot. Pair with `snapshot_header()` to write CSV files.

| Parameter | Type | Default | Description |
|---|---|---|---|
| `token` | `int` | Required | Instrument token |
| `levels` | `int` or `None` | `5` | Number of price levels |

Returns `str`.

```python
print(builder.snapshot_header())
print(builder.get_snapshot_row(token=40434, levels=5))
```

Expected output:

```
local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,...
0,0,408562,322585,1740,494540,120,322580,120,0,0,0,0,0,0,0,0,0,0,0,0,0,0
```

---

# 5. `SymbolMaster`

Loads the NSE FO or CM contract master CSV entirely in Rust and provides fast `O(1)` token → symbol lookups. Use it to populate `token_symbol`, `strike_price`, `option_type`, `expiry`, `lot_size`, and `name` on every decoded message.

---

## 5.1 Create

```python
from fastreader import SymbolMaster

sm = SymbolMaster()
```

---

## 5.2 `load(csv_path)`

Loads the contract master from an explicit file path.

| Parameter | Type | Description |
|---|---|---|
| `csv_path` | `str` | Full path to the NSE contract master CSV |

Returns `int` — number of contracts loaded.

```python
count = sm.load("/nas/50.30/CONTRACT/21_05_2026/NSE_FO_contract_21052026.csv")
print(count)   # 95632
```

Raises `RuntimeError` if the file cannot be opened or a required column is missing.

---

## 5.3 `load_for_date(segment, day, month, year, base_path=None)`

Builds the standard NSE contract master path from date components and loads it.

Path pattern produced:

```
{base_path}/CONTRACT/{DD}_{MM}_{YYYY}/NSE_{FO|CM}_contract_{DD}{MM}{YYYY}.csv
```

| Parameter | Type | Default | Description |
|---|---|---|---|
| `segment` | `str` | Required | `"NSE_FO"`, `"FO"`, `"NSE_CM"`, or `"CM"` |
| `day` | `int` | Required | Day of month, 1–31 |
| `month` | `int` | Required | Month, 1–12 |
| `year` | `int` | Required | Four-digit year |
| `base_path` | `str` | `"/nas/50.30"` | Root directory |

Returns `int` — number of contracts loaded.

```python
sm = SymbolMaster()
count = sm.load_for_date("NSE_FO", day=21, month=5, year=2026)
print(count)   # 95632
print(sm)      # SymbolMaster(contracts=95632)
```

---

## 5.4 `lookup(token)`

Returns a dictionary with full contract metadata for a token.

| Parameter | Type | Description |
|---|---|---|
| `token` | `int` | Instrument token ID |

Returned dictionary:

| Key | Type | Description |
|---|---|---|
| `token` | `int` | The requested token |
| `found` | `bool` | Whether the token is in the loaded master |
| `symbol` | `str` or `None` | Ticker symbol, e.g. `"FINNIFTY"` |
| `name` | `str` or `None` | Full instrument name, e.g. `"FINNIFTY26MAY21700CE"` |
| `option_type` | `str` or `None` | `"CE"`, `"PE"`, or `"XX"` (futures) |
| `strike` | `int` or `None` | Strike price in rupees (`-1` for futures) |
| `expiry` | `str` or `None` | Expiry date, e.g. `"26-May-2026"` |
| `lot_size` | `int` or `None` | Lot size |

```python
info = sm.lookup(token=40434)
print(info)
```

Expected output (token found):

```python
{
    'token': 40434,
    'found': True,
    'symbol': 'FINNIFTY',
    'name': 'FINNIFTY26MAY21700CE',
    'option_type': 'CE',
    'strike': 21700,
    'expiry': '26-May-2026',
    'lot_size': 40
}
```

Expected output (token not in master):

```python
{'token': 99999, 'found': False, 'symbol': None, 'name': None,
 'option_type': None, 'strike': None, 'expiry': None, 'lot_size': None}
```

---

## 5.5 `enrich(msg)`

Populates symbol fields directly on a message dict returned by `get_next_msg()`. Modifies the dict in place.

Added / overwritten keys: `token_symbol`, `strike_price`, `option_type`, `expiry`, `lot_size`, `name`.

Returns `True` if the token was found and the dict was enriched, `False` if not found.

| Parameter | Type | Description |
|---|---|---|
| `msg` | `dict` | A message dict from `get_next_msg()` |

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

msg = reader.get_next_msg()
found = sm.enrich(msg)

if found:
    print(msg['token_symbol'], msg['strike_price'], msg['option_type'])
    # FINNIFTY  21700  CE
```

No-op when the token is not in the loaded master — the dict is not modified.

```python
for msg in iter(reader.get_next_msg, None):
    sm.enrich(msg)
    # all known tokens are now enriched; unknown tokens unchanged
```

> **Tip:** For bulk streaming use `attach_symbol_master()` instead (section 3.6) — it enriches every message automatically so you do not have to call `enrich()` in a loop.

---

## 5.6 `len()` and `repr()`

```python
print(len(sm))   # 95632
print(sm)        # SymbolMaster(contracts=95632)
```

---

# Recommended Workflows

## Workflow A: Large file — fastest approach

```python
from fastreader import FeedPathBuilder, StreamingBinaryLoader, OrderbookBuilder

b = FeedPathBuilder()
file_path = b.build_and_verify("NSE_FO", stream_id=1, day=21, month=5, year=2026)

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
processed = builder.build_from_source(reader, limit=500000)

tokens = builder.get_active_tokens()
print(f"Processed {processed} messages, {len(tokens)} tokens")
print(builder.get_snapshot(token=tokens[0], levels=5))
```

---

## Workflow B: Load once, analyse many times

```python
from fastreader import MessageCacheReader, OrderbookBuilder

reader = MessageCacheReader()
reader.load_to_cache("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")
print(reader.get_cache_summary())

builder = OrderbookBuilder()
builder.build_from_source(reader)

tokens = builder.get_active_tokens()
print(builder.get_snapshot(token=tokens[0], levels=5))
```

---

## Workflow C: Process messages one at a time

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()

while True:
    msg = reader.get_next_msg()
    if msg is None:
        break
    builder.orderbook_add_msg(msg)

tokens = builder.get_active_tokens()
print(builder.get_snapshot(token=tokens[0], levels=5))
```

---

## Workflow D: Orders only (no trades)

```python
reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
builder.apply_filter(["N", "M", "X"])

processed = builder.build_from_source(reader)
print("Order messages processed:", processed)
```

---

## Workflow F: Enrich streaming messages with symbol metadata

```python
from fastreader import FeedPathBuilder, StreamingBinaryLoader, OrderbookBuilder, SymbolMaster

# Load contract master once
sm = SymbolMaster()
sm.load_for_date("NSE_FO", day=21, month=5, year=2026)
print(sm)   # SymbolMaster(contracts=95632)

# Attach to the stream — all get_next_msg() calls will be auto-enriched
b = FeedPathBuilder()
file_path = b.build_and_verify("NSE_FO", stream_id=1, day=21, month=5, year=2026)

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)
reader.attach_symbol_master(sm)

# Also build the orderbook in the same pass
builder = OrderbookBuilder()

for _ in range(500_000):
    msg = reader.get_next_msg()
    if msg is None:
        break
    builder.orderbook_add_msg(msg)
    # msg already has: token_symbol, strike_price, option_type, expiry, lot_size, name

# Snapshot with symbol info
for token in builder.get_active_tokens()[:5]:
    snap = builder.get_snapshot(token=token, levels=3)
    info = sm.lookup(token=token)
    print(f"{info['symbol']:>12}  {info['name']:<25}  {info['option_type']}  "
          f"strike={info['strike']:>6}  bid={snap['best_bid']}  ask={snap['best_ask']}")
```

Expected output:

```
  011NSETEST  011NSETEST36DECFUT        XX  strike=     0  bid=None  ask=None
     FINNIFTY  FINNIFTY26MAY21700CE      CE  strike= 21700  bid=(322585, 1740)  ask=(494540, 120)
       NIFTY  NIFTY2660921350CE         CE  strike= 21350  bid=...  ask=...
```

---

## Workflow E: Export all tokens to CSV

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

reader = StreamingBinaryLoader()
reader.open_stream(file_path, count_messages=False)

builder = OrderbookBuilder()
builder.build_from_source(reader, limit=500000)

tokens = builder.get_active_tokens()

with open("snapshots.csv", "w") as f:
    f.write(builder.snapshot_header() + "\n")
    for token in tokens:
        f.write(builder.get_snapshot_row(token=token, levels=5) + "\n")

print(f"Wrote {len(tokens)} rows to snapshots.csv")
```

---

# Common Mistakes

### `found: False` or empty `bids` / `asks`

Token was never seen in processed messages, or all its orders were cancelled before you called the snapshot.

**Fix:** Use `get_active_tokens()` first, then pick a token from that list.

```python
tokens = builder.get_active_tokens()
print(builder.get_snapshot(token=tokens[0], levels=5))   # guaranteed to exist
```

If `bids` and `asks` are still empty even on a known token, you processed the full day and end-of-day cancellations cleared the book. Process fewer messages to capture mid-session state.

---

### Using a future date in `build_and_verify()`

```python
b.build_and_verify("NSE_FO", stream_id=1, day=1, month=6, year=2026)
# RuntimeError: file not found — the file doesn't exist yet
```

Use the actual latest available date. Files are written daily.

---

### `count_messages=True` on a large file is slow

```python
reader.open_stream(large_file, count_messages=True)   # full file scan before you can read
```

**Fix:**

```python
reader.open_stream(large_file, count_messages=False)   # opens immediately
```

---

# Error Reference

| Situation | Error message |
|---|---|
| File does not exist (`open_stream`) | `RuntimeError: No such file or directory` |
| File does not exist (`build_and_verify`) | `RuntimeError: file not found: /path/to/file.bin` |
| Unknown segment | `RuntimeError: unknown segment 'X' — expected one of: NSE_CM, CM, NSE_FO, FO` |
| `stream_id` is 0 | `RuntimeError: stream_id must be > 0` |
| Invalid month | `RuntimeError: invalid month 13 — must be 1–12` |
| `SymbolMaster.load()` — file not found | `RuntimeError: cannot open /path/to/file.csv: No such file or directory` |
| `SymbolMaster.load()` — column missing | `RuntimeError: column 'FinInstrmId' not found in /path/to/file.csv` |
| `SymbolMaster.enrich()` — not a dict | `TypeError: enrich() expects a message dict from get_next_msg()` |
| `SymbolMaster.enrich()` — no `token` key | `TypeError: msg dict missing 'token' key` |
| `load_for_date()` unknown segment | `RuntimeError: unknown segment 'X' — expected NSE_FO, FO, NSE_CM, or CM` |
| Invalid day | `RuntimeError: invalid day 0 — must be 1–31` |
| Invalid year | `RuntimeError: invalid year 1999 — must be 2000–2100` |
| Wrong object to `orderbook_add_msg` | `TypeError: orderbook_add_msg expects one message dict from get_next_msg()` |
| Wrong object to `build_from_source` | `TypeError: build_from_source expects MessageCacheReader or StreamingBinaryLoader` |

---

# Full API Reference

## `FeedPathBuilder`

| Method | Signature | Returns | Description |
|---|---|---|---|
| `build` | `(segment, stream_id, day, month, year, base_path=None)` | `str` | Construct path string |
| `build_and_verify` | `(segment, stream_id, day, month, year, base_path=None)` | `str` | Construct path and verify file exists |

## `MessageCacheReader`

| Method | Signature | Returns | Description |
|---|---|---|---|
| `load_to_cache` | `(file_path)` | `int` | Load binary file into RAM |
| `get_all_messages` | `()` | `list[str]` | All messages as formatted strings |
| `get_order_message` | `()` | `list[str]` | Order messages only |
| `get_trade_message` | `()` | `list[str]` | Trade messages only |
| `get_all_trade_message` | `()` | `list[str]` | Alias for `get_trade_message()` |
| `get_cache_summary` | `()` | `dict` | File stats and memory usage |

## `StreamingBinaryLoader`

| Method | Signature | Returns | Description |
|---|---|---|---|
| `open_stream` | `(file_path, count_messages=True)` | `int` | Open file for streaming |
| `get_next_message` | `()` | `str` | Next message as formatted string, or `"END"` |
| `get_next_msg` | `()` | `dict` or `None` | Next message as Python dict, or `None` at EOF |
| `reset_cursor` | `()` | `None` | Rewind stream to start of file |

## `OrderbookBuilder`

| Method | Signature | Returns | Description |
|---|---|---|---|
| `apply_filter` | `(logic_criteria=None)` | `None` | Filter message types to process |
| `build_from_source` | `(source, limit=None)` | `int` | Build from reader object (recommended) |
| `build_from_list` | `(source)` | `int` | Build from cache reader or list of dicts |
| `orderbook_add_msg` | `(msg)` | `bool` | Process one decoded message dict |
| `get_active_tokens` | `()` | `list[int]` | All token IDs seen during processing |
| `get_snapshot` | `(token, levels=None)` | `dict` | Top-N bid/ask levels for a token |
| `get_orderbook_snapshot` | `(token, levels=None)` | `dict` | Alias for `get_snapshot()` |
| `get_full_depth` | `(token)` | `dict` | All bid/ask levels for a token |
| `snapshot_header` | `()` | `str` | CSV column header |
| `get_snapshot_row` | `(token, levels=None)` | `str` | CSV data row for a token |
