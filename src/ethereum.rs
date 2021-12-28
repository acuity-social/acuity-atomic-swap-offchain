use std::{
    sync::Arc,
    str::FromStr,
};
use rocksdb::DB;
use web3::futures::StreamExt;
use web3::contract::Contract;
use web3::types::{Address, FilterBuilder, U128};
use tokio::sync::broadcast::Sender;
use sp_io::hashing::keccak_256;

use crate::shared::*;

async fn update_order(order_id: [u8; 16], new_value: u128, db: Arc<DB>) {
    println!("order_id: {:?}", order_id);
    let order_key = OrderKey {
        chain_id: 60,
        adapter_id: 0,
        order_id: order_id,
    };
    let option = db.get_cf(&db.cf_handle("order_value").unwrap(), order_key.serialize()).unwrap();
    println!("order_value: {:?}", option);

    match option {
        Some(result) => {
            let value = u128::from_be_bytes(vector_as_u8_16_array(&result));
            println!("old value: {:?}", value);
            let key = OrderListKey {
                sell_chain_id: 60,
                sell_asset_id: <[u8; 8]>::default(),
                buy_chain_id: 76,
                buy_asset_id: <[u8; 8]>::default(),
                value: value,
                sell_adapter_id: 0,
                order_id: order_id,
            };
            // Remove order from list.
            db.delete_cf(&db.cf_handle("order_list").unwrap(), key.serialize()).unwrap();
        }
        None => {},
    }
    // Add order back into list.
    let key = OrderListKey {
        sell_chain_id: 60,
        sell_asset_id: <[u8; 8]>::default(),
        buy_chain_id: 76,
        buy_asset_id: <[u8; 8]>::default(),
        value: new_value,
        sell_adapter_id: 0,
        order_id: order_id,
    };
    db.put_cf(&db.cf_handle("order_list").unwrap(), key.serialize(), order_id).unwrap();

    // Store new value
    db.put_cf(&db.cf_handle("order_value").unwrap(), order_key.serialize(), new_value.to_be_bytes()).unwrap();
}

