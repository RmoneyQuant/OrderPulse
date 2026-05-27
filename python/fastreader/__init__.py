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
The Rust module exposes these main Python classes:

1. `MessageCacheReader`
   RAM-based reader. It loads the whole binary feed into memory first.

2. `StreamingBinaryLoader`
   Disk-streaming reader. It keeps only a file handle and reads messages one
   by one.

3. `OrderbookBuilder`
   Orderbook engine. It consumes decoded messages, cached messages, or streamed
   messages and maintains bid/ask book state.

4. `SymbolMaster`
   Contract master loader. It maps token to symbol metadata such as symbol,
   name, strike, option type, expiry, and lot size.

5. `FeedPathBuilder`
   Utility for building standard NSE feed file paths from segment, stream id,
   and date components.
"""

from __future__ import annotations

from typing import Any, Literal, Optional, Sequence, TypedDict, TypeAlias, overload


UInt32: TypeAlias = int
UInt64: TypeAlias = int
Price: TypeAlias = int
Quantity: TypeAlias = int
Token: TypeAlias = int
OrderId: TypeAlias = int
Timestamp: TypeAlias = int
StreamId: TypeAlias = int

PriceLevel: TypeAlias = tuple[Price, Quantity]

MessageType: TypeAlias = Literal["N", "M", "X", "T"]
OrderMessageType: TypeAlias = Literal["N", "M", "X"]
TradeMessageType: TypeAlias = Literal["T"]
OrderSide: TypeAlias = Literal["B", "S"]
Segment: TypeAlias = Literal["NSE_CM", "CM", "NSE_FO", "FO"]


class CacheSummary(TypedDict):
    """
    Summary returned by `MessageCacheReader.get_cache_summary()`.

    Keys
    ----
    file_source:
        Source binary file path that was loaded into cache.
    total_messages:
        Total parsed order + trade messages held in RAM.
    total_orders:
        Count of parsed order messages.
    total_trades:
        Count of parsed trade messages.
    memory_usage_bytes:
        Approximate Rust-side memory usage.

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
    and `OrderbookBuilder.orderbook_add_msg()` for order-style messages.

    Required keys
    -------------
    msg_type:
        "N", "M", or "X".
    order_id:
        Exchange/order identifier.
    token:
        Instrument token.
    order_type:
        "B" for buy/bid side or "S" for ask/sell side.
    price:
        Integer price as encoded in the feed.
    quantity:
        Order quantity.

    Optional keys are included in this stub because Rust may return them from
    `StreamingBinaryLoader.get_next_msg()`.

    Example
    -------
    >>> msg: OrderDict = {
    ...     "message_kind": "order",
    ...     "seq_no": 1,
    ...     "msg_len": 38,
    ...     "stream_id": 2,
    ...     "msg_type": "N",
    ...     "exch_ts": 1700000000000000000,
    ...     "local_ts": 1700000000000000500,
    ...     "order_id": 101,
    ...     "token": 1001,
    ...     "order_type": "B",
    ...     "price": 2250000,
    ...     "quantity": 75,
    ...     "flags": False,
    ...     "token_symbol": "NIFTY",
    ...     "strike_price": 21350,
    ...     "option_type": "CE",
    ... }
    """

    message_kind: Literal["order"]
    seq_no: UInt32
    msg_len: int
    stream_id: StreamId
    msg_type: OrderMessageType | int
    exch_ts: Timestamp
    local_ts: Timestamp
    order_id: OrderId
    token: Token
    order_type: OrderSide | int
    price: Price
    quantity: Quantity
    flags: bool

    token_symbol: Optional[str]
    strike_price: Optional[int]
    option_type: Optional[str]
    expiry: Optional[str]
    lot_size: Optional[int]
    name: Optional[str]


class TradeDict(TypedDict):
    """
    Python dictionary format accepted by `OrderbookBuilder.build_from_list()`
    and `OrderbookBuilder.orderbook_add_msg()` for trade messages.

    Required keys
    -------------
    msg_type:
        "T".
    buy_order_id:
        Order id on the buy side of the trade.
    sell_order_id:
        Order id on the sell side of the trade.
    token:
        Instrument token.
    trade_quantity:
        Executed quantity.

    Example
    -------
    >>> msg: TradeDict = {
    ...     "message_kind": "trade",
    ...     "seq_no": 10,
    ...     "msg_len": 45,
    ...     "stream_id": 2,
    ...     "msg_type": "T",
    ...     "exch_ts": 1700000010,
    ...     "local_ts": 1700000011,
    ...     "buy_order_id": 101,
    ...     "sell_order_id": 202,
    ...     "token": 1001,
    ...     "trade_price": 2250100,
    ...     "trade_quantity": 25,
    ...     "flags": False,
    ...     "token_symbol": "NIFTY",
    ...     "strike_price": 21350,
    ...     "option_type": "CE",
    ... }
    """

    message_kind: Literal["trade"]
    seq_no: UInt32
    msg_len: int
    stream_id: StreamId
    msg_type: TradeMessageType | int
    exch_ts: Timestamp
    local_ts: Timestamp
    buy_order_id: OrderId
    sell_order_id: OrderId
    token: Token
    trade_price: Price
    trade_quantity: Quantity
    flags: bool

    token_symbol: Optional[str]
    strike_price: Optional[int]
    option_type: Optional[str]
    expiry: Optional[str]
    lot_size: Optional[int]
    name: Optional[str]


DecodedMessage: TypeAlias = OrderDict | TradeDict


class Snapshot(TypedDict):
    """
    Snapshot dictionary returned by `OrderbookBuilder.get_snapshot()` and
    `OrderbookBuilder.get_snapshot()`.
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

    Note
    ----
    Current Rust `get_full_depth()` returns best bid, best ask, spread, bids,
    and asks. It does not return `mid_price`.
    """

    token: Token
    found: bool
    best_bid: Optional[PriceLevel]
    best_ask: Optional[PriceLevel]
    spread: Optional[Price]
    bids: list[PriceLevel]
    asks: list[PriceLevel]


class SymbolLookup(TypedDict):
    """
    Dictionary returned by `SymbolMaster.lookup()`.

    When `found=True`, metadata fields contain contract information.
    When `found=False`, metadata fields are None.

    Example
    -------
    >>> sm.lookup(40434)
    {
        'token': 40434,
        'found': True,
        'symbol': 'FINNIFTY',
        'name': 'FINNIFTY2660921700CE',
        'option_type': 'CE',
        'strike': 21700,
        'expiry': '26-May-2026',
        'lot_size': 65,
    }
    """

    token: Token
    found: bool
    symbol: Optional[str]
    name: Optional[str]
    option_type: Optional[str]
    strike: Optional[int]
    expiry: Optional[str]
    lot_size: Optional[int]


class MessageCacheReader:
    """
    RAM-based binary feed reader.

    `MessageCacheReader` loads the complete binary feed into memory and stores
    parsed messages in Rust-side memory.
    """

    def __init__(self) -> None:
        """
        Create an empty cache reader.

        Example
        -------
        >>> reader = MessageCacheReader()
        """
        ...

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
        """
        ...

    def get_all_messages(self) -> list[str]:
        """
        Return all cached messages formatted as human-readable strings.

        Returns
        -------
        list[str]
            Every cached order and trade message in read order.
        """
        ...

    def get_order_message(self) -> list[dict[str, Any]]:
        """
        Return only cached order messages as decoded dictionaries.
        """
        ...

    def get_trade_message(self) -> list[dict[str, Any]]:
        """
        Return only cached trade messages as decoded dictionaries.
        """
        ...

    def get_all_trade_message(self) -> list[dict[str, Any]]:
        """
        Alias for `get_trade_message()`.
        """
        ...

    def get_cache_summary(self) -> CacheSummary:
        """
        Return a dictionary summary of the current in-memory cache.
        """
        ...


