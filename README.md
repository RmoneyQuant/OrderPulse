# fastreader / OrderPulse

High-performance Python bindings for reading NSE binary order/trade feed files, enriching messages with contract metadata, and building token-level order book snapshots.

This library is written in Rust for speed and exposed to Python using PyO3. It is designed for users who need to parse NSE CM/FO binary feed files, stream messages one by one, cache full files in memory, attach symbol metadata, and build order book depth from order and trade messages.

---

## 1. What this library does

`fastreader` helps you:

- Read NSE binary order and trade feed files.
- Stream messages one by one without loading the full file into memory.
- Load full files into an in-memory cache for repeated analysis.
- Convert binary messages into Python dictionaries.
- Read order messages and trade messages separately.
- Detect whether the stream has reached end-of-file.
- Enrich raw token values with symbol, strike price, option type, expiry, lot size, and contract name.
- Build an order book from decoded messages.
- Query best bid, best ask, spread, full depth, and top-level snapshot.
- Build standard NSE feed file paths programmatically.

---

## 2. Architecture overview

The library has five main user-facing classes:

| Class | Purpose |
|---|---|
| `MessageCacheReader` | Loads the complete binary feed file into memory and lets the user fetch all, order-only, or trade-only messages. |
| `StreamingBinaryLoader` | Opens a binary feed file and reads messages one by one. Best for large files. |
| `SymbolMaster` | Loads contract master CSV and maps token to symbol metadata. |
| `OrderbookBuilder` | Builds and queries an order book from cached or streamed messages. |
| `FeedPathBuilder` | Builds standard NSE feed file paths from segment, stream id, and date. |

Internally, Rust parses raw binary packets into `OrderPacket` and `TradePacket`, wraps them as `Message::Order` or `Message::Trade`, and exposes clean Python dictionaries for end users.

---

## 3. Example files used in this README

The examples below use these paths:

```python
FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"
CONTRACT_FILE = "/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv"
```

Practical note:

- A date folder can exist but still be empty on some systems.
- Always verify the contract CSV file exists before calling `sm.load(...)`.
- If your file uses standard naming (`NSE_FO_contract_DDMMYYYY.csv` or `NSE_CM_contract_DDMMYYYY.csv`), you can use `load_for_date(...)`.
- If your file has a custom name (for example `cm_contract_stream_info.csv`), use `load(...)` with the exact full path.

> Note: `SymbolMaster.load()` requires the CSV headers used by the Rust parser, including columns like `FinInstrmId`, `TckrSymb`, `XpryDt`, `StrkPric`, `OptnTp`, `StockNm`, and either `NewBrdLotQty` or `MinLot`.

---

## 4. Installation

After building or installing the Python extension, import it like this:

```python
from fastreader import (
    MessageCacheReader,
    StreamingBinaryLoader,
    OrderbookBuilder,
    SymbolMaster,
    FeedPathBuilder,
)
```

If you are developing locally with Rust and PyO3, a typical development installation is:

```bash
maturin develop --release
```

Then test the import:

```python
import fastreader
print(fastreader)
```

Expected output example:

```text
<module 'fastreader' from '.../fastreader...so'>
```

---

## 5. Message dictionary format

When a message is returned to Python, it is returned as a dictionary.

### Order message dictionary

```python
{
    "message_kind": "order",
    "seq_no": 123,
    "msg_len": 38,
    "stream_id": 2,
    "msg_type": "N",
    "exch_ts": 1767000000000000000,
    "local_ts": 1767000000000001000,
    "order_id": 987654321,
    "token": 12345,
    "order_type": "B",
    "price": 250000,
    "quantity": 75,
    "flags": False,
    "token_symbol": None,
    "strike_price": None,
    "option_type": None,
}
```

Important fields:

| Field | Meaning |
|---|---|
| `message_kind` | `"order"` for order messages. |
| `seq_no` | Stream sequence number. |
| `msg_type` | NSE message type. Usually `N`, `M`, or `X` for order messages. |
| `stream_id` | Stream id from binary feed header. |
| `order_id` | Exchange order id. |
| `token` | Instrument token. |
| `order_type` | Usually `B` for buy or `S` for sell. |
| `price` | Raw price value from feed. |
| `quantity` | Order quantity. |
| `flags` | Missed/flag status from packet. |
| `token_symbol`, `strike_price`, `option_type` | Populated after symbol enrichment. |

### Trade message dictionary

```python
{
    "message_kind": "trade",
    "seq_no": 124,
    "msg_len": 45,
    "stream_id": 2,
    "msg_type": "T",
    "exch_ts": 1767000000000000000,
    "local_ts": 1767000000000001000,
    "buy_order_id": 111111,
    "sell_order_id": 222222,
    "token": 12345,
    "trade_price": 250050,
    "trade_quantity": 50,
    "flags": False,
    "token_symbol": None,
    "strike_price": None,
    "option_type": None,
}
```

