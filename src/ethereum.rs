use std::{
    sync::Arc,
    str::FromStr,
};
use rocksdb::DB;
use web3::futures::StreamExt;
use web3::contract::Contract;
use web3::types::{Address, FilterBuilder, U64};

use crate::shared::*;

pub async fn ethereum_listen(db: Arc<DB>) {
//    let ws = web3::transports::WebSocket::new("wss://mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161").await.unwrap();
    let ws = web3::transports::WebSocket::new("ws:/127.0.0.1:8546").await.unwrap();
    let web3 = web3::Web3::new(ws);

    let sell_addr = Address::from_str("0xd05647dd9D7B17aBEBa953fbF2dc8D8e87c19cb3").unwrap();
    let sell_contract = Contract::from_json(web3.eth(), sell_addr, include_bytes!("AcuityAtomicSwapSell.abi")).unwrap();

//    let add_to_order = sell_contract.abi().event("AddToOrder").unwrap().signature();
//    let remove_from_order = sell_contract.abi().event("RemoveFromOrder").unwrap().signature();

    let buy_addr = Address::from_str("0x744Ac7bbcFDDA8fdb41cF55c020d62f2109887A5").unwrap();
    let buy_contract = Contract::from_json(web3.eth(), buy_addr, include_bytes!("AcuityAtomicSwapBuy.abi")).unwrap();
    let lock_buy = buy_contract.abi().event("LockBuy").unwrap().signature();

    let filter = FilterBuilder::default()
        .address(vec![sell_contract.address()])
        .address(vec![buy_contract.address()])
        .build();

    let mut sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

    loop {
        let raw = sub.next().await;

        match raw {
            Some(event) => {
                let event = event.unwrap();

                match hex::encode(event.address).as_str() {
                    // Sell contract
                    "d05647dd9D7B17aBEBa953fbF2dc8D8e87c19cb3" => {
                        println!("sell event: {:?}", event);
                    },
                    // Buy contract
                    "744ac7bbcfdda8fdb41cf55c020d62f2109887a5" => {
                        println!("buy event: {:?}", event);

                        if event.topics[0] == lock_buy {
                            println!("LockBuy: {:?}", hex::encode(&event.data.0));
                            let hashed_secret = vector_as_u8_32_array(&event.data.0);
                            let asset_id = vector_as_u8_16_array_offset(&event.data.0, 32);
                            let order_id = vector_as_u8_16_array_offset(&event.data.0, 48);
                            let seller = vector_as_u8_32_array_offset(&event.data.0, 64);
                            let value = u128::from(U64::from(vector_as_u8_8_array_offset(&event.data.0, 120)).as_u64()) * 1_000_000_000;
                            let timeout = U64::from(vector_as_u8_8_array_offset(&event.data.0, 152)).as_u32();
                            let buyer = hex::encode(&vector_as_u8_20_array_offset(&event.data.0, 172));
                            println!("asset_id: {:?}", hex::encode(&asset_id));
                            println!("seller: {:?}", hex::encode(&seller));
                            println!("value: {:?}", value);
                            println!("timeout: {:?}", timeout);
                            println!("buyer: {:?}", buyer);

                            let order_id_value_hashed_secret = OrderIdValueHashedSecret {
                                order_id: order_id,
                                value: value,
                                hashed_secret: hashed_secret,
                            };

                            let buy_lock = BuyLock {
                                hashed_secret: hex::encode(&hashed_secret.to_vec()),
                                value: value,
                                timeout: timeout,
                                buyer: buyer,
                            };

                            println!("{:?}", order_id_value_hashed_secret);

                            db.put_cf(&db.cf_handle("buy_lock_list").unwrap(), order_id_value_hashed_secret.serialize(), bincode::serialize(&buy_lock).unwrap()).unwrap();
                        }
                    },
                    &_ => {},
                }
            },
            None => break,
        }
    }
/*
    let mut sub = web3.eth_subscribe().subscribe_new_heads().await.unwrap();
    (&mut sub)
        .for_each(|x| {
            println!("Ethereum block: {:?}", x.unwrap().number.unwrap());
            future::ready(())
        })
        .await;
*/

}