class StreamingBinaryLoader:
    """
    Disk-streaming binary feed reader.

    `StreamingBinaryLoader` opens the feed file and reads decoded messages one
    by one as Python dictionaries.

    Important behavior
    ------------------
    - `open_stream(..., count_messages=True)` scans the file and returns count.
    - `open_stream(..., count_messages=False)` opens quickly and returns 0.
    - `get_next_msg()` returns one decoded dict per call.
    - `get_next_msg()` returns None at end of file.
    """

    def __init__(self) -> None:
        """
        Create an unopened streaming reader.
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
            When True, Rust scans the whole file and returns message count.
            When False, Rust skips counting and returns 0.

        Returns
        -------
        int
            Total message count if `count_messages=True`, otherwise 0.
        """
        ...

    def reset_cursor(self) -> None:
        """
        Reset the streaming file cursor back to the start of the file.

        Example
        -------
        >>> reader.reset_cursor()
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
        This method peeks ahead and then restores the file cursor, so it should
        not consume the next message.

        Example
        -------
        >>> if reader.is_end_of_msg():
        ...     print("This is end of msg")
        ... else:
        ...     print("More messages available")
        """
        ...

    def get_next_msg(self) -> Optional[DecodedMessage]:
        """
        Read one message and return it as a Python dictionary.

        Returns
        -------
        Optional[DecodedMessage]
            - `OrderDict` for order messages.
            - `TradeDict` for trade messages.
            - None when end-of-file is reached.

        Deep explanation
        ----------------
        This is the recommended function when Python users need readable fields
        such as token, stream_id, price, quantity, order type, token symbol,
        strike price, and option type.

        By default, symbol fields are present but set to None:

        - token_symbol
        - strike_price
        - option_type

        If a `SymbolMaster` is attached through `attach_symbol_master()`, the
        loader auto-fills symbol metadata for each decoded message.

        Example
        -------
        >>> msg = reader.get_next_msg()
        >>> if msg is None:
        ...     print("This is end of msg")
        ... else:
        ...     print(msg["token"], msg["stream_id"], msg["token_symbol"])
        """
        ...

    def attach_symbol_master(self, master: SymbolMaster) -> None:
        """
        Attach a loaded `SymbolMaster` to this stream reader.

        After attaching, every `get_next_msg()` call automatically enriches
        messages with:

        - token_symbol
        - strike_price
        - option_type
        - expiry
        - lot_size
        - name

        Parameters
        ----------
        master:
            Loaded `SymbolMaster` instance.

        Example
        -------
        >>> sm = SymbolMaster()
        >>> sm.load_for_date("NSE_FO", day=26, month=5, year=2026)
        >>> reader.attach_symbol_master(sm)
        >>> msg = reader.get_next_msg()
        >>> print(msg["token_symbol"], msg["strike_price"], msg["option_type"])
        """
        ...

    def detach_symbol_master(self) -> None:
        """
        Remove the attached `SymbolMaster`.

        After detaching, `get_next_msg()` still returns symbol-related keys,
        but they remain None unless manually enriched.

        Example
        -------
        >>> reader.detach_symbol_master()
        """
        ...


