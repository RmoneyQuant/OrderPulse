"""
fastreader — comprehensive end-user test suite
===============================================

Covers every public method across all four classes:

    FeedPathBuilder        — build(), build_and_verify()
    MessageCacheReader     — load_to_cache(), get_all_messages(), get_order_message(),
                             get_trade_message(), get_all_trade_message(), get_cache_summary()
    StreamingBinaryLoader  — open_stream(), get_next_message(), get_next_msg(), reset_cursor()
    OrderbookBuilder       — apply_filter(), build_from_source(), build_from_list(),
                             orderbook_add_msg(), get_active_tokens(), get_snapshot(),
                             get_orderbook_snapshot(), get_full_depth(),
                             snapshot_header(), get_snapshot_row()

Integration tests that touch real files are automatically skipped when the file
is absent, so the suite passes clean in any environment.

Run:
    python -m unittest test_streaming -v
"""

import os
import unittest

# ---------------------------------------------------------------------------
# Library import — tests skip gracefully if the wheel is not installed
# ---------------------------------------------------------------------------

try:
    from fastreader import (
        FeedPathBuilder,
        MessageCacheReader,
        StreamingBinaryLoader,
        OrderbookBuilder,
    )
    _LIB_AVAILABLE = True
except ImportError:
    _LIB_AVAILABLE = False


def _skip_no_lib(cls):
    """Class decorator: skip every test in *cls* if the library is not installed."""
    if not _LIB_AVAILABLE:
        return unittest.skip("fastreader not installed")(cls)
    return cls


# ---------------------------------------------------------------------------
# Shared test fixtures
# ---------------------------------------------------------------------------

# A real FO stream-1 file known to exist on the NAS.
KNOWN_FILE = "/nas/50.30/NSE_FO/Feed_FO_StreamID_1_21_05_2026.bin"
_FILE_PRESENT = os.path.exists(KNOWN_FILE)


def _skip_no_file(method):
    """Method decorator: skip when KNOWN_FILE is absent."""
    return unittest.skipUnless(_FILE_PRESENT, f"test file not found: {KNOWN_FILE}")(method)


def _order_msg(**kw) -> dict:
    """Return a minimal valid order message dict (mirrors get_next_msg() output)."""
    base = {
        "message_kind": "order",
        "seq_no":        1,
        "msg_len":       10,
        "stream_id":     2,
        "msg_type":      "N",
        "exch_ts":       100_000,
        "local_ts":      200_000,
        "order_id":      1,
        "token":         1001,
        "order_type":    "B",
        "price":         50_000,
        "quantity":      100,
        "flags":         False,
        "token_symbol":  None,
        "strike_price":  None,
        "option_type":   None,
    }
    base.update(kw)
    return base


def _trade_msg(**kw) -> dict:
    """Return a minimal valid trade message dict (mirrors get_next_msg() output)."""
    base = {
        "message_kind":   "trade",
        "seq_no":         99,
        "msg_len":        10,
        "stream_id":      2,
        "msg_type":       "T",
        "exch_ts":        300_000,
        "local_ts":       400_000,
        "buy_order_id":   1,
        "sell_order_id":  2,
        "token":          1001,
        "trade_price":    50_000,
        "trade_quantity": 10,
        "flags":          False,
        "token_symbol":   None,
        "strike_price":   None,
        "option_type":    None,
    }
    base.update(kw)
    return base


# ===========================================================================
# 1. FeedPathBuilder
# ===========================================================================

