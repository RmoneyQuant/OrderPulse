from os import read

from fastreader import StreamingBinaryLoader
from fastreader import OrderbookBuilder

builder = OrderbookBuilder()
reader = StreamingBinaryLoader()


count = reader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin", count_messages=False)
print(reader.get_next_message())
print(reader.get_next_msg())
for i in range(10):
    msg = reader.get_next_msg()
    if msg is None:
        print("End of stream reached.")
        break
    a =  builder.orderbook_add_msg(msg)
  
    
    
builder.get_orderbook_snapshot(token=1333 , levels=5)
print()