"""
Type stubs for the `fastreader` / `orderpulse` PyO3 extension module.

This file documents the public Python API exposed from `lib.rs`.
It is meant to be copied into your Python package as:

    fastreader/__init__.pyi

or, if your wheel/package name is `orderpulse`, as:

    orderpulse/__init__.pyi

Why this file is useful
-----------------------
A `.pyi` file gives Python users and IDEs strong hints about your Rust-backed
extension module. It improves autocomplete, function signature help, static
checking, and user onboarding without changing runtime behavior.

Architecture overview
---------------------
The Rust module exposes three main Python classes:

1. `MessageCacheReader`
   RAM-based reader. It loads the whole binary feed into memory first.
   Use this when the file is small enough to fit in memory and you want to
   repeatedly inspect messages, filter orders/trades, or build the book many
   times without rereading the disk.

2. `StreamingBinaryLoader`
   Disk-streaming reader. It keeps only a file handle and reads messages one
   by one. Use this for large NSE binary feed files where loading everything
   into RAM is expensive.

3. `OrderbookBuilder`
   Orderbook engine. It consumes messages from either `MessageCacheReader`,
   `StreamingBinaryLoader`, or a Python `list[dict]`, applies optional message
   filters, updates bid/ask book state, and exposes snapshots.

Message model
-------------
The binary feed is parsed internally into either order messages or trade
messages.

Supported order message types:
    - "N": new order
    - "M": modify order
    - "X": cancel/delete order

Supported trade message type:
    - "T": trade

Order side / order_type values:
    - "B": buy/bid side
    - "S": sell/ask side

Price and quantity units
------------------------
The Rust code keeps numeric fields exactly as encoded in the binary feed.
For NSE-style feeds, prices are often stored as integer ticks/paise rather
than floating rupees. Convert externally only if your data dictionary says so.

Typical workflow
----------------
Streaming workflow for very large files:

    from fastreader import StreamingBinaryLoader, OrderbookBuilder

    reader = StreamingBinaryLoader()
    reader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin",
                       count_messages=False)

    builder = OrderbookBuilder()
    processed = builder.build_from_source(reader, limit=100_000)

    print(processed)
    print(builder.get_snapshot(token=1001, levels=5))

Cache workflow for repeated analysis:

    from fastreader import MessageCacheReader, OrderbookBuilder

    cache = MessageCacheReader()
    total = cache.load_to_cache("/path/to/feed.bin")

    print(cache.get_cache_summary())

    builder = OrderbookBuilder()
    processed = builder.build_from_list(cache)
    print(builder.get_full_depth(token=1001))
"""

from __future__ import annotations

from typing import Any, Dict, Literal, Optional, Sequence, TypedDict, TypeAlias, overload


UInt32: TypeAlias = int
UInt64: TypeAlias = int
Price: TypeAlias = int
Quantity: TypeAlias = int
Token: TypeAlias = int
OrderId: TypeAlias = int
Timestamp: TypeAlias = int
PriceLevel: TypeAlias = tuple[Price, Quantity]
MessageType: TypeAlias = Literal["N", "M", "X", "T"]
OrderMessageType: TypeAlias = Literal["N", "M", "X"]
TradeMessageType: TypeAlias = Literal["T"]
OrderSide: TypeAlias = Literal["B", "S"]


class CacheSummary(TypedDict):
    """
    Summary returned by `MessageCacheReader.get_cache_summary()`.

    Keys
    ----
    file_source:
        Source binary file path that was loaded into cache. It is `None`
        before `load_to_cache()` is called.
    total_messages:
        Total parsed order + trade messages held in RAM.
    total_orders:
        Count of parsed order messages.
    total_trades:
        Count of parsed trade messages.
    memory_usage_bytes:
        Approximate Rust-side memory usage computed as
        `total_messages * size_of::<Message>()`.

    Example
    -------
    >>> reader = MessageCacheReader()
    >>> reader.load_to_cache("feed.bin")
    250000
    >>> reader.get_cache_summary()
    {
        'file_source': 'feed.bin',
        'total_messages': 250000,
        'total_orders': 210000,
        'total_trades': 40000,
        'memory_usage_bytes': 16000000,
    }
    """

    file_source: Optional[str]
    total_messages: int
    total_orders: int
    total_trades: int
    memory_usage_bytes: int


