from fastreader import (
    MessageCacheReader,
    StreamingBinaryLoader,
    OrderbookBuilder,
    SymbolMaster,
    FeedPathBuilder,
)

from fastreader import MessageCacheReader


FEED_FILE = "/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin"
reader = MessageCacheReader()
count = reader.load_to_cache(FEED_FILE)

print("Loaded messages:", count)
print(reader.messages[0])

print("Hi")