Important fields:

| Field | Meaning |
|---|---|
| `message_kind` | `"trade"` for trade messages. |
| `msg_type` | `T` for trade. |
| `buy_order_id` | Buy-side order id. |
| `sell_order_id` | Sell-side order id. |
| `token` | Instrument token. |
| `trade_price` | Executed trade price. |
| `trade_quantity` | Executed trade quantity. |

---

# 6. `MessageCacheReader`

`MessageCacheReader` loads the full binary file into memory. Use this when the file is not too large or when you want to query the same file multiple times.

## 6.1 Create reader

```python
from fastreader import MessageCacheReader

reader = MessageCacheReader()
print(reader)
```

Expected output:

```text
<fastreader.MessageCacheReader object at ...>
```

---

## 6.2 `load_to_cache(file_path)`

Loads all supported order and trade messages into memory.

```python
from fastreader import MessageCacheReader

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

reader = MessageCacheReader()
count = reader.load_to_cache(FEED_FILE)

print("Loaded messages:", count)
```

Expected output example:

```text
Loaded messages: 2500000
```

The exact number depends on your binary feed file.

Working:

- Opens the binary feed file.
- Reads only order and trade packets.
- Stores parsed messages inside Rust memory.
- Returns the total number of loaded messages.

---

## 6.3 `get_all_messages()`

Returns all cached messages as readable strings.

```python
messages = reader.get_all_messages()

print("Total:", len(messages))
print(messages[0])
```

Expected output example for an order message:

```text
Total: 2500000
Order Message: SeqNo 42, MsgLen 10, MsgType 'N', ExchTs 100000, LocalTs 200000, OrderId 55, Token 1001, Side 'B', Price 500, Quantity 100, Missed 0
```

Expected output example for a trade message:

```text
Trade Message: SeqNo 99, MsgLen 10, MsgType 'T', ExchTs 300000, LocalTs 400000, BuyOrderId 10, SellOrderId 20, Token 5000, Price 750, Quantity 30, Missed 1
```

Working:

- Converts every cached Rust message into a formatted string.
- Useful for quick debugging and printing.
- Not ideal for structured analysis because it returns strings, not dictionaries.

---

## 6.4 `get_order_message()`

Returns only order messages as Python dictionaries.

```python
orders = reader.get_order_message()

print("Order count:", len(orders))
print(orders[0])
```

Expected output example:

```text
Order count: 1800000
{
    'message_kind': 'order',
    'seq_no': 42,
    'msg_len': 10,
    'stream_id': 2,
    'msg_type': 'N',
    'exch_ts': 100000,
    'local_ts': 200000,
    'order_id': 55,
    'token': 1001,
    'order_type': 'B',
    'price': 500,
    'quantity': 100,
    'flags': False,
    'token_symbol': None,
    'strike_price': None,
    'option_type': None
}
```

Working:

- Filters cached messages where `message_kind` is order.
- Converts each order packet into a Python dictionary.
- Good for pandas conversion and downstream analysis.

Example with pandas:

```python
import pandas as pd

orders_df = pd.DataFrame(reader.get_order_message())
print(orders_df.head())
```

Expected output example:

```text
  message_kind  seq_no  msg_len  stream_id msg_type  ...  quantity  flags token_symbol strike_price option_type
0        order      42       10          2        N  ...       100  False         None         None        None
```

---

## 6.5 `get_trade_message()`

Returns only trade messages as Python dictionaries.

```python
trades = reader.get_trade_message()

print("Trade count:", len(trades))
print(trades[0])
```

Expected output example:

```text
Trade count: 700000
{
    'message_kind': 'trade',
    'seq_no': 99,
    'msg_len': 10,
    'stream_id': 2,
    'msg_type': 'T',
    'exch_ts': 300000,
    'local_ts': 400000,
    'buy_order_id': 10,
    'sell_order_id': 20,
    'token': 5000,
    'trade_price': 750,
    'trade_quantity': 30,
    'flags': True,
    'token_symbol': None,
    'strike_price': None,
    'option_type': None
}
```

Working:

- Filters cached messages where `message_kind` is trade.
- Converts each trade packet into a Python dictionary.

---

## 6.6 `get_all_trade_message()`

Alias for `get_trade_message()`.

```python
trades_1 = reader.get_trade_message()
trades_2 = reader.get_all_trade_message()

print(len(trades_1) == len(trades_2))
```

Expected output:

```text
True
```

Working:

- Internally calls the same trade-message extraction logic.
- Kept for user convenience.

---

## 6.7 `get_cache_summary()`

Returns summary information about the loaded cache.

```python
summary = reader.get_cache_summary()
print(summary)
```

Expected output example:

```python
{
    'file_source': '/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin',
    'total_messages': 2500000,
    'total_orders': 1800000,
    'total_trades': 700000,
    'memory_usage_bytes': 120000000
}
```

Working:

- Counts total cached messages.
- Counts order messages.
- Counts trade messages.
- Estimates Rust-side memory usage.

---

# 7. `StreamingBinaryLoader`

`StreamingBinaryLoader` reads messages one by one from the binary file. Use this for large files where loading everything into memory is not ideal.

## 7.1 Create stream loader

```python
from fastreader import StreamingBinaryLoader

loader = StreamingBinaryLoader()
```

---

## 7.2 `open_stream(file_path, count_messages=True)`

Opens a binary file for streaming.

```python
from fastreader import StreamingBinaryLoader

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

loader = StreamingBinaryLoader()
total = loader.open_stream(FEED_FILE, count_messages=True)

print("Total messages:", total)
```

Expected output example:

```text
Total messages: 2500000
```

For faster opening on very large files, skip counting:

```python
loader = StreamingBinaryLoader()
total = loader.open_stream(FEED_FILE, count_messages=False)

print(total)
```

Expected output:

```text
0
```

Working:

- Opens the binary file.
- Validates the first message header.
- Optionally scans the full file to count messages.
- Sets the stream cursor to the beginning.

---

## 7.3 `get_next_msg()`

Reads the next message from the stream and returns a Python dictionary. Returns `None` when end-of-file is reached.

```python
msg = loader.get_next_msg()
print(msg)
```

Expected output example:

```python
{
    'message_kind': 'order',
    'seq_no': 42,
    'msg_len': 10,
    'stream_id': 2,
    'msg_type': 'N',
    'exch_ts': 100000,
    'local_ts': 200000,
    'order_id': 55,
    'token': 1001,
    'order_type': 'B',
    'price': 500,
    'quantity': 100,
    'flags': False,
    'token_symbol': None,
    'strike_price': None,
    'option_type': None
}
```

Read first five messages:

```python
for i in range(5):
    msg = loader.get_next_msg()
    if msg is None:
        print("End of file")
        break
    print(i, msg)
```

Expected output example:

```text
0 {'message_kind': 'order', 'seq_no': 1, ...}
1 {'message_kind': 'order', 'seq_no': 2, ...}
2 {'message_kind': 'trade', 'seq_no': 3, ...}
3 {'message_kind': 'order', 'seq_no': 4, ...}
4 {'message_kind': 'trade', 'seq_no': 5, ...}
```

Working:

- Reads one binary packet from the current file cursor.
- Converts it into a Python dictionary.
- Advances the cursor to the next message.
- Returns `None` after the final message.

---

## 7.4 `is_end_of_msg()`

Checks whether the stream is at the end of messages.

```python
while not loader.is_end_of_msg():
    msg = loader.get_next_msg()
    print(msg)

print("Completed")
```

Expected output example:

```text
{'message_kind': 'order', 'seq_no': 1, ...}
{'message_kind': 'trade', 'seq_no': 2, ...}
Completed
```

Working:

- Looks ahead to check whether another message exists.
- Restores the file cursor to its original position.
- Does not consume or skip any message.
- Returns `True` when no more messages are available.

Recommended safe streaming pattern:

```python
loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)

while True:
    msg = loader.get_next_msg()
    if msg is None:
        break

    # process message here
    print(msg["message_kind"], msg["token"])
```

---

## 7.5 `reset_cursor()`

Moves the stream cursor back to the beginning of the file.

```python
first_msg = loader.get_next_msg()
print("First read:", first_msg)

loader.reset_cursor()

again_first_msg = loader.get_next_msg()
print("After reset:", again_first_msg)
```

Expected output example:

```text
First read: {'message_kind': 'order', 'seq_no': 1, ...}
After reset: {'message_kind': 'order', 'seq_no': 1, ...}
```

Working:

- Seeks the file cursor to byte position `0`.
- Lets the user re-read the same file from the beginning.

---

## 7.6 `attach_symbol_master(master)`

Attaches a loaded `SymbolMaster` to the stream loader. After this, every `get_next_msg()` call automatically fills symbol metadata.

```python
from fastreader import StreamingBinaryLoader, SymbolMaster

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"
CONTRACT_FILE = "/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv"

sm = SymbolMaster()
loaded = sm.load(CONTRACT_FILE)
print("Contracts loaded:", loaded)

loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)
loader.attach_symbol_master(sm)

msg = loader.get_next_msg()
print(msg)
```

Expected output example:

```python
{
    'message_kind': 'order',
    'seq_no': 42,
    'stream_id': 2,
    'msg_type': 'N',
    'token': 12345,
    'token_symbol': 'RELIANCE',
    'strike_price': -1,
    'option_type': 'XX',
    'expiry': '29-Dec-2025',
    'lot_size': 1,
    'name': 'RELIANCE'
}
```

Working:

- Stores the token-to-contract map inside the stream loader.
- Every streamed message is enriched automatically when its token exists in the symbol master.

---

## 7.7 `detach_symbol_master()`

Removes the attached symbol master.

