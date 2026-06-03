from fastreader import (
		MessageCacheReader,
		StreamingBinaryLoader,
		OrderbookBuilder,
		SymbolMaster,
		FeedPathBuilder,
)

from fastreader import MessageCacheReader

reader = MessageCacheReader()
print(type(reader).__name__)

from fastreader import MessageCacheReader

reader = MessageCacheReader()
total = reader.load_to_cache("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")
print(total)

messages = reader.get_all_messages()

print("Total:", len(messages))
print(messages[0])
