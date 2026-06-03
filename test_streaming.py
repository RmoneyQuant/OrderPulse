import unittest

from fastreader import FeedPathBuilder, OrderbookBuilder, SymbolMaster


class TestPublicApi(unittest.TestCase):
    def test_imports(self) -> None:
        self.assertIsNotNone(FeedPathBuilder)
        self.assertIsNotNone(SymbolMaster)


class TestFeedPathBuilder(unittest.TestCase):
    def setUp(self) -> None:
        self.builder = FeedPathBuilder()

    def test_build_nse_fo_default_base(self) -> None:
        path = self.builder.build(
            segment="NSE_FO",
            stream_id=1,
            day=27,
            month=5,
            year=2026,
        )
        self.assertEqual(
            path,
            "/nas/50.30/NSE_FO/Feed_FO_StreamID_1_27_05_2026.bin",
        )

    def test_build_nse_cm_default_base(self) -> None:
        path = self.builder.build(
            segment="NSE_CM",
            stream_id=2,
            day=27,
            month=5,
            year=2026,
        )
        self.assertEqual(
            path,
            "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_27_05_2026.bin",
        )

    def test_build_custom_base_path(self) -> None:
        path = self.builder.build(
            segment="NSE_FO",
            stream_id=1,
            day=27,
            month=5,
            year=2026,
            base_path="/mnt/data",
        )
        self.assertEqual(
            path,
            "/mnt/data/NSE_FO/Feed_FO_StreamID_1_27_05_2026.bin",
        )

    def test_build_and_verify_missing_file_raises(self) -> None:
        with self.assertRaises(RuntimeError):
            self.builder.build_and_verify(
                segment="NSE_FO",
                stream_id=999,
                day=1,
                month=1,
                year=2099,
                base_path="/tmp/definitely_missing_orderpulse_path",
            )


class TestSymbolMaster(unittest.TestCase):
    def setUp(self) -> None:
        self.sm = SymbolMaster()

    def test_load_for_date_missing_file_reports_expected_pattern(self) -> None:
        with self.assertRaises(RuntimeError) as ctx:
            self.sm.load_for_date(segment="NSE_FO", day=27, month=5, year=2026)

        msg = str(ctx.exception)
        self.assertIn("/CONTRACT/27_05_2026/", msg)
        self.assertIn("NSE_FO_contract_27052026.csv", msg)

    def test_lookup_empty_master_returns_not_found_shape(self) -> None:
        info = self.sm.lookup(40434)
        self.assertEqual(info.get("token"), 40434)
        self.assertFalse(info.get("found"))
        for key in [
            "symbol",
            "name",
            "option_type",
            "strike",
            "expiry",
            "lot_size",
        ]:
            self.assertIn(key, info)


class TestOrderbookBuilder(unittest.TestCase):
    def test_build_from_list_with_synthetic_messages(self) -> None:
        builder = OrderbookBuilder()
        messages = [
            {
                "msg_type": "N",
                "order_id": 1,
                "token": 1001,
                "order_type": "B",
                "price": 100,
                "quantity": 10,
            },
            {
                "msg_type": "N",
                "order_id": 2,
                "token": 1001,
                "order_type": "S",
                "price": 102,
                "quantity": 20,
            },
            {
                "msg_type": "T",
                "buy_order_id": 1,
                "sell_order_id": 2,
                "token": 1001,
                "trade_price": 101,
                "trade_quantity": 5,
            },
        ]

        processed = builder.build_from_list(messages)
        self.assertEqual(processed, 3)

        snapshot = builder.get_snapshot(token=1001, levels=2)
        self.assertTrue(snapshot.get("found"))
        self.assertEqual(snapshot.get("best_bid"), (100, 5))
        self.assertEqual(snapshot.get("best_ask"), (102, 15))
        self.assertEqual(snapshot.get("spread"), 2)

    def test_orderbook_add_msg_accepts_decoded_dict(self) -> None:
        builder = OrderbookBuilder()
        accepted = builder.orderbook_add_msg(
            {
                "msg_type": "N",
                "order_id": 11,
                "token": 7001,
                "order_type": "B",
                "price": 500,
                "quantity": 3,
            }
        )
        self.assertTrue(accepted)

        snapshot = builder.get_snapshot(token=7001, levels=1)
        self.assertTrue(snapshot.get("found"))
        self.assertEqual(snapshot.get("best_bid"), (500, 3))


if __name__ == "__main__":
    unittest.main(verbosity=2)