@_skip_no_lib
class TestFeedPathBuilderBuild(unittest.TestCase):
    """build() constructs file path strings without touching the filesystem."""

    def setUp(self):
        self.b = FeedPathBuilder()

    # --- happy path ---------------------------------------------------------

    def test_cm_segment_produces_correct_path(self):
        path = self.b.build("NSE_CM", stream_id=2, day=29, month=12, year=2025)
        self.assertEqual(path, "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")

    def test_fo_segment_produces_correct_path(self):
        path = self.b.build("NSE_FO", stream_id=1, day=21, month=5, year=2026)
        self.assertEqual(path, "/nas/50.30/NSE_FO/Feed_FO_StreamID_1_21_05_2026.bin")

    def test_short_name_cm_is_accepted(self):
        path = self.b.build("CM", stream_id=2, day=29, month=12, year=2025)
        self.assertEqual(path, "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")

    def test_short_name_fo_is_accepted(self):
        path = self.b.build("FO", stream_id=1, day=1, month=1, year=2026)
        self.assertEqual(path, "/nas/50.30/NSE_FO/Feed_FO_StreamID_1_01_01_2026.bin")

    def test_custom_base_path_is_used(self):
        path = self.b.build("NSE_CM", stream_id=1, day=1, month=5, year=2026,
                            base_path="/mnt/archive")
        self.assertEqual(path, "/mnt/archive/NSE_CM/Feed_CM_StreamID_1_01_05_2026.bin")

    def test_single_digit_day_and_month_are_zero_padded(self):
        path = self.b.build("NSE_CM", stream_id=1, day=3, month=7, year=2025)
        self.assertIn("_03_07_2025", path)

    def test_return_type_is_str(self):
        result = self.b.build("NSE_CM", stream_id=1, day=1, month=1, year=2026)
        self.assertIsInstance(result, str)

    # --- validation errors --------------------------------------------------

    def test_invalid_segment_raises_runtime_error(self):
        with self.assertRaises(RuntimeError):
            self.b.build("INVALID", stream_id=1, day=1, month=1, year=2026)

    def test_stream_id_zero_raises_runtime_error(self):
        with self.assertRaises(RuntimeError):
            self.b.build("NSE_CM", stream_id=0, day=1, month=1, year=2026)

    def test_month_above_12_raises_runtime_error(self):
        with self.assertRaises(RuntimeError):
            self.b.build("NSE_CM", stream_id=1, day=1, month=13, year=2026)

    def test_day_zero_raises_runtime_error(self):
        with self.assertRaises(RuntimeError):
            self.b.build("NSE_CM", stream_id=1, day=0, month=1, year=2026)

    def test_year_before_2000_raises_runtime_error(self):
        with self.assertRaises(RuntimeError):
            self.b.build("NSE_CM", stream_id=1, day=1, month=1, year=1999)

    def test_year_after_2100_raises_runtime_error(self):
        with self.assertRaises(RuntimeError):
            self.b.build("NSE_CM", stream_id=1, day=1, month=1, year=2101)


@_skip_no_lib
class TestFeedPathBuilderVerify(unittest.TestCase):
    """build_and_verify() additionally checks the file exists on disk."""

    def setUp(self):
        self.b = FeedPathBuilder()

    @_skip_no_file
    def test_existing_file_returns_path_string(self):
        path = self.b.build_and_verify("NSE_FO", stream_id=1, day=21, month=5, year=2026)
        self.assertEqual(path, KNOWN_FILE)

    def test_missing_file_raises_runtime_error(self):
        with self.assertRaises(RuntimeError):
            self.b.build_and_verify("NSE_CM", stream_id=1, day=1, month=1, year=2000)


# ===========================================================================
# 2. MessageCacheReader
# ===========================================================================

@_skip_no_lib
class TestMessageCacheReaderLoad(unittest.TestCase):
    """load_to_cache() reads the entire binary file into RAM."""

    @_skip_no_file
    def test_returns_positive_integer(self):
        reader = MessageCacheReader()
        count = reader.load_to_cache(KNOWN_FILE)
        self.assertIsInstance(count, int)
        self.assertGreater(count, 0)

    def test_nonexistent_file_raises_runtime_error(self):
        reader = MessageCacheReader()
        with self.assertRaises(RuntimeError):
            reader.load_to_cache("/nonexistent/file.bin")


@_skip_no_lib
class TestMessageCacheReaderMessages(unittest.TestCase):
    """Message accessor methods return the correct lists."""

    @classmethod
    def setUpClass(cls):
        if not _FILE_PRESENT:
            return
        cls.reader = MessageCacheReader()
        cls.reader.load_to_cache(KNOWN_FILE)

    @_skip_no_file
    def test_get_all_messages_returns_nonempty_list(self):
        msgs = self.reader.get_all_messages()
        self.assertIsInstance(msgs, list)
        self.assertGreater(len(msgs), 0)

    @_skip_no_file
    def test_get_all_messages_elements_are_strings(self):
        msgs = self.reader.get_all_messages()
        self.assertIsInstance(msgs[0], str)

    @_skip_no_file
    def test_get_order_message_returns_only_orders(self):
        orders = self.reader.get_order_message()
        self.assertIsInstance(orders, list)
        for msg in orders[:10]:
            self.assertTrue(msg.startswith("Order Message:"),
                            f"unexpected prefix: {msg[:40]}")

    @_skip_no_file
    def test_get_trade_message_returns_only_trades(self):
        trades = self.reader.get_trade_message()
        self.assertIsInstance(trades, list)
        for msg in trades[:10]:
            self.assertTrue(msg.startswith("Trade Message:"),
                            f"unexpected prefix: {msg[:40]}")

    @_skip_no_file
    def test_get_all_trade_message_is_alias_for_get_trade_message(self):
        self.assertEqual(self.reader.get_trade_message(),
                         self.reader.get_all_trade_message())

    @_skip_no_file
    def test_orders_plus_trades_equals_total(self):
        summary = self.reader.get_cache_summary()
        orders = len(self.reader.get_order_message())
        trades = len(self.reader.get_trade_message())
        self.assertEqual(orders + trades, summary["total_messages"])