```python
loader.detach_symbol_master()
msg = loader.get_next_msg()
print(msg["token_symbol"], msg["strike_price"], msg["option_type"])
```

Expected output:

```text
None None None
```

Working:

- Clears the attached metadata map.
- Future streamed messages return raw token fields only.

---

# 8. `SymbolMaster`

`SymbolMaster` loads the contract master CSV and provides fast token lookup and message enrichment.

## 8.1 Create symbol master

```python
from fastreader import SymbolMaster

sm = SymbolMaster()
print(sm)
print(len(sm))
```

Expected output:

```text
SymbolMaster(contracts=0)
0
```

---

## 8.2 `load(csv_path)`

Loads contract metadata from an explicit CSV file path.

```python
CONTRACT_FILE = "/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv"

sm = SymbolMaster()
count = sm.load(CONTRACT_FILE)

print("Loaded contracts:", count)
print(sm)
print("Length:", len(sm))
```

Safer path-discovery example (recommended for notebook use):

```python
from pathlib import Path

sm = SymbolMaster()

# 1) Try your exact expected path first
candidates = [
    Path("/nas/50.30/CONTRACT/27_05_2026/NSE_FO_contract_27052026.csv"),
    Path("/home/pratima/CONTRACT/27_05_2026/NSE_FO_contract_27052026.csv"),
    Path.cwd() / "NSE_FO_contract_27052026.csv",
]

csv_path = next((str(p) for p in candidates if p.exists()), None)

# 2) Fallback: pick latest available FO contract CSV from NAS
if csv_path is None:
    fo_files = sorted(Path("/nas/50.30/CONTRACT").glob("*/NSE_FO_contract_*.csv"))
    if fo_files:
        csv_path = str(fo_files[-1])
        print("Using latest available FO CSV:", csv_path)
    else:
        print("No NSE_FO contract CSV found under /nas/50.30/CONTRACT")

if csv_path is not None:
    count = sm.load(csv_path)
    print("Loaded contracts:", count)
```

Expected output example:

```text
Loaded contracts: 50000
SymbolMaster(contracts=50000)
Length: 50000
```

Working:

- Opens the CSV file.
- Reads required columns:
  - `FinInstrmId`
  - `TckrSymb`
  - `XpryDt`
  - `StrkPric`
  - `OptnTp`
  - `StockNm`
  - `NewBrdLotQty` or `MinLot`
- Converts expiry Unix timestamp into readable date.
- Converts strike price by dividing raw strike by `100`.
- Stores mapping as `token -> contract metadata`.

---

## 8.3 `load_for_date(segment, day, month, year, base_path=None)`

Builds the standard contract master path automatically and loads it.

```python
sm = SymbolMaster()
count = sm.load_for_date("NSE_CM", day=10, month=10, year=2025, base_path="/nas/50.30")

print(count)
```

Expected path internally:

```text
/nas/50.30/CONTRACT/10_10_2025/NSE_CM_contract_10102025.csv
```

Expected output example:

```text
50000
```

Supported segment values:

```text
NSE_FO, FO, NSE_CM, CM
```

Working:

- Normalizes segment to `FO` or `CM`.
- Builds path using date and base path.
- Calls `load()` internally.

Important note for your provided CSV path:

```python
CONTRACT_FILE = "/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv"
sm.load(CONTRACT_FILE)
```

Use `load()` when your CSV filename is custom. Use `load_for_date()` only when your filename follows the library pattern:

```text
/nas/50.30/CONTRACT/DD_MM_YYYY/NSE_CM_contract_DDMMYYYY.csv
```

---

## 8.4 `lookup(token)`

Looks up one token and returns contract metadata.

```python
info = sm.lookup(12345)
print(info)
```

Expected output example when token is found:

```python
{
    'token': 12345,
    'found': True,
    'symbol': 'RELIANCE',
    'name': 'RELIANCE',
    'option_type': 'XX',
    'strike': -1,
    'expiry': '29-Dec-2025',
    'lot_size': 1
}
```

Expected output when token is not found:

```python
{
    'token': 999999999,
    'found': False,
    'symbol': None,
    'name': None,
    'option_type': None,
    'strike': None,
    'expiry': None,
    'lot_size': None
}
```

Working:

- Searches the loaded Rust hash map by token.
- Returns `found=True` and metadata if available.
- Returns `found=False` and `None` fields if unavailable.

---

## 8.5 `enrich(msg)`

Enriches a message dictionary in place.

```python
loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)

msg = loader.get_next_msg()
print("Before:", msg)

found = sm.enrich(msg)
print("Found:", found)
print("After:", msg)
```

Expected output example:

```text
Before: {'message_kind': 'order', 'token': 12345, 'token_symbol': None, 'strike_price': None, 'option_type': None, ...}
Found: True
After: {'message_kind': 'order', 'token': 12345, 'token_symbol': 'RELIANCE', 'strike_price': -1, 'option_type': 'XX', 'expiry': '29-Dec-2025', 'lot_size': 1, 'name': 'RELIANCE', ...}
```