pub async fn ethereum_listen(db: Arc<DB>, tx: Sender<RequestMessage>) {
//    let ws = web3::transports::WebSocket::new("wss://mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161").await.unwrap();
    let ws = web3::transports::WebSocket::new("ws:/127.0.0.1:8546").await.unwrap();
    let web3 = web3::Web3::new(ws);

    let sell_addr = Address::from_str("0xd05647dd9D7B17aBEBa953fbF2dc8D8e87c19cb3").unwrap();
    let sell_contract = Contract::from_json(web3.eth(), sell_addr, include_bytes!("AcuityAtomicSwapSell.abi")).unwrap();
    let add_to_order = sell_contract.abi().event("AddToOrder").unwrap().signature();
    let remove_from_order = sell_contract.abi().event("RemoveFromOrder").unwrap().signature();
    let lock_sell = sell_contract.abi().event("LockSell").unwrap().signature();
    let unlock_sell = sell_contract.abi().event("UnlockSell").unwrap().signature();
    let timeout_sell = sell_contract.abi().event("TimeoutSell").unwrap().signature();

    let buy_addr = Address::from_str("0x744Ac7bbcFDDA8fdb41cF55c020d62f2109887A5").unwrap();
    let buy_contract = Contract::from_json(web3.eth(), buy_addr, include_bytes!("AcuityAtomicSwapBuy.abi")).unwrap();
    let lock_buy = buy_contract.abi().event("LockBuy").unwrap().signature();
    let unlock_buy = buy_contract.abi().event("UnlockBuy").unwrap().signature();
    let timeout_buy = buy_contract.abi().event("TimeoutBuy").unwrap().signature();

    let filter = FilterBuilder::default()
        .address(vec![sell_contract.address(), buy_contract.address()])
        .build();

    let mut sub = web3.eth_subscribe().subscribe_logs(filter).await.unwrap();

    loop {
        let raw = sub.next().await;

        match raw {
            Some(event) => {
                let event = event.unwrap();
//                println!("address: {:?}", hex::encode(&event.address));

                match hex::encode(event.address).as_str() {
                    // Sell contract
                    "d05647dd9d7b17abeba953fbf2dc8d8e87c19cb3" => {
                        println!("sell event: {:?}", event);

                        if event.topics[0] == add_to_order {
                            println!("AddToOrder: {:?}", hex::encode(&event.data.0));
                            let order_id = vector_as_u8_16_array(&event.data.0);
                            let seller = vector_as_u8_32_array_offset(&event.data.0, 32);
                            let chain_id = 76;
                            let adapter_id = 0;
                            let asset_id = <[u8; 8]>::default();
                            let price = U128::from(vector_as_u8_16_array_offset(&event.data.0, 80)).as_u128();
                            let foreign_address = vector_as_u8_32_array_offset(&event.data.0, 96);
                            let value = U128::from(vector_as_u8_16_array_offset(&event.data.0, 144)).as_u128();
                            println!("order_id: {:?}", hex::encode(&order_id));
                            println!("seller: {:?}", hex::encode(&seller));
                            println!("price: {:?}", price);
                            println!("foreign_address: {:?}", hex::encode(&foreign_address));
                            println!("value: {:?}", value);

                            let order = OrderStatic {
                                seller: seller,
                                chain_id: chain_id,
                                adapter_id: adapter_id,
                                asset_id: asset_id,
                                price: price,
                                foreign_address: foreign_address,
                            };
                            let order_key = OrderKey {
                                chain_id: 60,
                                adapter_id: 0,
                                order_id: order_id,
                            };
                            db.put_cf(&db.cf_handle("order_static").unwrap(), order_key.serialize(), bincode::serialize(&order).unwrap()).unwrap();
                            update_order(order_id, value, db.clone()).await;
                            tx.send(RequestMessage::GetOrderBook { sell_chain_id: 60, sell_asset_id: "0000000000000000".to_string(), buy_chain_id: 76, buy_asset_id: "0000000000000000".to_string() }).unwrap();
                            tx.send(RequestMessage::GetOrder { sell_chain_id: 60, sell_adapter_id: 0, order_id: hex::encode(order_id) }).unwrap();
                        }
                        if event.topics[0] == remove_from_order {
                            println!("RemoveFromOrder: {:?}", hex::encode(&event.data.0));
//                            event RemoveFromOrder(address seller, bytes32 assetIdPrice, bytes32 foreignAddress, uint256 value);
                        }
                        if event.topics[0] == lock_sell {
                            println!("LockSell: {:?}", hex::encode(&event.data.0));
//                            event LockSell(bytes16 orderId, bytes32 hashedSecret, uint256 timeout, uint256 value);
                        }
                        if event.topics[0] == unlock_sell {
                            println!("UnlockSell: {:?}", hex::encode(&event.data.0));
//                            event UnlockSell(bytes16 orderId, bytes32 secret, address buyer);
                        }
                        if event.topics[0] == timeout_sell {
                            println!("TimeoutSell: {:?}", hex::encode(&event.data.0));
//                            event TimeoutSell(bytes16 orderId, bytes32 hashedSecret);
                        }
                    },
                    // Buy contract
                    "744ac7bbcfdda8fdb41cf55c020d62f2109887a5" => {
                        println!("buy event: {:?}", event);

                        if event.topics[0] == lock_buy {
                            println!("LockBuy: {:?}", hex::encode(&event.data.0));
                            let buyer = vector_as_u8_32_array(&event.data.0);
                            let seller = vector_as_u8_20_array_offset(&event.data.0, 44);
                            let hashed_secret = vector_as_u8_32_array_offset(&event.data.0, 64);
                            let timeout = U128::from(vector_as_u8_16_array_offset(&event.data.0, 112)).as_u128();
                            let value = U128::from(vector_as_u8_16_array_offset(&event.data.0, 144)).as_u128();
                            let asset_id = vector_as_u8_16_array_offset(&event.data.0, 160);
                            let order_id = vector_as_u8_16_array_offset(&event.data.0, 176);
                            let foreign_address = vector_as_u8_32_array_offset(&event.data.0, 192);
                            println!("asset_id: {:?}", hex::encode(&asset_id));
                            println!("seller: {:?}", hex::encode(&seller));
                            println!("value: {:?}", value);
                            println!("timeout: {:?}", timeout);
                            println!("buyer: {:?}", hex::encode(&buyer));
                            println!("foreign_address: {:?}", hex::encode(&foreign_address));

                            let order_lock_list_key = OrderLockListKey {
                                chain_id: 76,
                                adapter_id: 0,
                                order_id: order_id,
                                value: value,
                                hashed_secret: hashed_secret,
                            };

                            let buy_lock = BuyLock {
                                order_id: order_id,
                                value: value,
                                timeout: timeout,
                                buyer: buyer,
                                foreign_address: foreign_address,
                                state: LockState::Locked,
                            };

                            println!("{:?}", order_lock_list_key);

                            db.put_cf(&db.cf_handle("order_lock_list").unwrap(), order_lock_list_key.serialize(), hashed_secret).unwrap();

                            let lock_key = LockKey {
                                chain_id: 76,
                                adapter_id: 0,
                                hashed_secret: hashed_secret,
                            };

                            db.put_cf(&db.cf_handle("buy_lock").unwrap(), lock_key.serialize(), bincode::serialize(&buy_lock).unwrap()).unwrap();
                            tx.send(RequestMessage::GetOrderBook { sell_chain_id: 76, sell_asset_id: "0000000000000000".to_string(), buy_chain_id: 60, buy_asset_id: "0000000000000000".to_string() }).unwrap();
                            tx.send(RequestMessage::GetOrder { sell_chain_id: 76, sell_adapter_id: 0, order_id: hex::encode(order_id) } ).unwrap();
                        }
                        if event.topics[0] == unlock_buy {
                            println!("UnlockBuy: {:?}", hex::encode(&event.data.0));
                            let buyer = vector_as_u8_32_array(&event.data.0);
                            let secret = vector_as_u8_32_array_offset(&event.data.0, 32);
                            let seller = vector_as_u8_20_array_offset(&event.data.0, 76);
                            println!("buyer: {:?}", hex::encode(&buyer));
                            println!("secret: {:?}", hex::encode(&secret));
                            println!("seller: {:?}", hex::encode(&seller));

                            let hashed_secret = keccak_256(&secret);
                            let lock_key = LockKey {
                                chain_id: 76,
                                adapter_id: 0,
                                hashed_secret: hashed_secret,
                            };
                            let result = db.get_cf(&db.cf_handle("buy_lock").unwrap(), lock_key.serialize()).unwrap().unwrap();
                            let mut buy_lock: BuyLock = bincode::deserialize(&result).unwrap();
                            println!("buy_lock: {:?}", buy_lock);
                            buy_lock.state = LockState::Unlocked;
                            db.put_cf(&db.cf_handle("buy_lock").unwrap(), lock_key.serialize(), bincode::serialize(&buy_lock).unwrap()).unwrap();
                            tx.send(RequestMessage::GetOrder { sell_chain_id: 76, sell_adapter_id: 0, order_id: hex::encode(buy_lock.order_id) } ).unwrap();
                        }
                        if event.topics[0] == timeout_buy {
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