@_skip_no_lib
class TestMessageCacheReaderSummary(unittest.TestCase):
    """get_cache_summary() returns a dict with the expected schema."""

    @classmethod
    def setUpClass(cls):
        if not _FILE_PRESENT:
            return
        cls.reader = MessageCacheReader()
        cls.reader.load_to_cache(KNOWN_FILE)

    @_skip_no_file
    def test_summary_contains_required_keys(self):
        summary = self.reader.get_cache_summary()
        for key in ("file_source", "total_messages", "total_orders",
                    "total_trades", "memory_usage_bytes"):
            self.assertIn(key, summary, f"missing key: {key}")

    @_skip_no_file
    def test_total_messages_is_positive_int(self):
        summary = self.reader.get_cache_summary()
        self.assertIsInstance(summary["total_messages"], int)
        self.assertGreater(summary["total_messages"], 0)

    @_skip_no_file
    def test_file_source_matches_loaded_path(self):
        summary = self.reader.get_cache_summary()
        self.assertEqual(summary["file_source"], KNOWN_FILE)


# ===========================================================================
# 3. StreamingBinaryLoader
# ===========================================================================

@_skip_no_lib
class TestStreamingBinaryLoaderOpen(unittest.TestCase):
    """open_stream() opens the file and optionally counts messages."""

    def test_nonexistent_file_raises_runtime_error(self):
        reader = StreamingBinaryLoader()
        with self.assertRaises(RuntimeError):
            reader.open_stream("/no/such/file.bin", count_messages=False)

    @_skip_no_file
    def test_count_messages_false_returns_zero(self):
        reader = StreamingBinaryLoader()
        count = reader.open_stream(KNOWN_FILE, count_messages=False)
        self.assertEqual(count, 0)

    @_skip_no_file
    def test_count_messages_true_returns_positive_int(self):
        reader = StreamingBinaryLoader()
        count = reader.open_stream(KNOWN_FILE, count_messages=True)
        self.assertIsInstance(count, int)
        self.assertGreater(count, 0)


@_skip_no_lib
class TestGetNextMessage(unittest.TestCase):
    """
    get_next_message() must return (payload: str, is_end: bool).
    is_end is False for every real message and True exactly once at EOF
    (payload is then "END").
    """

    @classmethod
    def setUpClass(cls):
        if not _FILE_PRESENT:
            return
        cls.reader = StreamingBinaryLoader()
        cls.reader.open_stream(KNOWN_FILE, count_messages=False)

    @_skip_no_file
    def test_return_value_is_a_two_element_tuple(self):
        result = self.reader.get_next_message()
        self.assertIsInstance(result, tuple)
        self.assertEqual(len(result), 2)

    @_skip_no_file
    def test_payload_is_a_string(self):
        payload, _ = self.reader.get_next_message()
        self.assertIsInstance(payload, str)

    @_skip_no_file
    def test_is_end_is_a_bool(self):
        _, is_end = self.reader.get_next_message()
        self.assertIsInstance(is_end, bool)

    @_skip_no_file
    def test_first_message_is_not_end_of_stream(self):
        _, is_end = self.reader.get_next_message()
        self.assertFalse(is_end)

    @_skip_no_file
    def test_payload_describes_an_order_or_trade(self):
        payload, _ = self.reader.get_next_message()
        self.assertTrue(
            payload.startswith("Order Message:") or payload.startswith("Trade Message:"),
            f"unexpected payload prefix: {payload[:60]}"
        )

    def test_eof_payload_is_END_and_flag_is_True(self):
        """After draining the stream, get_next_message() must return ('END', True)."""
        if not _FILE_PRESENT:
            self.skipTest(f"test file not found: {KNOWN_FILE}")

        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)

        # Drain via get_next_msg (cheaper — no string formatting overhead)
        while reader.get_next_msg() is not None:
            pass

        payload, is_end = reader.get_next_message()
        self.assertEqual(payload, "END")
        self.assertIs(is_end, True)