Expected output if token is not present in contract master:

```text
Found: False
```

Working:

- Reads the `token` field from the message dictionary.
- Looks up token in loaded symbol master.
- Adds/updates:
  - `token_symbol`
  - `strike_price`
  - `option_type`
  - `expiry`
  - `lot_size`
  - `name`
- Returns `True` if enrichment happened.

---

# 9. `OrderbookBuilder`

`OrderbookBuilder` builds and queries order book depth from order/trade messages.

## 9.1 Create builder

```python
from fastreader import OrderbookBuilder

builder = OrderbookBuilder()
print(builder)
```

Expected output:

```text
<fastreader.OrderbookBuilder object at ...>
```

---

## 9.2 `apply_filter(logic_criteria=None)`

Filters which message types are processed.

```python
builder = OrderbookBuilder()
builder.apply_filter(["N", "M", "X"])
```

This means:

- Process new order messages: `N`
- Process modify order messages: `M`
- Process cancel/delete messages: `X`
- Skip trade messages: `T`

To clear the filter:

```python
builder.apply_filter(None)
```

Expected output:

```text
No direct output. Filter is applied internally.
```

Working:

- Stores allowed message types as bytes.
- During order book building, unsupported message types are skipped.

---

## 9.3 `orderbook_add_msg(msg)`

Adds one decoded message dictionary into the order book.

```python
loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)

builder = OrderbookBuilder()

msg = loader.get_next_msg()
accepted = builder.orderbook_add_msg(msg)

print("Accepted:", accepted)
```

Expected output example:

```text
Accepted: True
```

Expected output when message is skipped by filter or business rule:

```text
Accepted: False
```

Working:

- Expects one dictionary returned by `get_next_msg()`.
- Converts the Python dictionary back into Rust message format.
- Applies the order/trade update to the order book manager.
- Returns whether the message was accepted and processed.

Common error:

```python
builder.orderbook_add_msg("wrong input")
```

Expected error:

```text
TypeError: orderbook_add_msg expects one message dict from get_next_msg()
```

---

## 9.4 `build_from_list(source)`

Builds the order book from either:

1. A `MessageCacheReader`, or
2. A Python `list[dict]` of decoded messages.

### Example A: build from cache reader

```python
reader = MessageCacheReader()
reader.load_to_cache(FEED_FILE)

builder = OrderbookBuilder()
processed = builder.build_from_list(reader)

print("Processed:", processed)
```

Expected output example:

```text
Processed: 2400000
```

### Example B: build from list of dictionaries

```python
reader = MessageCacheReader()
reader.load_to_cache(FEED_FILE)

orders = reader.get_order_message()

builder = OrderbookBuilder()
processed = builder.build_from_list(orders)

print("Processed:", processed)
```

Expected output example:

```text
Processed: 1800000
```

Working:

- If source is `MessageCacheReader`, it uses internal cached Rust messages directly.
- If source is `list[dict]`, it converts each dictionary into Rust message format.
- Applies each message to the order book.
- Returns count of processed messages.

---

## 9.5 `build_from_source(source, limit=None)`

Builds the order book from either a `MessageCacheReader` or a `StreamingBinaryLoader`.

### Build from cache reader

```python
reader = MessageCacheReader()
reader.load_to_cache(FEED_FILE)

builder = OrderbookBuilder()
processed = builder.build_from_source(reader)

print(processed)
```

Expected output example:

```text
2400000
```

### Build from streaming loader with limit

```python
loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)

builder = OrderbookBuilder()
processed = builder.build_from_source(loader, limit=10000)

print("Processed first messages:", processed)
```

Expected output example:

```text
Processed first messages: 10000
```

Working:

- If source is a cache reader, delegates to `build_from_list()`.
- If source is a streaming loader, reads from the current stream cursor.
- Stops at end-of-file or after `limit` accepted messages.

Important:

```python
processed = builder.build_from_source(loader, limit=10000)
```

The `limit` counts accepted/processed messages, not necessarily raw messages read if filters skip some messages.

---

## 9.6 `get_active_tokens()`

Returns all tokens currently present in the order book.

```python
active_tokens = builder.get_active_tokens()

print("Number of active tokens:", len(active_tokens))
print(active_tokens[:10])
```

Expected output example:

```text
Number of active tokens: 1250
[12345, 12346, 12347, 2885, 3045, 11536, 1333, 4963, 1594, 1660]
```

Working:

- Queries the order book manager.
- Returns tokens for which depth/order book state exists.

---

## 9.7 `get_snapshot(token, levels=None)`

Returns top order book levels for a token.

```python
token = builder.get_active_tokens()[0]
snapshot = builder.get_snapshot(token, levels=5)

print(snapshot)
```

Expected output example when token is found:

```python
{
    'token': 12345,
    'found': True,
    'mid_price': 250025,
    'best_bid': (250000, 100),
    'best_ask': (250050, 150),
    'spread': 50,
    'bids': [(250000, 100), (249950, 75), (249900, 50)],
    'asks': [(250050, 150), (250100, 25), (250150, 10)]
}
```

Expected output when token is not found:

```python
{
    'token': 999999999,
    'found': False,
    'mid_price': 0,
    'best_bid': None,
    'best_ask': None,
    'spread': None,
    'bids': [],
    'asks': []
}
```

Working:

- Reads top bid and ask levels from the order book.
- Calculates:
  - `best_bid`
  - `best_ask`
  - `spread = best_ask_price - best_bid_price`
  - `mid_price`
- Defaults to 5 levels if `levels` is not provided.

---

## 9.8 `get_full_depth(token)`

Returns full available depth for a token.

```python
full_depth = builder.get_full_depth(token)
print(full_depth)
```

Expected output example:

```python
{
    'token': 12345,
    'found': True,
    'best_bid': (250000, 100),
    'best_ask': (250050, 150),
    'spread': 50,
    'bids': [(250000, 100), (249950, 75), (249900, 50), ...],
    'asks': [(250050, 150), (250100, 25), (250150, 10), ...]
}
```

Expected output when token is not found:

```python
{
    'token': 999999999,
    'found': False,
    'best_bid': None,
    'best_ask': None,
    'spread': None,
    'bids': [],
    'asks': []
}
```

Working:

- Returns all available bid and ask levels for the token.
- Best for detailed order book inspection.
- Use `get_snapshot()` for faster top-N view.

---

## 9.9 `snapshot_header()`

Returns CSV header for snapshot rows.

```python
header = builder.snapshot_header()
print(header)
```

Expected output:

```text
local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,ask_qty_1,bid_price_2,bid_qty_2,ask_price_2,ask_qty_2,bid_price_3,bid_qty_3,ask_price_3,ask_qty_3,bid_price_4,bid_qty_4,ask_price_4,ask_qty_4
```

Working:

- Returns a fixed CSV header for 5-level snapshot output.

---

## 9.10 `get_snapshot_row(token, levels=None)`

Returns one token snapshot as a CSV row string.

```python
row = builder.get_snapshot_row(token, levels=5)
print(builder.snapshot_header())
print(row)
```

Expected output example:

```text
local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,bid_qty_1,...
0,0,250025,250000,100,250050,150,249950,75,250100,25,249900,50,250150,10,0,0,0,0,0,0,0,0
```

Working:

- Fetches top levels from the order book.
- Pads missing bid/ask levels with `0,0`.
- Returns a CSV-compatible row.

Save snapshots to CSV:

```python
with open("snapshots.csv", "w") as f:
    f.write(builder.snapshot_header() + "\n")
    for token in builder.get_active_tokens():
        f.write(builder.get_snapshot_row(token, levels=5) + "\n")
```

---

# 10. `FeedPathBuilder`

`FeedPathBuilder` creates standard NSE feed file paths.

## 10.1 Create path builder

```python
from fastreader import FeedPathBuilder

path_builder = FeedPathBuilder()
print(path_builder)
```

Expected output:

```text
FeedPathBuilder()
```

---

## 10.2 `build(segment, stream_id, day, month, year, base_path=None)`

Builds a feed file path.

```python
builder = FeedPathBuilder()

path = builder.build(
    "NSE_CM",
    stream_id=2,
    day=29,
    month=12,
    year=2025,
)

print(path)
```

Expected output:

```text
/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
```

With custom base path:

```python
path = builder.build(
    "NSE_CM",
    stream_id=2,
    day=29,
    month=12,
    year=2025,
    base_path="/nas/50.30",
)

print(path)
```

Expected output:

```text
/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
```

Working:

- Accepts segment values such as `NSE_CM`, `CM`, `NSE_FO`, or `FO`.
- Validates stream id and date values.
- Returns the standard feed file path string.

---

## 10.3 `build_and_verify(segment, stream_id, day, month, year, base_path=None)`

Builds the path and checks whether the file exists.

```python
builder = FeedPathBuilder()

path = builder.build_and_verify(
    "NSE_CM",
    stream_id=2,
    day=29,
    month=12,
    year=2025,
    base_path="/nas/50.30",
)

print(path)
```

Expected output if file exists:

```text
/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
```

Expected error if file does not exist:

```text
RuntimeError: file does not exist: /nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
```

Working:

- Builds the same path as `build()`.
- Performs disk existence check.
- Raises `RuntimeError` if file is missing.

---

# 11. Complete workflow examples

## 11.1 Fast cached workflow

Use this when memory is sufficient and you want repeated access.