class OrderbookBuilder:
    """
    Rust-backed orderbook construction engine.

    `OrderbookBuilder` consumes order/trade messages and maintains market depth
    by token.
    """

    def __init__(self) -> None:
        """
        Create an empty orderbook builder.
        """
        ...

    def apply_filter(self, logic_criteria: Optional[Sequence[str]] = None) -> None:
        """
        Restrict which message types are processed by the builder.

        Parameters
        ----------
        logic_criteria:
            Sequence of message type strings. Pass None to process all messages.

        Supported filters
        -----------------
        - ["N"] processes only new orders.
        - ["M"] processes only modify orders.
        - ["X"] processes only cancel/delete orders.
        - ["T"] processes only trades.
        - ["N", "M", "X", "T"] processes all normal events.
        - None clears the filter.

        Example
        -------
        >>> builder.apply_filter(["N", "M", "X"])
        """
        ...

    def orderbook_add_msg(self, msg: DecodedMessage) -> bool:
        """
        Push one already-decoded message into the orderbook.

        Parameters
        ----------
        msg:
            One dictionary returned by `StreamingBinaryLoader.get_next_msg()`.

        Returns
        -------
        bool
            True if the message was accepted and applied.
            False if the message was valid but skipped by `apply_filter()` or
            business rules.

        Raises
        ------
        TypeError
            Raised if `msg` is not a dictionary or required keys are missing.

        Important change
        ----------------
        In the current Rust code, `orderbook_add_msg()` expects one decoded
        message dictionary, not a `StreamingBinaryLoader`.

        Correct usage
        -------------
        >>> reader = StreamingBinaryLoader()
        >>> reader.open_stream("feed.bin", count_messages=False)
        0
        >>> builder = OrderbookBuilder()
        >>> while True:
        ...     msg = reader.get_next_msg()
        ...     if msg is None:
        ...         print("This is end of msg")
        ...         break
        ...     builder.orderbook_add_msg(msg)
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
            Either a `MessageCacheReader` or a list of decoded message dicts.

        Returns
        -------
        int
            Number of messages actually processed.
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
            `StreamingBinaryLoader`.

        Returns
        -------
        int
            Number of messages actually processed.
        """
        ...

    def get_active_tokens(self) -> list[int]:
        """
        Return all tokens currently active in the internal orderbook manager.

        Returns
        -------
        list[int]
            Tokens that currently have orderbook state.

        Example
        -------
        >>> tokens = builder.get_active_tokens()
        >>> print(tokens[:10])
        [1001, 1002, 1003]
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
            Number of bid/ask levels to return. Defaults to 5.

        Returns
        -------
        Snapshot
            Snapshot dictionary containing found, mid_price, best levels,
            spread, top bids, and top asks.
        """
        ...

    def snapshot_header(self) -> str:
        """
        Return CSV header for `get_snapshot_row()` output.

        Returns
        -------
        str
            Comma-separated header with fixed 5 bid/ask levels.
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
            Requested top depth count. Defaults to 5.

        Returns
        -------
        str
            Comma-separated row matching `snapshot_header()`.
        """
        ...