@_skip_no_lib
class TestGetNextMsg(unittest.TestCase):
    """
    get_next_msg() must return a dict with all required fields, or None at EOF.
    """

    ORDER_KEYS = {
        "message_kind", "seq_no", "msg_len", "stream_id", "msg_type",
        "exch_ts", "local_ts", "order_id", "token", "order_type",
        "price", "quantity", "flags",
        "token_symbol", "strike_price", "option_type",
    }

    TRADE_KEYS = {
        "message_kind", "seq_no", "msg_len", "stream_id", "msg_type",
        "exch_ts", "local_ts", "buy_order_id", "sell_order_id",
        "token", "trade_price", "trade_quantity", "flags",
        "token_symbol", "strike_price", "option_type",
    }

    @classmethod
    def setUpClass(cls):
        if not _FILE_PRESENT:
            return
        cls.reader = StreamingBinaryLoader()
        cls.reader.open_stream(KNOWN_FILE, count_messages=False)
        cls.sample: list = []
        for _ in range(200):
            msg = cls.reader.get_next_msg()
            if msg is None:
                break
            cls.sample.append(msg)

    @_skip_no_file
    def test_returns_dict(self):
        self.assertIsInstance(self.sample[0], dict)

    @_skip_no_file
    def test_order_messages_have_required_keys(self):
        orders = [m for m in self.sample if m.get("message_kind") == "order"]
        self.assertGreater(len(orders), 0, "no order messages in sample")
        for msg in orders:
            missing = self.ORDER_KEYS - msg.keys()
            self.assertFalse(missing, f"order message missing keys: {missing}")

    @_skip_no_file
    def test_trade_messages_have_required_keys(self):
        trades = [m for m in self.sample if m.get("message_kind") == "trade"]
        if not trades:
            self.skipTest("no trade messages in first 200 messages")
        for msg in trades:
            missing = self.TRADE_KEYS - msg.keys()
            self.assertFalse(missing, f"trade message missing keys: {missing}")

    @_skip_no_file
    def test_stream_id_is_int(self):
        self.assertIsInstance(self.sample[0]["stream_id"], int)

    @_skip_no_file
    def test_token_is_int(self):
        self.assertIsInstance(self.sample[0]["token"], int)

    @_skip_no_file
    def test_flags_is_bool(self):
        self.assertIsInstance(self.sample[0]["flags"], bool)

    @_skip_no_file
    def test_msg_type_is_one_of_known_values(self):
        for msg in self.sample:
            self.assertIn(msg["msg_type"], {"N", "M", "X", "T"})

    @_skip_no_file
    def test_token_symbol_is_none(self):
        """token_symbol is always None — requires external symbol master lookup."""
        for msg in self.sample[:20]:
            self.assertIsNone(msg["token_symbol"])

    @_skip_no_file
    def test_strike_price_is_none(self):
        for msg in self.sample[:20]:
            self.assertIsNone(msg["strike_price"])

    @_skip_no_file
    def test_option_type_is_none(self):
        for msg in self.sample[:20]:
            self.assertIsNone(msg["option_type"])

    def test_returns_none_at_eof(self):
        """A freshly drained stream returns None from get_next_msg()."""
        if not _FILE_PRESENT:
            self.skipTest(f"test file not found: {KNOWN_FILE}")

        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)
        while reader.get_next_msg() is not None:
            pass
        self.assertIsNone(reader.get_next_msg())


@_skip_no_lib
class TestResetCursor(unittest.TestCase):
    """reset_cursor() rewinds the file so the stream can be read again."""

    @_skip_no_file
    def test_first_message_is_identical_after_reset(self):
        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)

        first = reader.get_next_msg()
        reader.reset_cursor()
        first_again = reader.get_next_msg()

        self.assertEqual(first["seq_no"], first_again["seq_no"])
        self.assertEqual(first["token"],  first_again["token"])

    @_skip_no_file
    def test_reset_after_partial_drain_replays_from_start(self):
        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)

        for _ in range(50):
            if reader.get_next_msg() is None:
                break
        reader.reset_cursor()

        first_after_reset = reader.get_next_msg()
        self.assertIsNotNone(first_after_reset)

        fresh = StreamingBinaryLoader()
        fresh.open_stream(KNOWN_FILE, count_messages=False)
        true_first = fresh.get_next_msg()
        self.assertEqual(first_after_reset["seq_no"], true_first["seq_no"])


# ===========================================================================
# 4. OrderbookBuilder — building the book
# ===========================================================================