```python
from fastreader import MessageCacheReader, OrderbookBuilder

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

reader = MessageCacheReader()
count = reader.load_to_cache(FEED_FILE)
print("Loaded:", count)

summary = reader.get_cache_summary()
print(summary)

builder = OrderbookBuilder()
processed = builder.build_from_list(reader)
print("Processed into orderbook:", processed)

active_tokens = builder.get_active_tokens()
print("Active tokens:", len(active_tokens))

if active_tokens:
    token = active_tokens[0]
    print("Snapshot:", builder.get_snapshot(token, levels=5))
    print("Full depth:", builder.get_full_depth(token))
```

Expected output example:

```text
Loaded: 2500000
{'file_source': '/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin', 'total_messages': 2500000, 'total_orders': 1800000, 'total_trades': 700000, 'memory_usage_bytes': 120000000}
Processed into orderbook: 2400000
Active tokens: 1250
Snapshot: {'token': 12345, 'found': True, 'mid_price': 250025, ...}
Full depth: {'token': 12345, 'found': True, 'best_bid': (250000, 100), ...}
```

---

## 11.2 Memory-friendly streaming workflow

Use this for very large files.

```python
from fastreader import StreamingBinaryLoader, OrderbookBuilder

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)

builder = OrderbookBuilder()
processed = builder.build_from_source(loader, limit=100000)

print("Processed:", processed)
print("Active tokens:", len(builder.get_active_tokens()))
```

Expected output example:

```text
Processed: 100000
Active tokens: 450
```

---

## 11.3 Streaming with automatic symbol enrichment

```python
from fastreader import StreamingBinaryLoader, SymbolMaster

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"
CONTRACT_FILE = "/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv"

sm = SymbolMaster()
sm.load(CONTRACT_FILE)

loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)
loader.attach_symbol_master(sm)

for _ in range(10):
    msg = loader.get_next_msg()
    if msg is None:
        break

    print(
        msg["message_kind"],
        msg["token"],
        msg.get("token_symbol"),
        msg.get("strike_price"),
        msg.get("option_type"),
    )
```

Expected output example:

```text
order 12345 RELIANCE -1 XX
order 2885 HDFCBANK -1 XX
trade 3045 SBIN -1 XX
```

---

## 11.4 Manual enrichment of cached/streamed messages

```python
from fastreader import StreamingBinaryLoader, SymbolMaster

sm = SymbolMaster()
sm.load("/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv")

loader = StreamingBinaryLoader()
loader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin", count_messages=False)

msg = loader.get_next_msg()
found = sm.enrich(msg)

print("Enriched:", found)
print(msg)
```

Expected output example:

```text
Enriched: True
{'message_kind': 'order', 'token': 12345, 'token_symbol': 'RELIANCE', 'strike_price': -1, 'option_type': 'XX', 'expiry': '29-Dec-2025', 'lot_size': 1, 'name': 'RELIANCE', ...}
```

---

## 11.5 Build path and open stream

```python
from fastreader import FeedPathBuilder, StreamingBinaryLoader

path_builder = FeedPathBuilder()

feed_path = path_builder.build(
    "NSE_CM",
    stream_id=2,
    day=29,
    month=12,
    year=2025,
    base_path="/nas/50.30",
)

print(feed_path)

loader = StreamingBinaryLoader()
loader.open_stream(feed_path, count_messages=False)

msg = loader.get_next_msg()
print(msg)
```

Expected output:

```text
/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
{'message_kind': 'order', 'seq_no': 1, ...}
```

---

## 11.6 Export top-5 snapshots to CSV

```python
from fastreader import MessageCacheReader, OrderbookBuilder

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

reader = MessageCacheReader()
reader.load_to_cache(FEED_FILE)

builder = OrderbookBuilder()
builder.build_from_list(reader)

with open("orderbook_snapshots.csv", "w") as f:
    f.write(builder.snapshot_header() + "\n")

    for token in builder.get_active_tokens():
        f.write(builder.get_snapshot_row(token, levels=5) + "\n")

print("Saved orderbook_snapshots.csv")
```

Expected output:

```text
Saved orderbook_snapshots.csv
```

---

# 12. Error handling examples

## 12.1 Missing binary file

```python
reader = MessageCacheReader()
reader.load_to_cache("/wrong/path/file.bin")
```

Expected error:

```text
RuntimeError: No such file or directory
```

---

## 12.2 Missing contract CSV

```python
sm = SymbolMaster()
sm.load("/wrong/path/contracts.csv")
```

Expected error:

```text
RuntimeError: cannot open /wrong/path/contracts.csv: No such file or directory
```

Checklist:

- Confirm file exists with `Path(path).exists()`.
- If using `load_for_date(...)`, confirm the generated filename follows `NSE_{FO|CM}_contract_DDMMYYYY.csv`.
- If your CSV has a custom filename, switch to `sm.load(full_custom_path)`.

---

## 12.2A SymbolMaster object not created

```python
loaded = sm.load("/some/path/NSE_FO_contract_27052026.csv")
```

Expected error:

```text
NameError: name 'sm' is not defined
```