class SymbolMaster:
    """
    Loads the NSE FO/CM contract master CSV and provides fast token lookups.

    Main use cases
    --------------
    1. Load a contract master CSV.
    2. Lookup token metadata.
    3. Enrich decoded messages returned by `StreamingBinaryLoader.get_next_msg()`.
    4. Attach to `StreamingBinaryLoader` so messages are auto-enriched.

    Example
    -------
    >>> sm = SymbolMaster()
    >>> count = sm.load_for_date("NSE_FO", day=26, month=5, year=2026)
    >>> print(count)
    95632
    >>> info = sm.lookup(40434)
    >>> print(info["symbol"], info["strike"], info["option_type"])
    FINNIFTY 21700 CE
    """

    def __init__(self) -> None:
        """
        Create an empty symbol master.

        Example
        -------
        >>> sm = SymbolMaster()
        >>> len(sm)
        0
        """
        ...

    def load(self, csv_path: str) -> int:
        """
        Load contract master data from an explicit CSV file path.

        Parameters
        ----------
        csv_path:
            Full path to the contract master CSV.

        Returns
        -------
        int
            Number of contracts loaded.

        Required CSV columns
        --------------------
        - FinInstrmId
        - TckrSymb
        - XpryDt
        - StrkPric
        - OptnTp
        - StockNm

        Optional CSV columns
        --------------------
        - NewBrdLotQty
        - MinLot

        Example
        -------
        >>> sm = SymbolMaster()
        >>> sm.load("/nas/50.30/CONTRACT/26_05_2026/NSE_FO_contract_26052026.csv")
        95632
        """
        ...

    def load_for_date(
        self,
        segment: str,
        day: int,
        month: int,
        year: int,
        base_path: Optional[str] = None,
    ) -> int:
        """
        Build the standard NSE contract master path from date components and load it.

        Parameters
        ----------
        segment:
            "NSE_FO", "FO", "NSE_CM", or "CM".
        day:
            Day of month.
        month:
            Month number.
        year:
            Four-digit year.
        base_path:
            Root directory. Defaults to "/nas/50.30" in Rust.

        Returns
        -------
        int
            Number of contracts loaded.

        Path pattern
        ------------
        {base_path}/CONTRACT/{DD}_{MM}_{YYYY}/NSE_{FO|CM}_contract_{DD}{MM}{YYYY}.csv

        Example
        -------
        >>> sm = SymbolMaster()
        >>> sm.load_for_date("NSE_FO", day=26, month=5, year=2026)
        95632
        """
        ...

    def lookup(self, token: int) -> SymbolLookup:
        """
        Look up a single token.

        Parameters
        ----------
        token:
            Instrument token.

        Returns
        -------
        SymbolLookup
            Dictionary with `found=True/False` and contract metadata.

        Example
        -------
        >>> sm.lookup(40434)
        {
            'token': 40434,
            'found': True,
            'symbol': 'FINNIFTY',
            'name': 'FINNIFTY2660921700CE',
            'option_type': 'CE',
            'strike': 21700,
            'expiry': '26-May-2026',
            'lot_size': 65,
        }
        """
        ...

    def enrich(self, msg: DecodedMessage) -> bool:
        """
        Enrich one decoded message dictionary in place.

        Parameters
        ----------
        msg:
            Message dictionary returned by `StreamingBinaryLoader.get_next_msg()`.

        Returns
        -------
        bool
            True if the token was found and metadata was added.
            False if the token was not found.

        Fields updated
        --------------
        - token_symbol
        - strike_price
        - option_type
        - expiry
        - lot_size
        - name

        Example
        -------
        >>> msg = reader.get_next_msg()
        >>> if msg is not None:
        ...     found = sm.enrich(msg)
        ...     print(found, msg["token_symbol"], msg["strike_price"])
        """
        ...

    def __len__(self) -> int:
        """
        Return the number of contracts currently loaded.

        Example
        -------
        >>> len(sm)
        95632
        """
        ...

    def __repr__(self) -> str:
        """
        Return a readable representation.

        Example
        -------
        >>> repr(sm)
        'SymbolMaster(contracts=95632)'
        """
        ...