class OrderDict(TypedDict):
    """
    Python dictionary format accepted by `OrderbookBuilder.build_from_list()`
    for order-style messages.

    Required keys
    -------------
    msg_type:
        "N", "M", or "X". The Rust code also accepts the byte value as `int`,
        but strings are recommended for readability.
    order_id:
        Exchange/order identifier. Used internally to add, modify, cancel,
        and trade against orders.
    token:
        Instrument token.
    order_type:
        "B" for bid/buy side or "S" for ask/sell side.
    price:
        Integer price as encoded in the feed.
    quantity:
        Order quantity.

    Optional keys
    -------------
    exch_ts:
        Exchange timestamp. Defaults to 0 when omitted.
    local_ts:
        Local receive timestamp. Defaults to 0 when omitted.
    flags:
        Missed/flag marker. Defaults to False when omitted.

    Example
    -------
    >>> msg: OrderDict = {
    ...     "msg_type": "N",
    ...     "exch_ts": 1700000000000000000,
    ...     "order_id": 101,
    ...     "token": 1001,
    ...     "order_type": "B",
    ...     "price": 2250000,
    ...     "quantity": 75,
    ...     "local_ts": 1700000000000000500,
    ...     "flags": False,
    ... }
    """

    msg_type: OrderMessageType | int
    order_id: OrderId
    token: Token
    order_type: OrderSide | int
    price: Price
    quantity: Quantity
    exch_ts: Timestamp
    local_ts: Timestamp
    flags: bool


class TradeDict(TypedDict):
    """
    Python dictionary format accepted by `OrderbookBuilder.build_from_list()`
    for trade messages.

    Required keys
    -------------
    msg_type:
        "T". The Rust code also accepts the byte value as `int`, but the
        string form is recommended.
    buy_order_id:
        Order id on the buy side of the trade.
    sell_order_id:
        Order id on the sell side of the trade.
    token:
        Instrument token.
    trade_quantity:
        Executed quantity.

    Optional keys
    -------------
    exch_ts:
        Exchange timestamp. Defaults to 0 when omitted.
    trade_price:
        Executed price. Defaults to 0 when omitted.
    local_ts:
        Local receive timestamp. Defaults to 0 when omitted.
    flags:
        Missed/flag marker. Defaults to False when omitted.

    Example
    -------
    >>> msg: TradeDict = {
    ...     "msg_type": "T",
    ...     "buy_order_id": 101,
    ...     "sell_order_id": 202,
    ...     "token": 1001,
    ...     "trade_price": 2250100,
    ...     "trade_quantity": 25,
    ... }
    """

    msg_type: TradeMessageType | int
    buy_order_id: OrderId
    sell_order_id: OrderId
    token: Token
    trade_quantity: Quantity
    exch_ts: Timestamp
    trade_price: Price
    local_ts: Timestamp
    flags: bool


DecodedMessage: TypeAlias = OrderDict | TradeDict


class Snapshot(TypedDict):
    """
    Snapshot dictionary returned by `OrderbookBuilder.get_snapshot()` and
    `OrderbookBuilder.get_snapshot()`.

    Keys
    ----
    token:
        Instrument token requested by the user.
    found:
        True when the orderbook has state for this token, otherwise False.
    mid_price:
        Mid price calculated by the Rust orderbook manager. When no book is
        found, this is 0.
    best_bid:
        Best bid level as `(price, quantity)`, or None when unavailable.
    best_ask:
        Best ask level as `(price, quantity)`, or None when unavailable.
    spread:
        `best_ask_price - best_bid_price`, or None when either side is missing.
    bids:
        Top bid levels as a list of `(price, quantity)` tuples.
    asks:
        Top ask levels as a list of `(price, quantity)` tuples.

    Expected output
    ---------------
    >>> builder.get_snapshot(token=1001, levels=2)
    {
        'token': 1001,
        'found': True,
        'mid_price': 2250050,
        'best_bid': (2250000, 150),
        'best_ask': (2250100, 75),
        'spread': 100,
        'bids': [(2250000, 150), (2249900, 300)],
        'asks': [(2250100, 75), (2250200, 225)],
    }

    Missing-token output
    --------------------
    >>> builder.get_snapshot(token=999999, levels=5)
    {
        'token': 999999,
        'found': False,
        'mid_price': 0,
        'best_bid': None,
        'best_ask': None,
        'spread': None,
        'bids': [],
        'asks': [],
    }
    """

    token: Token
    found: bool
    mid_price: Price
    best_bid: Optional[PriceLevel]
    best_ask: Optional[PriceLevel]
    spread: Optional[Price]
    bids: list[PriceLevel]
    asks: list[PriceLevel]