Fix:

```python
from fastreader import SymbolMaster

sm = SymbolMaster()
loaded = sm.load("/some/path/NSE_FO_contract_27052026.csv")
print(loaded)
```

---

## 12.3 Wrong input to order book

```python
builder = OrderbookBuilder()
builder.orderbook_add_msg(["not", "a", "dict"])
```

Expected error:

```text
TypeError: orderbook_add_msg expects one message dict from get_next_msg()
```

---

## 12.4 Unsupported message type in dictionary

```python
builder = OrderbookBuilder()
builder.build_from_list([{"msg_type": "Z"}])
```

Expected error:

```text
TypeError: unsupported msg_type: Z
```

---

# 13. Best practices

## Use `MessageCacheReader` when:

- File size is manageable.
- You need to repeatedly access messages.
- You want to convert orders/trades into pandas DataFrames.
- You want fast order book building from memory.

## Use `StreamingBinaryLoader` when:

- File is very large.
- You want low memory usage.
- You want to process messages one by one.
- You want to stop after a fixed limit.

## Use `SymbolMaster` when:

- You need token-to-symbol mapping.
- You want `token_symbol`, `strike_price`, `option_type`, `expiry`, `lot_size`, and `name` in messages.
- You want cleaner outputs for library users.

## Use `OrderbookBuilder` when:

- You need best bid/best ask.
- You need spread and mid price.
- You need top-5 or full depth by token.
- You need CSV-style snapshot rows.

## Use `FeedPathBuilder` when:

- Your files follow the standard `/nas/50.30` feed path pattern.
- You want to avoid hardcoding feed paths.
- You want path validation before opening files.

---

# 14. Full end-to-end example

```python
from fastreader import (
    FeedPathBuilder,
    StreamingBinaryLoader,
    SymbolMaster,
    OrderbookBuilder,
)

# 1. Build feed path
path_builder = FeedPathBuilder()
feed_path = path_builder.build(
    "NSE_CM",
    stream_id=2,
    day=29,
    month=12,
    year=2025,
    base_path="/nas/50.30",
)

# 2. Load contract metadata
contract_path = "/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv"
sm = SymbolMaster()
contract_count = sm.load(contract_path)
print("Contracts loaded:", contract_count)

# 3. Open binary stream
loader = StreamingBinaryLoader()
loader.open_stream(feed_path, count_messages=False)
loader.attach_symbol_master(sm)

# 4. Build orderbook from first 100000 accepted messages
builder = OrderbookBuilder()
builder.apply_filter(["N", "M", "X", "T"])
processed = builder.build_from_source(loader, limit=100000)
print("Processed messages:", processed)

# 5. Query snapshots
active_tokens = builder.get_active_tokens()
print("Active tokens:", len(active_tokens))

for token in active_tokens[:5]:
    info = sm.lookup(token)
    snapshot = builder.get_snapshot(token, levels=5)

    print("Token info:", info)
    print("Snapshot:", snapshot)
```

Expected output example:

```text
Contracts loaded: 50000
Processed messages: 100000
Active tokens: 450
Token info: {'token': 12345, 'found': True, 'symbol': 'RELIANCE', 'name': 'RELIANCE', 'option_type': 'XX', 'strike': -1, 'expiry': '29-Dec-2025', 'lot_size': 1}
Snapshot: {'token': 12345, 'found': True, 'mid_price': 250025, 'best_bid': (250000, 100), 'best_ask': (250050, 150), 'spread': 50, 'bids': [...], 'asks': [...]}
```

---

# 15. Developer notes

- Binary message parsing is handled in Rust for performance.
- Python receives normal dictionaries, lists, strings, and integers.
- `get_next_msg()` returns `None` at end-of-file.
- `is_end_of_msg()` checks EOF without advancing the cursor.
- Symbol enrichment is optional.
- `attach_symbol_master()` is best for streaming workflows.
- `SymbolMaster.enrich()` is useful when you already have a message dictionary.
- `build_from_source()` is the most flexible order book API because it accepts both cached and streaming sources.
- `get_snapshot()` is best for application display.
- `get_snapshot_row()` and `snapshot_header()` are best for CSV export.

---

# 16. Minimal quick-start

```python
from fastreader import StreamingBinaryLoader, SymbolMaster, OrderbookBuilder

FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"
CONTRACT_FILE = "/nas/50.30/CONTRACT/10_10_2025/cm_contract_stream_info.csv"

sm = SymbolMaster()
sm.load(CONTRACT_FILE)

loader = StreamingBinaryLoader()
loader.open_stream(FEED_FILE, count_messages=False)
loader.attach_symbol_master(sm)

builder = OrderbookBuilder()
builder.build_from_source(loader, limit=100000)

for token in builder.get_active_tokens()[:10]:
    print(sm.lookup(token))
    print(builder.get_snapshot(token, levels=5))
```

This is the recommended starting point for most library users.
