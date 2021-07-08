use std::sync::mpsc::channel;
use rocksdb::DB;
use substrate_api_client::{Api, Metadata};
use keyring::AccountKeyring;
use web3::futures::{future, StreamExt};
use sp_runtime::AccountId32 as AccountId;
use codec::{Decode, Encode};

// Look at the how the transfer event looks like in in the metadata
#[derive(Decode)]
struct TransferEventArgs {
    from: AccountId,
    to: AccountId,
    value: u128,
}


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
/*     println!(
         "{}",
         Metadata::pretty_format(&api.get_metadata().unwrap())
             .unwrap_or_else(|| "pretty format failed".to_string())
     );
*/

     println!("Subscribe to events");
     let (events_in, events_out) = channel();

     api.subscribe_events(events_in).unwrap();
     let args: TransferEventArgs = api
         .wait_for_event("Balances", "Transfer", None, &events_out)
         .unwrap();

     println!("Transactor: {:?}", args.from);
     println!("Destination: {:?}", args.to);
     println!("Value: {:?}", args.value);

    let ws = web3::transports::WebSocket::new("wss://mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161").await?;
    let web3 = web3::Web3::new(ws);
    let mut sub = web3.eth_subscribe().subscribe_new_heads().await?;

    println!("Got subscription id: {:?}", sub.id());

    (&mut sub)
        .for_each(|x| {
            println!("Ethereum block: {:?}", x.unwrap().number.unwrap());
            future::ready(())
        })
        .await;

    Ok(())
}