class FeedPathBuilder:
    """
    Python-accessible feed file path builder.

    It builds standard NSE feed binary paths from:

    - segment
    - stream_id
    - day
    - month
    - year
    - optional base_path

    Example
    -------
    >>> builder = FeedPathBuilder()
    >>> builder.build("NSE_CM", stream_id=2, day=29, month=12, year=2025)
    '/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin'
    """

    def __init__(self) -> None:
        """
        Create a feed path builder.

        Example
        -------
        >>> builder = FeedPathBuilder()
        """
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
        """
        Build the feed file path from components.

        Parameters
        ----------
        segment:
            "NSE_CM", "CM", "NSE_FO", or "FO".
        stream_id:
            Stream identifier. Must be greater than 0.
        day:
            Day of month.
        month:
            Month number.
        year:
            Four-digit year.
        base_path:
            Root directory. Defaults to "/nas/50.30" in Rust.

        Returns
        -------
        str
            Full binary feed file path.

        Example
        -------
        >>> builder = FeedPathBuilder()
        >>> path = builder.build("NSE_CM", stream_id=2, day=29, month=12, year=2025)
        >>> print(path)
        /nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin
        """
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
        """
        Build the feed file path and verify that it exists on disk.

        Parameters
        ----------
        segment:
            "NSE_CM", "CM", "NSE_FO", or "FO".
        stream_id:
            Stream identifier. Must be greater than 0.
        day:
            Day of month.
        month:
            Month number.
        year:
            Four-digit year.
        base_path:
            Root directory. Defaults to "/nas/50.30" in Rust.

        Returns
        -------
        str
            Full binary feed file path.

        Raises
        ------
        RuntimeError
            Raised if the generated path does not exist.

        Example
        -------
        >>> builder = FeedPathBuilder()
        >>> path = builder.build_and_verify("NSE_CM", stream_id=2, day=29, month=12, year=2025)
        """
        ...

    def __repr__(self) -> str:
        """
        Return a readable representation.

        Example
        -------
        >>> repr(FeedPathBuilder())
        'FeedPathBuilder()'
        """
        ...


# Re-export runtime classes from the compiled extension module.
# This keeps rich docs/types above while ensuring real behavior at runtime.
try:
    from .fastreader import FeedPathBuilder as _FeedPathBuilder
    from .fastreader import MessageCacheReader as _MessageCacheReader
    from .fastreader import OrderbookBuilder as _OrderbookBuilder
    from .fastreader import StreamingBinaryLoader as _StreamingBinaryLoader
    from .fastreader import SymbolMaster as _SymbolMaster

    MessageCacheReader = _MessageCacheReader
    StreamingBinaryLoader = _StreamingBinaryLoader
    OrderbookBuilder = _OrderbookBuilder
    SymbolMaster = _SymbolMaster
    FeedPathBuilder = _FeedPathBuilder
except Exception:
    # Allow import-time fallback in environments where the native module
    # is not available yet (e.g., docs generation without built artifacts).
    pass
