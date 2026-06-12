from fastreader import MessageCacheReader
from fastreader import OrderbookBuilder

def main():
    print("Loading messages from cache...")
    reader = MessageCacheReader()
    r = reader.open_stream("/nas/50.30/NSE_CM/Feed_CM_StreamID_2_29_12_2025.bin")

    messages = reader.get_all_messages()
    builder = OrderbookBuilder()
    l=[]
    for msg in messages[:5]:
        ob=builder.Orderbook_add_msg(msg)
        print(ob)
        l.append(ob)
        
    print("done"  , l)
   

if __name__ == "__main__":
    main()