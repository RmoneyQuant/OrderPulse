from fastreader import StreamingBinaryLoader, OrderbookBuilder


def generate_orderbook_snapshot(
    file_path: str,
    token: int = 1333,
    levels: int = 5,
    count_messages: bool = False,
):
    """
    Read NSE binary feed file using StreamingBinaryLoader,
    build orderbook using OrderbookBuilder,
    and return snapshot for given token.

    Parameters
    ----------
    file_path : str
        Full path of NSE binary feed file.

    token : int
        Instrument token for which snapshot is required.

    levels : int
        Number of bid/ask levels required.

    count_messages : bool
        If True, reader may count total messages before processing.
        For large files, keep False for faster processing.

    Returns
    -------
    snapshot : dict
        Orderbook snapshot for the given token.
    """

    reader = StreamingBinaryLoader()
    reader.open_stream(file_path, count_messages=count_messages)

    builder = OrderbookBuilder()

    processed_count = 0

    while True:
        processed = builder.orderbook_add_msg(reader)

        if not processed:
            break

        processed_count += 1

    snapshot = builder.get_snapshot(token=token, levels=levels)

    return {
        "processed_messages": processed_count,
        "token": token,
        "levels": levels,
        "snapshot": snapshot,
    }


# -------------------------------
# Direct .py file usage
# -------------------------------
if __name__ == "__main__":
    file_path = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"

    result = generate_orderbook_snapshot(
        file_path=file_path,
        token=1333,
        levels=5,
        count_messages=False,
    )

    print("Processed messages:", result["processed_messages"])
    print("Snapshot:")
    print(result["snapshot"])