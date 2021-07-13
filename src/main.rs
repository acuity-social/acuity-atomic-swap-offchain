use rocksdb::DB;
use web3::futures::{future, StreamExt};
use substrate_subxt::{
    balances::{
        TransferEvent,
    },
    sp_core::Decode,
    ClientBuilder,
    DefaultNodeRuntime,
    EventSubscription,
};

#[tokio::main]
async fn main() {
    let path = "database";
    let _db = DB::open_default(path).unwrap();

    let client = ClientBuilder::<DefaultNodeRuntime>::new()
        .register_type_size::<[u8; 32]>("[u8; 32]")
        .register_type_size::<[u8; 16]>("[u8; 16]")
        .register_type_size::<u64>("Timestamp")
        .register_type_size::<[u8; 20]>("EthereumAddress")
        .set_url("ws://127.0.0.1:9946").build().await.unwrap();

    let sub = client.subscribe_events().await.unwrap();
    let decoder = client.events_decoder();
    let mut sub = EventSubscription::<DefaultNodeRuntime>::new(sub, decoder);
    sub.filter_event::<TransferEvent<_>>();
    let raw = sub.next().await.unwrap().unwrap();
    let event = TransferEvent::<DefaultNodeRuntime>::decode(&mut &raw.data[..]);
    if let Ok(e) = event {
        println!("Balance transfer success: value: {:?}", e.amount);
    } else {
        println!("Failed to subscribe to Balances::Transfer Event");
    }

    let ws = web3::transports::WebSocket::new("wss://mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161").await.unwrap();
    let web3 = web3::Web3::new(ws);
    let mut sub = web3.eth_subscribe().subscribe_new_heads().await.unwrap();

    println!("Got subscription id: {:?}", sub.id());

    (&mut sub)
        .for_each(|x| {
            println!("Ethereum block: {:?}", x.unwrap().number.unwrap());
            future::ready(())
        })
        .await;
}