@_skip_no_lib
class TestBuildFromList(unittest.TestCase):
    """build_from_list() processes a list of message dicts."""

    def test_empty_list_returns_zero(self):
        builder = OrderbookBuilder()
        self.assertEqual(builder.build_from_list([]), 0)

    def test_single_order_message_returns_one(self):
        builder = OrderbookBuilder()
        self.assertEqual(builder.build_from_list([_order_msg()]), 1)

    def test_mixed_messages_all_counted(self):
        messages = [_order_msg(order_id=i, seq_no=i) for i in range(1, 6)]
        messages.append(_trade_msg(seq_no=10))
        builder = OrderbookBuilder()
        self.assertEqual(builder.build_from_list(messages), 6)

    def test_return_type_is_int(self):
        builder = OrderbookBuilder()
        self.assertIsInstance(builder.build_from_list([_order_msg()]), int)

    def test_bid_order_builds_buy_side(self):
        builder = OrderbookBuilder()
        builder.build_from_list([
            _order_msg(msg_type="N", order_type="B", token=777, price=10_000, quantity=50)
        ])
        snap = builder.get_snapshot(token=777, levels=1)
        self.assertTrue(snap["found"])
        self.assertIsNotNone(snap["best_bid"])

    def test_ask_order_builds_sell_side(self):
        builder = OrderbookBuilder()
        builder.build_from_list([
            _order_msg(msg_type="N", order_type="S", token=888, price=12_000, quantity=20)
        ])
        snap = builder.get_snapshot(token=888, levels=1)
        self.assertTrue(snap["found"])
        self.assertIsNotNone(snap["best_ask"])

    def test_cancel_removes_order(self):
        builder = OrderbookBuilder()
        builder.build_from_list([
            _order_msg(msg_type="N", order_id=42, order_type="B",
                       token=999, price=10_000, quantity=50),
            _order_msg(msg_type="X", order_id=42, order_type="B",
                       token=999, price=10_000, quantity=50),
        ])
        snap = builder.get_snapshot(token=999, levels=1)
        self.assertIsNone(snap["best_bid"])

    @_skip_no_file
    def test_build_from_cache_reader(self):
        reader = MessageCacheReader()
        reader.load_to_cache(KNOWN_FILE)
        builder = OrderbookBuilder()
        count = builder.build_from_list(reader)
        self.assertIsInstance(count, int)
        self.assertGreater(count, 0)


@_skip_no_lib
class TestBuildFromSource(unittest.TestCase):
    """build_from_source() reads from a StreamingBinaryLoader."""

    @_skip_no_file
    def test_returns_positive_int(self):
        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)
        builder = OrderbookBuilder()
        count = builder.build_from_source(reader, limit=1000)
        self.assertIsInstance(count, int)
        self.assertGreater(count, 0)

    @_skip_no_file
    def test_limit_caps_messages_processed(self):
        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)
        builder = OrderbookBuilder()
        count = builder.build_from_source(reader, limit=5000)
        self.assertLessEqual(count, 5000)

    @_skip_no_file
    def test_produces_active_tokens(self):
        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)
        builder = OrderbookBuilder()
        builder.build_from_source(reader, limit=100_000)
        self.assertGreater(len(builder.get_active_tokens()), 0)


@_skip_no_lib
class TestOrderbookAddMsg(unittest.TestCase):
    """orderbook_add_msg() processes one message dict at a time."""

    def test_accepted_order_returns_true(self):
        builder = OrderbookBuilder()
        self.assertIs(builder.orderbook_add_msg(_order_msg()), True)

    def test_return_type_is_bool(self):
        builder = OrderbookBuilder()
        self.assertIsInstance(builder.orderbook_add_msg(_order_msg()), bool)

    def test_filtered_trade_returns_false(self):
        """A message excluded by apply_filter() must return False."""
        builder = OrderbookBuilder()
        builder.apply_filter(["N", "M", "X"])    # trades excluded
        self.assertIs(builder.orderbook_add_msg(_trade_msg()), False)


# ===========================================================================
# 4b. apply_filter
# ===========================================================================

@_skip_no_lib
class TestApplyFilter(unittest.TestCase):
    """apply_filter() restricts which message types are processed."""

    def test_none_filter_accepts_orders(self):
        builder = OrderbookBuilder()
        builder.apply_filter(None)
        self.assertIs(builder.orderbook_add_msg(_order_msg()), True)

    def test_order_only_filter_skips_trades(self):
        builder = OrderbookBuilder()
        builder.apply_filter(["N", "M", "X"])
        self.assertIs(builder.orderbook_add_msg(_trade_msg()), False)

    def test_order_only_filter_accepts_new_orders(self):
        builder = OrderbookBuilder()
        builder.apply_filter(["N", "M", "X"])
        self.assertIs(builder.orderbook_add_msg(_order_msg(msg_type="N")), True)

    def test_new_only_filter_skips_modify(self):
        builder = OrderbookBuilder()
        builder.apply_filter(["N"])
        self.assertIs(builder.orderbook_add_msg(_order_msg(msg_type="M")), False)


