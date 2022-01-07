use std::{
    sync::Arc,
    str::FromStr,
};
use rocksdb::DB;
use web3::futures::{future, StreamExt};
use web3::contract::Contract;
use web3::types::{Address, FilterBuilder, U128};
use tokio::sync::broadcast::Sender;
use sp_io::hashing::keccak_256;

use crate::shared::*;

async fn update_order(order_id: [u8; 16], db: Arc<DB>, new_value: Option<u128>) {
    println!("order_id: {:?}", order_id);
    let order_key = OrderKey {
        chain_id: 9001,
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
                sell_chain_id: 9001,
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

    match new_value {
        Some(new_value) => {
            println!("new value: {:?}", new_value);

            // Add order back into list.
            let key = OrderListKey {
                sell_chain_id: 9001,
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
        None => {}
    }

}

pub async fn arbitrum_listen(db: Arc<DB>, tx: Sender<RequestMessage>) {
//    let ws = web3::transports::WebSocket::new("wss://arb1.arbitrum.io/ws").await.unwrap();
//    let ws = web3::transports::WebSocket::new("ws://localhost:8548/ws").await.unwrap();
    let ws = web3::transports::WebSocket::new("wss://rinkeby.arbitrum.io/ws").await.unwrap();
    let web3 = web3::Web3::new(ws);

    println!("Connected to Arbitrum.");

    let sell_addr = Address::from_str("0x744Ac7bbcFDDA8fdb41cF55c020d62f2109887A5").unwrap();
    let sell_contract = Contract::from_json(web3.eth(), sell_addr, include_bytes!("AcuityAtomicSwapSell.abi")).unwrap();
    let add_to_order = sell_contract.abi().event("AddToOrder").unwrap().signature();
    let remove_from_order = sell_contract.abi().event("RemoveFromOrder").unwrap().signature();
    let lock_sell = sell_contract.abi().event("LockSell").unwrap().signature();
    let unlock_sell = sell_contract.abi().event("UnlockSell").unwrap().signature();
    let timeout_sell = sell_contract.abi().event("TimeoutSell").unwrap().signature();

    let buy_addr = Address::from_str("0xd05647dd9D7B17aBEBa953fbF2dc8D8e87c19cb3").unwrap();
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
                println!("event: {:?}", event);
//                println!("address: {:?}", hex::encode(&event.address));

                match hex::encode(event.address).as_str() {
                    // Sell contract
                    "744ac7bbcfdda8fdb41cf55c020d62f2109887a5" => {
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
                                chain_id: 9001,
                                adapter_id: 0,
                                order_id: order_id,
                            };
                            db.put_cf(&db.cf_handle("order_static").unwrap(), order_key.serialize(), bincode::serialize(&order).unwrap()).unwrap();
                            update_order(order_id, db.clone(), Some(value)).await;
                            tx.send(RequestMessage::GetOrderBook { sell_chain_id: 9001, sell_asset_id: "0000000000000000".to_string(), buy_chain_id: 76, buy_asset_id: "0000000000000000".to_string() }).unwrap();
                            tx.send(RequestMessage::GetOrder { sell_chain_id: 9001, sell_adapter_id: 0, order_id: hex::encode(order_id) }).unwrap();
                        }
                        if event.topics[0] == remove_from_order {
                            println!("RemoveFromOrder: {:?}", hex::encode(&event.data.0));
//                            event RemoveFromOrder(address seller, bytes32 assetIdPrice, bytes32 foreignAddress, uint256 value);
                        }
                        if event.topics[0] == lock_sell {
                            println!("LockSell: {:?}", hex::encode(&event.data.0));
//                            event LockSell(bytes16 orderId, bytes32 hashedSecret, uint256 timeout, uint256 value);
                            let order_id = vector_as_u8_16_array(&event.data.0);
                            let hashed_secret = vector_as_u8_32_array_offset(&event.data.0, 32);
                            let timeout = U128::from(vector_as_u8_16_array_offset(&event.data.0, 80)).as_u128();
                            let value = U128::from(vector_as_u8_16_array_offset(&event.data.0, 112)).as_u128();
                            println!("order_id: {:?}", hex::encode(&order_id));
                            println!("hashed_secret: {:?}", hex::encode(&hashed_secret));
                            println!("timeout: {:?}", timeout);
                            println!("value: {:?}", value);

                            let sell_lock = SellLock {
                                state: LockState::Locked,
                                timeout: timeout,
                                secret: None,
                            };
                            let lock_key = LockKey {
                                chain_id: 9001,
                                adapter_id: 0,
                                hashed_secret: hashed_secret,
                            };
                            db.put_cf(&db.cf_handle("sell_lock").unwrap(), lock_key.serialize(), bincode::serialize(&sell_lock).unwrap()).unwrap();
//                            update_order(order_id, db.clone(), None).await;
                            tx.send(RequestMessage::GetOrder { sell_chain_id: 9001, sell_adapter_id: 0, order_id: hex::encode(order_id) } ).unwrap();
                        }
                        if event.topics[0] == unlock_sell {
                            println!("UnlockSell: {:?}", hex::encode(&event.data.0));
//                            event UnlockSell(bytes16 orderId, bytes32 secret, address buyer);
                            let order_id = vector_as_u8_16_array(&event.data.0);
                            let secret = vector_as_u8_32_array_offset(&event.data.0, 32);
                            println!("order_id: {:?}", hex::encode(&order_id));
                            println!("secret: {:?}", hex::encode(&secret));

                            let hashed_secret = keccak_256(&secret);

                            let mut sell_lock: SellLock = match db.get_cf(&db.cf_handle("sell_lock").unwrap(), hashed_secret).unwrap() {
                                Some(result) => bincode::deserialize(&result).unwrap(),
                                None => SellLock {
                                    timeout: 0,
                                    state: LockState::NotLocked,
                                    secret: None,
                                }
                            };

                            println!("sell_lock: {:?}", sell_lock);

                            sell_lock.state = LockState::Unlocked;
                            sell_lock.secret = Some(secret);
                            let lock_key = LockKey {
                                chain_id: 9001,
                                adapter_id: 0,
                                hashed_secret: hashed_secret,
                            };
                            db.put_cf(&db.cf_handle("sell_lock").unwrap(), lock_key.serialize(), bincode::serialize(&sell_lock).unwrap()).unwrap();
                            tx.send(RequestMessage::GetOrder { sell_chain_id: 9001, sell_adapter_id: 0, order_id: hex::encode(order_id) } ).unwrap();
                        }
                        if event.topics[0] == timeout_sell {
                            println!("TimeoutSell: {:?}", hex::encode(&event.data.0));
//                            event TimeoutSell(bytes16 orderId, bytes32 hashedSecret);
                        }
                    },
                    // Buy contract
                    "d05647dd9d7b17abeba953fbf2dc8d8e87c19cb3" => {
                        println!("buy event: {:?}", event);

                        if event.topics[0] == lock_buy {
                            println!("LockBuy: {:?}", hex::encode(&event.data.0));
                            let buyer = vector_as_u8_32_array(&event.data.0);
                            let seller = vector_as_u8_20_array_offset(&event.data.0, 44);
                            let hashed_secret = vector_as_u8_32_array_offset(&event.data.0, 64);
                            let timeout = U128::from(vector_as_u8_16_array_offset(&event.data.0, 112)).as_u128();
                            let value = U128::from(vector_as_u8_16_array_offset(&event.data.0, 144)).as_u128();
//                            let chain_id = vector_as_u8_16_array_offset(&event.data.0, 19001);
//                            let adapter_id = vector_as_u8_16_array_offset(&event.data.0, 19001);
                            let order_id = vector_as_u8_16_array_offset(&event.data.0, 168);
                            let foreign_address = vector_as_u8_32_array_offset(&event.data.0, 192);
                            println!("buyer: {:?}", hex::encode(&buyer));
                            println!("seller: {:?}", hex::encode(&seller));
                            println!("hashed_secret: {:?}", hex::encode(&hashed_secret));
                            println!("timeout: {:?}", timeout);
                            println!("value: {:?}", value);
//                            println!("chain_id: {:?}", &chain_id);
//                            println!("adapter_id: {:?}", &adapter_id);
                            println!("order_id: {:?}", hex::encode(&order_id));
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
                            tx.send(RequestMessage::GetOrderBook { sell_chain_id: 76, sell_asset_id: "0000000000000000".to_string(), buy_chain_id: 9001, buy_asset_id: "0000000000000000".to_string() }).unwrap();
                            tx.send(RequestMessage::GetOrder { sell_chain_id: 76, sell_adapter_id: 0, order_id: hex::encode(order_id) } ).unwrap();
                        }
                        if event.topics[0] == unlock_buy {
                            println!("UnlockBuy: {:?}", hex::encode(&event.data.0));
                            let buyer = vector_as_u8_32_array(&event.data.0);
                            let secret = vector_as_u8_32_array_offset(&event.data.0, 32);
                            println!("buyer: {:?}", hex::encode(&buyer));
                            println!("secret: {:?}", hex::encode(&secret));

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
