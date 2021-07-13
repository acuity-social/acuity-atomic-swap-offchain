use std::sync::mpsc::channel;
use rocksdb::DB;
use substrate_api_client::utils::FromHexString;
use substrate_api_client::{Api, Metadata};
use substrate_api_client::events::EventsDecoder;
use keyring::AccountKeyring;
use web3::futures::{future, StreamExt};
use sp_runtime::AccountId32 as AccountId;
use codec::{Decode, Encode};
use log::{debug, error};
use std::convert::TryFrom;

#[derive(Decode)]
struct AddToOrderEventArgs {
    seller: AccountId,
    asset_id: [u8; 16],
    price: u128,
    foreign_address: [u8; 32],
    value: u128,
}

#[tokio::main]
async fn main() {
    let path = "database";
    let _db = DB::open_default(path).unwrap();

    let url = "127.0.0.1:9946";
    let signer = AccountKeyring::Alice.pair();

     let api = Api::new(format!("ws://{}", url))
         .map(|api| api.set_signer(signer.clone()))
         .unwrap();
/*
    // print full substrate metadata json formatted
     println!(
         "{}",
         Metadata::pretty_format(&api.get_metadata().unwrap())
             .unwrap_or_else(|| "pretty format failed".to_string())
     );
*/
    println!("Subscribe to events");
    let (events_in, events_out) = channel();

    api.subscribe_events(events_in).unwrap();

    let event_decoder = match None {
        Some(d) => d,
        None => EventsDecoder::try_from(Metadata::try_from(api.get_metadata().unwrap()).unwrap()).unwrap(),
    };

    loop {
        let event_str = events_out.recv().unwrap();
        let _events = event_decoder.decode_events(&mut Vec::from_hex(event_str).unwrap().as_slice());
        println!("wait for raw event");
        match _events {
            Ok(raw_events) => {
                for (phase, event) in raw_events.into_iter() {
                    println!("Decoded Event: {:?}, {:?}", phase, event);
/*                    match event {
                        RuntimeEvent::Raw(raw)
                            if raw.module == module && raw.variant == variant =>
                        {
                            return Ok(raw);
                        }
                        _ => println!("ignoring unsupported module event: {:?}", event),
                    }
*/                }
            }
            Err(_) => println!("couldn't decode event record list"),
        }
    }

/*
    let args: AddToOrderEventArgs = api
        .wait_for_event("AtomicSwap", "AddToOrder", None, &events_out)
        .unwrap();

    println!("Seller: {:?}", args.seller);
    println!("AssetId: {:?}", args.asset_id);
    println!("Price: {:?}", args.price);
    println!("Foreign address: {:?}", args.foreign_address);
    println!("Value: {:?}", args.value);
*/




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