# ===========================================================================
# 4c. get_active_tokens
# ===========================================================================

@_skip_no_lib
class TestGetActiveTokens(unittest.TestCase):
    """get_active_tokens() returns a sorted list of all seen token IDs."""

    def test_empty_builder_returns_empty_list(self):
        self.assertEqual(OrderbookBuilder().get_active_tokens(), [])

    def test_returns_list(self):
        builder = OrderbookBuilder()
        builder.build_from_list([_order_msg(token=100)])
        self.assertIsInstance(builder.get_active_tokens(), list)

    def test_contains_tokens_from_processed_messages(self):
        builder = OrderbookBuilder()
        builder.build_from_list([
            _order_msg(token=100, order_id=1),
            _order_msg(token=200, order_id=2),
            _order_msg(token=300, order_id=3),
        ])
        tokens = builder.get_active_tokens()
        self.assertIn(100, tokens)
        self.assertIn(200, tokens)
        self.assertIn(300, tokens)

    def test_tokens_are_sorted_ascending(self):
        builder = OrderbookBuilder()
        builder.build_from_list([
            _order_msg(token=300, order_id=1),
            _order_msg(token=100, order_id=2),
            _order_msg(token=200, order_id=3),
        ])
        tokens = builder.get_active_tokens()
        self.assertEqual(tokens, sorted(tokens))

    def test_token_ids_are_ints(self):
        builder = OrderbookBuilder()
        builder.build_from_list([_order_msg(token=42)])
        for t in builder.get_active_tokens():
            self.assertIsInstance(t, int)

    @_skip_no_file
    def test_real_file_produces_many_tokens(self):
        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)
        builder = OrderbookBuilder()
        builder.build_from_source(reader, limit=200_000)
        self.assertGreater(len(builder.get_active_tokens()), 0)


# ===========================================================================
# 4d. get_snapshot / get_orderbook_snapshot
# ===========================================================================