class FullDepthSnapshot(TypedDict):
    """
    Full-depth dictionary returned by `OrderbookBuilder.get_full_depth()`.

    Unlike `get_snapshot()`, this method asks the Rust orderbook manager for
    every available bid and ask level for one token.

    Keys
    ----
    token:
        Instrument token requested by the user.
    found:
        True when full-depth book state exists for this token.
    best_bid:
        Best bid level as `(price, quantity)`, or None.
    best_ask:
        Best ask level as `(price, quantity)`, or None.
    spread:
        Best ask price minus best bid price, or None.
    bids:
        All bid levels known to the manager.
    asks:
        All ask levels known to the manager.

    Example
    -------
    >>> builder.get_full_depth(1001)
    {
        'token': 1001,
        'found': True,
        'best_bid': (2250000, 150),
        'best_ask': (2250100, 75),
        'spread': 100,
        'bids': [(2250000, 150), (2249900, 300), ...],
        'asks': [(2250100, 75), (2250200, 225), ...],
    }
    """

    token: Token
    found: bool
    best_bid: Optional[PriceLevel]
    best_ask: Optional[PriceLevel]
    spread: Optional[Price]
    bids: list[PriceLevel]
    asks: list[PriceLevel]


class CachedMessage:
    """
    Structured message object returned by `MessageCacheReader`.

    Common fields are always populated. Order-only and trade-only fields are
    represented as `None` when not applicable to that message kind.
    """

    message_kind: Literal["order", "trade"]
    seq_no: int
    msg_len: int
    stream_id: int
    msg_type: str
    exch_ts: int
    local_ts: int
    flags: bool
    token: int
    order_type: Optional[str]
    order_id: Optional[int]
    price: Optional[int]
    quantity: Optional[int]
    buy_order_id: Optional[int]
    sell_order_id: Optional[int]
    trade_price: Optional[int]
    trade_quantity: Optional[int]


class MessageCacheReader:
    """
    RAM-based binary feed reader.

    `MessageCacheReader` loads the complete binary feed into memory and stores
    parsed messages in Rust-side memory. It is best when you need repeated
    access to the same file, repeated book builds, summaries, or debugging.

    Use this class when
    -------------------
    - The file can fit comfortably in RAM.
    - You want `get_all_messages()`, `get_order_message()`, or
      `get_trade_message()` for debugging.
    - You want to build the orderbook multiple times without rereading disk.

    Avoid this class when
    ---------------------
    - The binary file is huge.
    - You only need one pass through the file.
    - RAM is limited. Use `StreamingBinaryLoader` instead.

    Example
    -------
    >>> from fastreader import MessageCacheReader
    >>> reader = MessageCacheReader()
    >>> total = reader.load_to_cache("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")
    >>> print(total)
    1250000
    >>> summary = reader.get_cache_summary()
    >>> print(summary["total_orders"], summary["total_trades"])
    1100000 150000
    """

    def __init__(self) -> None:
        """
        Create an empty cache reader.

        No file is opened at construction time. Call `load_to_cache()` to parse
        and store messages.

        Example
        -------
        >>> reader = MessageCacheReader()
        >>> reader.get_cache_summary()["total_messages"]
        0
        """
        ...

    @property
    def messages(self) -> list[CachedMessage]:
        """
        Read-only property exposing cached messages as structured objects.

        Equivalent to ``get_all_messages()``.
        """
        ...

    def __len__(self) -> int: ...

    @overload
    def __getitem__(self, index: int) -> CachedMessage: ...

    def load_to_cache(self, file_path: str) -> int:
        """
        Load a binary feed file fully into Rust-side memory.

        Parameters
        ----------
        file_path:
            Path to the binary NSE/feed file.

        Returns
        -------
        int
            Number of parsed order + trade messages stored in cache.

        Raises
        ------
        RuntimeError
            Raised when the file cannot be opened, parsed, or validated.

        Deep explanation
        ----------------
        Internally the Rust code calls `read_trd_ord_only::read_trd_ord_only()`.
        Only order/trade style messages are retained in the cache. After loading,
        the reader stores:

        - the source file path,
        - a Rust `Arc<Vec<Message>>`,
        - parsed order/trade message structures.

        This makes later calls very fast because they iterate over memory rather
        than disk.

        Example
        -------
        >>> reader = MessageCacheReader()
        >>> count = reader.load_to_cache("feed.bin")
        >>> count
        250000

        Expected output
        ---------------
        The exact number depends on the file. A successful call returns an
        integer count. A failed call raises `RuntimeError`.
        """
        ...

    def get_all_messages(self) -> list[CachedMessage]:
        """
        Return all cached messages as structured objects.

        Returns
        -------
        list[CachedMessage]
            Every cached order and trade message in the order they were read.
        """
        ...

    def get_order_message(self) -> list[dict[str, Any]]:
        """
        Return only cached order messages as decoded dictionaries.

        Returns
        -------
        list[dict[str, Any]]
            Messages whose internal enum is `Message::Order`.

        Deep explanation
        ----------------
        This includes order message types such as new, modify, and cancel/delete
        when they are represented as order packets in the feed. Trade packets are
        excluded.

        Example
        -------
        >>> orders = reader.get_order_message()
        >>> orders[0]["message_kind"]
        'order'
        """
        ...

    def get_trade_message(self) -> list[dict[str, Any]]:
        """
        Return only cached trade messages as decoded dictionaries.

        Returns
        -------
        list[dict[str, Any]]
            Messages whose internal enum is `Message::Trade`.

        Deep explanation
        ----------------
        This is useful when you want to inspect executions separately from order
        add/modify/cancel activity.

        Example
        -------
        >>> trades = reader.get_trade_message()
        >>> trades[0]["message_kind"]
        'trade'
        """
        ...

    def get_all_trade_message(self) -> list[dict[str, Any]]:
        """
        Alias for `get_trade_message()`.

        Returns
        -------
        list[dict[str, Any]]
            Same output as `get_trade_message()`.

        Deep explanation
        ----------------
        Your Rust code currently implements this as:

            self.get_trade_message()

        So it does not add any extra logic. It exists as a convenience method.

        Example
        -------
        >>> reader.get_all_trade_message() == reader.get_trade_message()
        True
        """
        ...

    def get_cache_summary(self) -> CacheSummary:
        """
        Return a dictionary summary of the current in-memory cache.

        Returns
        -------
        CacheSummary
            Dictionary containing source path, message counts, and approximate
            Rust memory usage.

        Example
        -------
        >>> reader = MessageCacheReader()
        >>> reader.load_to_cache("feed.bin")
        250000
        >>> reader.get_cache_summary()
        {
            'file_source': 'feed.bin',
            'total_messages': 250000,
            'total_orders': 210000,
            'total_trades': 40000,
            'memory_usage_bytes': 16000000,
        }
        """
        ...


