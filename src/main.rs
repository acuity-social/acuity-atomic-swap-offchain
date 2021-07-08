use rocksdb::DB;
use substrate_api_client::{Api, Metadata};
use keyring::AccountKeyring;
use web3::futures::{future, StreamExt};

#[tokio::main]
async fn main() -> web3::Result {
    let path = "database";
    let _db = DB::open_default(path).unwrap();

    let url = "127.0.0.1:9946";
    let signer = AccountKeyring::Alice.pair();

     let api = Api::new(format!("ws://{}", url))
         .map(|api| api.set_signer(signer.clone()))
         .unwrap();

    // print full substrate metadata json formatted
     println!(
         "{}",
         Metadata::pretty_format(&api.get_metadata().unwrap())
             .unwrap_or_else(|| "pretty format failed".to_string())
     );

    let ws = web3::transports::WebSocket::new("wss://mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161").await?;
    let web3 = web3::Web3::new(ws.clone());
    let mut sub = web3.eth_subscribe().subscribe_new_heads().await?;

    println!("Got subscription id: {:?}", sub.id());

    (&mut sub)
        .take(5)
        .for_each(|x| {
            println!("Got: {:?}", x);
            future::ready(())
        })
        .await;

    sub.unsubscribe().await?;

    Ok(())
}