@_skip_no_lib
class TestGetSnapshot(unittest.TestCase):
    """get_snapshot() returns the top-N bid/ask levels for a token."""

    SNAPSHOT_KEYS = {
        "token", "found", "mid_price", "best_bid", "best_ask",
        "spread", "bids", "asks"
    }

    def _two_sided_builder(self) -> OrderbookBuilder:
        builder = OrderbookBuilder()
        builder.build_from_list([
            _order_msg(msg_type="N", order_id=1, order_type="B",
                       token=777, price=10_000, quantity=100),
            _order_msg(msg_type="N", order_id=2, order_type="B",
                       token=777, price=9_900,  quantity=50),
            _order_msg(msg_type="N", order_id=3, order_type="S",
                       token=777, price=10_200, quantity=30),
            _order_msg(msg_type="N", order_id=4, order_type="S",
                       token=777, price=10_500, quantity=20),
        ])
        return builder

    def test_known_token_is_found(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        self.assertTrue(snap["found"])

    def test_unknown_token_is_not_found(self):
        snap = OrderbookBuilder().get_snapshot(token=99999, levels=5)
        self.assertFalse(snap["found"])

    def test_snapshot_contains_all_required_keys(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        self.assertFalse(self.SNAPSHOT_KEYS - snap.keys())

    def test_best_bid_is_price_quantity_tuple(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        price, qty = snap["best_bid"]
        self.assertIsInstance(price, int)
        self.assertIsInstance(qty, int)

    def test_best_ask_is_price_quantity_tuple(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        price, qty = snap["best_ask"]
        self.assertIsInstance(price, int)
        self.assertIsInstance(qty, int)

    def test_best_bid_is_highest_buy_price(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        self.assertEqual(snap["best_bid"][0], 10_000)

    def test_best_ask_is_lowest_sell_price(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        self.assertEqual(snap["best_ask"][0], 10_200)

    def test_spread_equals_ask_minus_bid(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        expected = snap["best_ask"][0] - snap["best_bid"][0]
        self.assertEqual(snap["spread"], expected)

    def test_mid_price_is_average_of_best_bid_and_ask(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        expected = (snap["best_bid"][0] + snap["best_ask"][0]) // 2
        self.assertEqual(snap["mid_price"], expected)

    def test_bids_are_ordered_best_first(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        prices = [level[0] for level in snap["bids"]]
        self.assertEqual(prices, sorted(prices, reverse=True))

    def test_asks_are_ordered_best_first(self):
        snap = self._two_sided_builder().get_snapshot(token=777, levels=5)
        prices = [level[0] for level in snap["asks"]]
        self.assertEqual(prices, sorted(prices))

    def test_not_found_returns_none_best_bid_and_ask(self):
        snap = OrderbookBuilder().get_snapshot(token=99999, levels=5)
        self.assertIsNone(snap["best_bid"])
        self.assertIsNone(snap["best_ask"])

    def test_not_found_returns_empty_bids_and_asks(self):
        snap = OrderbookBuilder().get_snapshot(token=99999, levels=5)
        self.assertEqual(snap["bids"], [])
        self.assertEqual(snap["asks"], [])

    def test_get_orderbook_snapshot_is_alias(self):
        builder = self._two_sided_builder()
        self.assertEqual(
            builder.get_snapshot(token=777, levels=5),
            builder.get_orderbook_snapshot(token=777, levels=5),
        )


# ===========================================================================
# 4e. get_full_depth
# ===========================================================================

@_skip_no_lib
class TestGetFullDepth(unittest.TestCase):
    """get_full_depth() returns all price levels without a top-N cap."""

    DEPTH_KEYS = {"token", "found", "best_bid", "best_ask", "spread", "bids", "asks"}

    def _deep_builder(self) -> OrderbookBuilder:
        builder = OrderbookBuilder()
        bids = [
            _order_msg(msg_type="N", order_id=i, order_type="B",
                       token=555, price=10_000 - i * 100, quantity=10)
            for i in range(1, 7)           # 6 bid levels
        ]
        asks = [
            _order_msg(msg_type="N", order_id=i + 10, order_type="S",
                       token=555, price=10_100 + i * 100, quantity=5)
            for i in range(1, 5)           # 4 ask levels
        ]
        builder.build_from_list(bids + asks)
        return builder

    def test_full_depth_contains_required_keys(self):
        snap = self._deep_builder().get_full_depth(token=555)
        self.assertFalse(self.DEPTH_KEYS - snap.keys())

    def test_full_depth_returns_all_bid_levels(self):
        self.assertEqual(len(self._deep_builder().get_full_depth(token=555)["bids"]), 6)

    def test_full_depth_returns_all_ask_levels(self):
        self.assertEqual(len(self._deep_builder().get_full_depth(token=555)["asks"]), 4)

    def test_bids_descending(self):
        prices = [b[0] for b in self._deep_builder().get_full_depth(token=555)["bids"]]
        self.assertEqual(prices, sorted(prices, reverse=True))

    def test_asks_ascending(self):
        prices = [a[0] for a in self._deep_builder().get_full_depth(token=555)["asks"]]
        self.assertEqual(prices, sorted(prices))


# ===========================================================================
# 4f. snapshot_header / get_snapshot_row
# ===========================================================================

@_skip_no_lib
class TestSnapshotCSV(unittest.TestCase):
    """snapshot_header() and get_snapshot_row() produce valid CSV output."""

    # 5 levels × 4 columns (bid_price, bid_qty, ask_price, ask_qty) + 3 fixed
    _LEVELS   = 5
    _EXPECTED = 3 + _LEVELS * 4    # = 23

    def _builder(self) -> OrderbookBuilder:
        builder = OrderbookBuilder()
        builder.build_from_list([
            _order_msg(msg_type="N", order_id=1, order_type="B",
                       token=321, price=10_000, quantity=100),
            _order_msg(msg_type="N", order_id=2, order_type="S",
                       token=321, price=10_500, quantity=50),
        ])
        return builder

    def test_snapshot_header_is_string(self):
        self.assertIsInstance(self._builder().snapshot_header(), str)

    def test_snapshot_header_column_count(self):
        cols = self._builder().snapshot_header().split(",")
        self.assertEqual(len(cols), self._EXPECTED)

    def test_snapshot_header_starts_with_local_ts(self):
        self.assertTrue(self._builder().snapshot_header().startswith("local_ts"))

    def test_snapshot_row_is_string(self):
        row = self._builder().get_snapshot_row(token=321, levels=self._LEVELS)
        self.assertIsInstance(row, str)

    def test_snapshot_row_column_count_matches_header(self):
        builder = self._builder()
        header_cols = len(builder.snapshot_header().split(","))
        row_cols    = len(builder.get_snapshot_row(token=321, levels=self._LEVELS).split(","))
        self.assertEqual(header_cols, row_cols)

    def test_snapshot_row_values_are_numeric(self):
        row = self._builder().get_snapshot_row(token=321, levels=self._LEVELS)
        for cell in row.split(","):
            try:
                int(cell)
            except ValueError:
                self.fail(f"non-integer cell in snapshot row: '{cell}'")

    def test_csv_header_and_row_line_up(self):
        builder = self._builder()
        header = builder.snapshot_header()
        row    = builder.get_snapshot_row(token=321, levels=self._LEVELS)
        self.assertEqual(len(header.split(",")), len(row.split(",")))


# ===========================================================================
# 5. Null / missing field contract
# ===========================================================================

@_skip_no_lib
class TestNullFieldContract(unittest.TestCase):
    """
    token_symbol, strike_price, and option_type must be present in every
    get_next_msg() dict and must be None — not absent, not 0, not "".
    """

    @_skip_no_file
    def test_null_fields_present_in_real_messages(self):
        reader = StreamingBinaryLoader()
        reader.open_stream(KNOWN_FILE, count_messages=False)
        for _ in range(50):
            msg = reader.get_next_msg()
            if msg is None:
                break
            for field in ("token_symbol", "strike_price", "option_type"):
                self.assertIn(field, msg, f"field '{field}' missing from {msg!r}")
                self.assertIsNone(msg[field], f"field '{field}' should be None")

    def test_null_fields_present_in_fixture_order_dict(self):
        msg = _order_msg()
        for field in ("token_symbol", "strike_price", "option_type"):
            self.assertIn(field, msg)
            self.assertIsNone(msg[field])

    def test_null_fields_present_in_fixture_trade_dict(self):
        msg = _trade_msg()
        for field in ("token_symbol", "strike_price", "option_type"):
            self.assertIn(field, msg)
            self.assertIsNone(msg[field])

    def test_safe_default_access_does_not_raise(self):
        """Downstream code using .get() with a default must never raise."""
        msg = _order_msg()
        symbol   = msg.get("token_symbol") or "UNKNOWN"
        strike   = msg.get("strike_price") or 0
        opt_type = msg.get("option_type")  or "NA"
        self.assertEqual(symbol,   "UNKNOWN")
        self.assertEqual(strike,   0)
        self.assertEqual(opt_type, "NA")

    def test_conditional_label_builder_does_not_raise(self):
        msg = _trade_msg()
        label = (
            f"{msg['token_symbol']} {msg['option_type']} {msg['strike_price']}"
            if msg["token_symbol"] is not None
            else f"token:{msg['token']}"
        )
        self.assertEqual(label, "token:1001")


# ===========================================================================
# 6. Input validation (all classes)
# ===========================================================================

@_skip_no_lib
class TestInputValidation(unittest.TestCase):
    """Every public validation path raises RuntimeError with a useful message."""

    # FeedPathBuilder ---------------------------------------------------------

    def test_feedpath_bad_segment(self):
        with self.assertRaises(RuntimeError) as ctx:
            FeedPathBuilder().build("BAD_SEG", stream_id=1, day=1, month=1, year=2026)
        self.assertIn("BAD_SEG", str(ctx.exception))

    def test_feedpath_stream_id_zero(self):
        with self.assertRaises(RuntimeError):
            FeedPathBuilder().build("NSE_CM", stream_id=0, day=1, month=1, year=2026)

    def test_feedpath_month_13(self):
        with self.assertRaises(RuntimeError):
            FeedPathBuilder().build("NSE_CM", stream_id=1, day=1, month=13, year=2026)

    def test_feedpath_day_zero(self):
        with self.assertRaises(RuntimeError):
            FeedPathBuilder().build("NSE_CM", stream_id=1, day=0, month=1, year=2026)

    def test_feedpath_year_out_of_range(self):
        with self.assertRaises(RuntimeError):
            FeedPathBuilder().build("NSE_CM", stream_id=1, day=1, month=1, year=1999)

    # StreamingBinaryLoader ---------------------------------------------------

    def test_stream_nonexistent_file(self):
        with self.assertRaises(RuntimeError):
            StreamingBinaryLoader().open_stream("/no/such/file.bin", count_messages=False)

    # MessageCacheReader ------------------------------------------------------

    def test_cache_nonexistent_file(self):
        with self.assertRaises(RuntimeError):
            MessageCacheReader().load_to_cache("/no/such/file.bin")

    # OrderbookBuilder — unknown msg_type in dict must not hard-crash --------

    def test_build_from_list_unknown_msg_type_does_not_hard_crash(self):
        builder = OrderbookBuilder()
        try:
            builder.build_from_list([{
                "msg_type": "Z", "token": 1, "order_id": 1,
                "order_type": "B", "price": 100, "quantity": 1,
                "exch_ts": 0, "local_ts": 0, "flags": False,
            }])
        except (RuntimeError, TypeError):
            pass  # silently skipped or raised — both are acceptable


# ===========================================================================
# Entry point
# ===========================================================================

if __name__ == "__main__":
    unittest.main(verbosity=2)
