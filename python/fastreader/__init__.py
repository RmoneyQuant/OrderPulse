"""
fastreader — high-performance NSE binary feed reader and orderbook builder.

Root-level imports:

    from fastreader import (
        FeedPathBuilder, MessageCacheReader,
        StreamingBinaryLoader, OrderbookBuilder,
        SymbolMaster,
    )

Compatibility aliases for older code:

    from fastreader import BinaryDataLoader, BinaryLoader, ReadMsgFromBinary
"""

from __future__ import annotations

from typing import Iterator, Optional, Tuple

from .fastreader import (
    FeedPathBuilder,
    MessageCacheReader,
    OrderbookBuilder,
    StreamingBinaryLoader,
    SymbolMaster,
)
from . import fastreader as _fastreader

__version__: str = "0.2.37"
__author__: str = "OrderPulse"

__doc__ = _fastreader.__doc__


# ---------------------------------------------------------------------------
# BinaryDataLoader — convenience wrapper: loads a file once into cache and
# exposes a streaming cursor over the same data.
# ---------------------------------------------------------------------------

class BinaryDataLoader:
    """
    Convenience wrapper that opens a binary file for both cache-based and
    streaming access at the same time.

    Example
    -------
    >>> loader = BinaryDataLoader("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")
    >>> print(loader.total_messages())
    1250000
    >>> payload, is_end = loader.get_next_message()
    >>> print(is_end)
    False
    """

    def __init__(self, path: str, token: Optional[int] = None) -> None:
        if token is not None:
            raise ValueError(
                "token filtering is not supported. "
                "Use OrderbookBuilder.apply_filter() or filter messages in Python."
            )

        self._cache = MessageCacheReader()
        self._cache.load_to_cache(path)

        self._stream = StreamingBinaryLoader()
        self._stream.open_stream(path, count_messages=False)

    # --- summary helpers ----------------------------------------------------

    def total_messages(self) -> int:
        """Total messages (orders + trades) in the file."""
        return int(self._cache.get_cache_summary()["total_messages"])

    def total_orders(self) -> int:
        """Order messages (N / M / X) in the file."""
        return int(self._cache.get_cache_summary()["total_orders"])

    def total_trades(self) -> int:
        """Trade messages (T) in the file."""
        return int(self._cache.get_cache_summary()["total_trades"])

    def summary(self) -> str:
        """Return a human-readable summary string."""
        s = self._cache.get_cache_summary()
        return (
            f"file_source={s['file_source']}\n"
            f"total_messages={s['total_messages']}\n"
            f"total_orders={s['total_orders']}\n"
            f"total_trades={s['total_trades']}\n"
            f"memory_usage_bytes={s['memory_usage_bytes']}"
        )

    # --- cache-based access -------------------------------------------------

    def get_all_messages(self, limit: Optional[int] = None) -> list[str]:
        """All messages as formatted strings, optionally capped at *limit*."""
        msgs = self._cache.get_all_messages()
        return msgs if limit is None else msgs[:limit]

    def get_order_messages(self, limit: Optional[int] = None) -> list[str]:
        """Order messages only (N / M / X), optionally capped at *limit*."""
        msgs = [m for m in self._cache.get_order_message()]
        return msgs if limit is None else msgs[:limit]

    def get_trade_messages(self, limit: Optional[int] = None) -> list[str]:
        """Trade messages only (T), optionally capped at *limit*."""
        msgs = [m for m in self._cache.get_trade_message()]
        return msgs if limit is None else msgs[:limit]

    # --- streaming access ---------------------------------------------------

    def get_next_message(self) -> Tuple[str, bool]:
        """
        Read the next message from the stream.

        Returns
        -------
        (payload, is_end_of_stream) : tuple[str, bool]
            *payload*         — formatted message string, or ``"END"`` at EOF.
            *is_end_of_stream* — ``True`` when the stream is exhausted.
        """
        return self._stream.get_next_message()

    def reset_cursor(self) -> None:
        """Rewind the streaming cursor to the start of the file."""
        self._stream.reset_cursor()


# ---------------------------------------------------------------------------
# BinaryLoader — iterator façade over BinaryDataLoader.
# Yields formatted message strings until the stream is exhausted.
# ---------------------------------------------------------------------------

class BinaryLoader(Iterator[str]):
    """
    Iterator that yields one formatted message string per step.

    Example
    -------
    >>> loader = BinaryDataLoader(file_path)
    >>> for msg in BinaryLoader(loader):
    ...     print(msg)
    Order Message: SeqNo 1, MsgType 'N', Token 200, ...
    """

    def __init__(self, reader: BinaryDataLoader) -> None:
        self._reader = reader
        self._ended = False

    def __iter__(self) -> "BinaryLoader":
        return self

    def __next__(self) -> str:
        if self._ended:
            raise StopIteration

        payload, is_end = self._reader.get_next_message()
        if is_end:
            self._ended = True
            raise StopIteration
        return payload


# ---------------------------------------------------------------------------
# Compatibility alias
# ---------------------------------------------------------------------------

ReadMsgFromBinary = BinaryDataLoader


# ---------------------------------------------------------------------------
# Public namespace
# ---------------------------------------------------------------------------

__all__ = [
    # Primary API
    "FeedPathBuilder",
    "MessageCacheReader",
    "StreamingBinaryLoader",
    "OrderbookBuilder",
    "SymbolMaster",
    # Convenience wrappers
    "BinaryDataLoader",
    "BinaryLoader",
    # Aliases
    "ReadMsgFromBinary",
    # Package metadata
    "__version__",
    "__author__",
]