class StreamingBinaryLoader:
    """
    Disk-streaming binary feed reader.

    `StreamingBinaryLoader` opens the feed file and reads messages one by one.
    It is designed for large files where loading everything into memory is not
    practical.

    Use this class when
    -------------------
    - Binary files are very large.
    - You want a single sequential pass.
    - You want to process messages incrementally into `OrderbookBuilder`.

    Important behavior
    ------------------
    - `open_stream(..., count_messages=True)` scans the file to count messages,
      then resets the file cursor. This can be slow for huge files.
    - `open_stream(..., count_messages=False)` opens quickly and returns 0.
    - `get_next_msg()` returns None when the file is exhausted.

    Example
    -------
    >>> reader = StreamingBinaryLoader()
    >>> reader.open_stream("feed.bin", count_messages=False)
    0
    >>> print(reader.get_next_msg())
    {'message_kind': 'order', 'seq_no': 1, 'msg_type': 'N', ...}
    """

    def __init__(self) -> None:
        """
        Create an unopened streaming reader.

        Example
        -------
        >>> reader = StreamingBinaryLoader()
        >>> reader.reset_cursor()  # safe even before open_stream()
        """
        ...

    def open_stream(self, file_path: str, count_messages: bool = True) -> int:
        """
        Open a binary feed file for sequential streaming.

        Parameters
        ----------
        file_path:
            Path to the binary feed file.
        count_messages:
            When True, Rust scans the whole file to count readable messages and
            returns that count. When False, Rust skips counting and returns 0.

        Returns
        -------
        int
            If `count_messages=True`, returns total message count.
            If `count_messages=False`, returns 0.

        Raises
        ------
        RuntimeError
            Raised if the file cannot be opened or the binary header is invalid.

        Deep explanation
        ----------------
        The method validates the first non-space binary header. Valid message
        type bytes are `T`, `N`, `M`, and `X`. After validation, the file cursor
        is positioned at the start of the file.

        For large feed files, use `count_messages=False` for faster startup:

        >>> reader.open_stream("huge_feed.bin", count_messages=False)
        0

        For small files or progress bars, use `count_messages=True`:

        >>> total = reader.open_stream("small_feed.bin", count_messages=True)
        >>> print(total)
        50000
        """
        ...

    def reset_cursor(self) -> None:
        """
        Reset the streaming file cursor back to the start of the file.

        Returns
        -------
        None

        Deep explanation
        ----------------
        If no file is open, the Rust method simply returns successfully.
        If a file is open, it performs `seek(0)` so the next call to
        `get_next_msg()` or `OrderbookBuilder.build_from_source()` starts
        reading from the beginning again.

        Example
        -------
        >>> reader = StreamingBinaryLoader()
        >>> reader.open_stream("feed.bin", count_messages=False)
        0
        >>> first = reader.get_next_msg()
        >>> reader.reset_cursor()
        >>> again = reader.get_next_msg()
        >>> first == again
        True
        """
        ...

    def get_next_msg(self) -> Optional[dict[str, Any]]:
        """
        Read one decoded message from the current stream position.

        Returns
        -------
        Optional[dict[str, Any]]
            One decoded message dictionary, or None at EOF.

        Raises
        ------
        RuntimeError
            Raised if file reading fails.

        Deep explanation
        ----------------
        This method is mostly for inspection/debugging. For high-performance
        orderbook construction, prefer `get_next_msg()` with
        `OrderbookBuilder.orderbook_add_msg()` or use
        `OrderbookBuilder.build_from_source()`.

        Example
        -------
        >>> reader = StreamingBinaryLoader()
        >>> reader.open_stream("feed.bin", count_messages=False)
        0
        >>> while True:
        ...     msg = reader.get_next_msg()
        ...     if msg is None:
        ...         break
        ...     print(msg["token"], msg["msg_type"])

        Expected output shape
        ---------------------
        {'message_kind': 'order', 'msg_type': 'N', ...}
        {'message_kind': 'trade', 'msg_type': 'T', ...}
        None
        """
        ...

    def is_end_of_msg(self) -> bool:
        """
        Check whether the next call to `get_next_msg()`
        would hit end-of-file.

        Returns
        -------
        bool
            True if there is no next message available.
            False if another message can still be read.

        Important
        ---------
        This method peeks ahead and then restores the file cursor, so it does
        not consume the next message.
        """
        ...

    def attach_symbol_master(self, master: "SymbolMaster") -> None:
        """
        Attach a loaded `SymbolMaster` to auto-enrich streamed messages.

        After attaching, each `get_next_msg()` result may include enriched keys
        when the token exists in the loaded contract master:

        - `token_symbol`
        - `strike_price`
        - `option_type`
        - `expiry`
        - `lot_size`
        - `name`
        """
        ...

    def detach_symbol_master(self) -> None:
        """
        Remove the attached symbol master.

        Subsequent streamed messages will return symbol-enrichment fields as
        `None` again.
        """
        ...


class OrderbookBuilder:
    """
    Rust-backed orderbook construction engine.

    `OrderbookBuilder` consumes order and trade messages and maintains market
    depth by token. It can process messages from:

    - `MessageCacheReader` for cached RAM-based processing,
    - `StreamingBinaryLoader` for one-pass disk streaming,
    - `list[dict]` for Python-created decoded test messages.

    Main responsibilities
    ---------------------
    - Apply optional message-type filters.
    - Process new/modify/cancel order events.
    - Process trade events.
    - Maintain bid/ask levels internally.
    - Expose top-N snapshot, full depth, and CSV-style snapshot rows.

    Recommended workflow
    --------------------
    >>> reader = StreamingBinaryLoader()
    >>> reader.open_stream("feed.bin", count_messages=False)
    0
    >>> builder = OrderbookBuilder()
    >>> builder.apply_filter(["N", "M", "X", "T"])
    >>> processed = builder.build_from_source(reader, limit=100_000)
    >>> print(processed)
    100000
    >>> print(builder.get_snapshot(token=1001, levels=5))
    {'token': 1001, 'found': True, 'mid_price': ..., ...}
    """

    def __init__(self) -> None:
        """
        Create an empty orderbook builder.

        No market data is processed during initialization. The internal
        `OrderBookManager` starts empty.

        Example
        -------
        >>> builder = OrderbookBuilder()
        >>> builder.get_snapshot(1001)["found"]
        False
        """
        ...

    def apply_filter(self, logic_criteria: Optional[Sequence[str]] = None) -> None:
        """
        Restrict which message types are processed by the builder.

        Parameters
        ----------
        logic_criteria:
            Sequence of message type strings. Only the first byte/character of
            each item is used. Pass None to process all supported message types.

        Returns
        -------
        None

        Supported filters
        -----------------
        - ["N"]: process only new orders.
        - ["M"]: process only modify orders.
        - ["X"]: process only cancel/delete orders.
        - ["T"]: process only trades.
        - ["N", "M", "X", "T"]: process all normal orderbook events.
        - None: process all messages, same as no filter.

        Deep explanation
        ----------------
        Filtering happens before the message is applied to the internal book.
        If a message does not pass the filter, it is skipped and does not change
        the orderbook.

        Important subtlety
        ------------------
        `orderbook_add_msg()` returns False for both:

        1. stream ended, and
        2. message existed but was filtered/skipped.

        So when using filters with one-message-at-a-time processing, use care if
        you need to distinguish skipped messages from EOF.

        Example
        -------
        >>> builder = OrderbookBuilder()
        >>> builder.apply_filter(["N", "M", "X"])  # ignore trades
        >>> builder.build_from_list([
        ...     {"msg_type": "N", "order_id": 1, "token": 1001, "order_type": "B", "price": 100, "quantity": 10},
        ...     {"msg_type": "T", "buy_order_id": 1, "sell_order_id": 2, "token": 1001, "trade_price": 100, "trade_quantity": 5},
        ... ])
        1
        """
        ...

    def orderbook_add_msg(self, msg: dict[str, Any]) -> bool:
        """
        Apply one decoded message dictionary to the orderbook.

        Parameters
        ----------
        msg:
            One decoded message dictionary such as the output from
            `StreamingBinaryLoader.get_next_msg()`.

        Returns
        -------
        bool
            True if the message was accepted and applied.
            False if the message was filtered/skipped.

        Raises
        ------
        TypeError
            Raised if `msg` is not a dictionary with required keys.

        Deep explanation
        ----------------
        This method applies a single already-decoded message through the same
        internal processing path as bulk builds.

        Use this when you want manual control over the stream loop:

        >>> reader = StreamingBinaryLoader()
        >>> reader.open_stream("feed.bin", count_messages=False)
        0
        >>> builder = OrderbookBuilder()
        >>> while True:
        ...     msg = reader.get_next_msg()
        ...     if msg is None:
        ...         break
        ...     builder.orderbook_add_msg(msg)
        >>> snapshot = builder.get_snapshot(token=1001, levels=5)

        For most users, `build_from_source(reader)` is easier and safer.
        """
        ...

    @overload
    def build_from_list(self, source: MessageCacheReader) -> int:
        ...

    @overload
    def build_from_list(self, source: Sequence[DecodedMessage]) -> int:
        ...

    def build_from_list(self, source: MessageCacheReader | Sequence[DecodedMessage]) -> int:
        """
        Build/update the orderbook from cached messages or Python dictionaries.

        Parameters
        ----------
        source:
            Either:

            1. `MessageCacheReader`, already loaded with `load_to_cache()`, or
            2. a sequence of Python dictionaries matching `OrderDict` or
               `TradeDict`.

        Returns
        -------
        int
            Number of messages that were actually processed. Filtered/skipped
            messages are not counted.

        Raises
        ------
        TypeError
            Raised when `source` is not a `MessageCacheReader` or list-like
            object of dictionaries, when required keys are missing, or when an
            unsupported `msg_type` is provided.

        Deep explanation
        ----------------
        When `source` is `MessageCacheReader`, Rust iterates through its internal
        cached `Vec<Message>`.

        When `source` is a Python list of dicts, Rust converts each dict into an
        internal `OrderPacket` or `TradePacket`. This is very useful for testing
        your orderbook logic without needing a binary file.

        Required order dict fields
        --------------------------
        `msg_type`, `order_id`, `token`, `order_type`, `price`, `quantity`

        Required trade dict fields
        --------------------------
        `msg_type`, `buy_order_id`, `sell_order_id`, `token`, `trade_quantity`

        Optional fields with defaults
        -----------------------------
        `exch_ts=0`, `local_ts=0`, `flags=False`, `trade_price=0`

        Example: from Python dicts
        --------------------------
        >>> builder = OrderbookBuilder()
        >>> processed = builder.build_from_list([
        ...     {"msg_type": "N", "order_id": 1, "token": 1001, "order_type": "B", "price": 100, "quantity": 10},
        ...     {"msg_type": "N", "order_id": 2, "token": 1001, "order_type": "S", "price": 102, "quantity": 20},
        ... ])
        >>> processed
        2
        >>> builder.get_snapshot(1001, levels=1)
        {
            'token': 1001,
            'found': True,
            'mid_price': 101,
            'best_bid': (100, 10),
            'best_ask': (102, 20),
            'spread': 2,
            'bids': [(100, 10)],
            'asks': [(102, 20)],
        }

        Example: from cache
        -------------------
        >>> cache = MessageCacheReader()
        >>> cache.load_to_cache("feed.bin")
        250000
        >>> builder = OrderbookBuilder()
        >>> builder.build_from_list(cache)
        250000
        """
        ...

    @overload
    def build_from_source(self, source: MessageCacheReader, limit: Optional[int] = None) -> int:
        ...

    @overload
    def build_from_source(self, source: StreamingBinaryLoader, limit: Optional[int] = None) -> int:
        ...

    def build_from_source(
        self,
        source: MessageCacheReader | StreamingBinaryLoader,
        limit: Optional[int] = None,
    ) -> int:
        """
        Build/update the orderbook from either a cache reader or stream reader.

        Parameters
        ----------
        source:
            `MessageCacheReader` or `StreamingBinaryLoader`.
        limit:
            Maximum number of successfully processed messages when source is a
            `StreamingBinaryLoader`. For `MessageCacheReader`, the Rust code
            delegates to `build_from_list(source)`, so the limit is ignored.

        Returns
        -------
        int
            Number of messages actually processed.

        Raises
        ------
        TypeError
            Raised if `source` is neither `MessageCacheReader` nor
            `StreamingBinaryLoader`.
        RuntimeError
            Raised if streaming read fails.

        Deep explanation
        ----------------
        This is the easiest high-level build function:

        - For `MessageCacheReader`, it processes all cached messages.
        - For `StreamingBinaryLoader`, it repeatedly reads from the current file
          cursor until EOF or until `limit` processed messages are reached.

        Important detail about `limit`
        ------------------------------
        In the Rust implementation, the loop stops when the number of processed
        messages reaches `limit`. If a filter is active, skipped messages do not
        increment the processed count.

        Example: stream first 100,000 processed messages
        -----------------------------------------------
        >>> reader = StreamingBinaryLoader()
        >>> reader.open_stream("feed.bin", count_messages=False)
        0
        >>> builder = OrderbookBuilder()
        >>> builder.build_from_source(reader, limit=100_000)
        100000

        Example: process complete cache
        -------------------------------
        >>> cache = MessageCacheReader()
        >>> cache.load_to_cache("feed.bin")
        250000
        >>> builder = OrderbookBuilder()
        >>> builder.build_from_source(cache)
        250000
        """
        ...

    def get_active_tokens(self) -> list[int]:
        """
        Return tokens currently active in the internal orderbook state.

        Returns
        -------
        list[int]
            Token ids that currently have orderbook data.
        """
        ...

    def get_full_depth(self, token: int) -> FullDepthSnapshot:
        """
        Return complete known bid/ask depth for one token.

        Parameters
        ----------
        token:
            Instrument token.

        Returns
        -------
        FullDepthSnapshot
            Full-depth dictionary with best levels, spread, bids, and asks.

        Deep explanation
        ----------------
        This method asks the Rust `OrderBookManager` for all available levels
        for the token. It is more detailed than `get_snapshot()`, which limits
        levels.

        Use it when you need all currently known book depth:

        >>> depth = builder.get_full_depth(1001)
        >>> if depth["found"]:
        ...     print(depth["best_bid"], depth["best_ask"], depth["spread"])
        (2250000, 150) (2250100, 75) 100

        Expected missing output
        -----------------------
        >>> builder.get_full_depth(999999)
        {
            'token': 999999,
            'found': False,
            'best_bid': None,
            'best_ask': None,
            'spread': None,
            'bids': [],
            'asks': [],
        }
        """
        ...

    def get_snapshot(self, token: int, levels: Optional[int] = None) -> Snapshot:
        """
        Return top-N orderbook snapshot for one token.

        Parameters
        ----------
        token:
            Instrument token.
        levels:
            Number of bid/ask levels to return. Defaults to 5 when omitted.

        Returns
        -------
        Snapshot
            Snapshot dictionary containing `found`, `mid_price`, best levels,
            spread, top bids, and top asks.

        Deep explanation
        ----------------
        This is the main user-facing orderbook query. It is ideal for dashboards,
        backtesting snapshots, and quick checks after processing a stream.

        The Rust code calls `manager.get_top_levels(token, levels)`. If a book
        exists, bids and asks are returned as Python lists of `(price, quantity)`
        tuples.

        Example
        -------
        >>> snapshot = builder.get_snapshot(token=1001, levels=5)
        >>> snapshot["found"]
        True
        >>> snapshot["best_bid"]
        (2250000, 150)
        >>> snapshot["best_ask"]
        (2250100, 75)
        >>> snapshot["spread"]
        100
        """
        ...

    def snapshot_header(self) -> str:
        """
        Return CSV header for `get_snapshot_row()` output.

        Returns
        -------
        str
            Comma-separated header with fixed 5 bid/ask levels.

        Deep explanation
        ----------------
        The Rust implementation returns a fixed 23-column schema:

        - local_ts
        - exch_ts
        - mid_price
        - bid_price_0, bid_qty_0, ask_price_0, ask_qty_0
        - bid_price_1, bid_qty_1, ask_price_1, ask_qty_1
        - bid_price_2, bid_qty_2, ask_price_2, ask_qty_2
        - bid_price_3, bid_qty_3, ask_price_3, ask_qty_3
        - bid_price_4, bid_qty_4, ask_price_4, ask_qty_4

        Example
        -------
        >>> builder.snapshot_header()
        'local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,bid_price_1,bid_qty_1,ask_price_1,ask_qty_1,bid_price_2,bid_qty_2,ask_price_2,ask_qty_2,bid_price_3,bid_qty_3,ask_price_3,ask_qty_3,bid_price_4,bid_qty_4,ask_price_4,ask_qty_4'
        """
        ...

    def get_snapshot_row(self, token: int, levels: Optional[int] = None) -> str:
        """
        Return one CSV-formatted snapshot row for a token.

        Parameters
        ----------
        token:
            Instrument token.
        levels:
            Requested top depth count. Defaults to 5. The row formatter pads to
            five bid/ask levels with zero values.

        Returns
        -------
        str
            Comma-separated row matching `snapshot_header()`.

        Deep explanation
        ----------------
        This function is useful when writing snapshots directly to a CSV file.
        It always produces the fixed schema returned by `snapshot_header()`.

        In your current Rust code, `local_ts` and `exch_ts` are set to 0 in
        `get_snapshot_row()`. The book levels and `mid_price` come from the
        internal manager. If the token is missing, all prices/quantities are 0.

        Example
        -------
        >>> header = builder.snapshot_header()
        >>> row = builder.get_snapshot_row(token=1001, levels=5)
        >>> print(header)
        local_ts,exch_ts,mid_price,bid_price_0,bid_qty_0,ask_price_0,ask_qty_0,...
        >>> print(row)
        0,0,2250050,2250000,150,2250100,75,2249900,300,2250200,225,0,0,0,0,0,0,0,0,0,0,0,0

        File writing example
        --------------------
        >>> with open("snapshot.csv", "w") as f:
        ...     f.write(builder.snapshot_header() + "\n")
        ...     f.write(builder.get_snapshot_row(1001, levels=5) + "\n")
        """
        ...


class FeedPathBuilder:
    """
    Build standardized NSE feed binary file paths.

    Supports both FO and CM segments with default base path `/nas/50.30`.
    """

    def __init__(self) -> None:
        ...

    def build(
        self,
        segment: str,
        stream_id: int,
        day: int,
        month: int,
        year: int,
        base_path: Optional[str] = None,
    ) -> str:
        ...

    def build_and_verify(
        self,
        segment: str,
        stream_id: int,
        day: int,
        month: int,
        year: int,
        base_path: Optional[str] = None,
    ) -> str:
        ...

    def __repr__(self) -> str:
        ...


class SymbolMaster:
    """
    Load contract master CSV files and enrich token-based messages.
    """

    def __init__(self) -> None:
        ...

    def load(self, csv_path: str) -> int:
        ...

    def load_for_date(
        self,
        segment: str,
        day: int,
        month: int,
        year: int,
        base_path: Optional[str] = None,
    ) -> int:
        ...

    def lookup(self, token: int) -> Dict[str, Any]:
        ...

    def enrich(self, msg: Dict[str, Any]) -> bool:
        ...

    def __len__(self) -> int:
        ...

    def __repr__(self) -> str:
        ...
