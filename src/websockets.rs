use std::{
    net::SocketAddr,
    sync::Arc,
};
use rocksdb::{DB, IteratorMode, Direction};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use serde::Serialize;
use web3::futures::{StreamExt, SinkExt};
use crate::shared::*;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct JsonOrder {
    order_id: String,
    seller: String,
    chain_id: u32,          // buying chain
    adapter_id: u32,        // buying adapter
    asset_id: String,      // buying asset
    price: u128,
    foreign_address: String,
    value: u128,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JsonLock {
    pub buyer: String,
    pub hashed_secret: String,
    pub buy_lock_value: u128,
    pub buy_lock_state: String,
    pub buy_lock_timeout: u128,
    pub buy_lock_foreign_address: String,
    pub sell_lock_state: String,
    pub sell_lock_timeout: u128,
    pub secret: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
enum JsonResponseMessage {
    #[serde(rename_all = "camelCase")]
    OrderBook {
        sell_chain_id: u32,
        sell_asset_id: String,
        buy_chain_id: u32,
        buy_asset_id: String,
        order_book: Vec<JsonOrder>,
    },
    #[serde(rename_all = "camelCase")]
    Order {
        sell_chain_id: u32,
        sell_adapter_id: u32,
        order: JsonOrder,
        locks: Vec<JsonLock>,
    },
}

async fn process_msg(db: &Arc<DB>, msg: RequestMessage) -> String {
    println!("msg: {:?}", msg);

    match msg {
        RequestMessage::GetOrderBook { sell_chain_id, sell_asset_id, buy_chain_id, buy_asset_id } => {
            println!("getOrderBook");
            let start_key = OrderListKey {
                sell_chain_id: sell_chain_id,
                sell_asset_id: vector_as_u8_8_array(&hex::decode(sell_asset_id.clone()).unwrap()),
                buy_chain_id: buy_chain_id,
                buy_asset_id: vector_as_u8_8_array(&hex::decode(buy_asset_id.clone()).unwrap()),
                value: u128::default(),
                sell_adapter_id: u32::default(),
                order_id: <[u8; 16]>::default(),
            };

            let iterator = db.iterator_cf(&db.cf_handle("order_list").unwrap(), IteratorMode::From(&start_key.serialize(), Direction::Forward));
            let order_list_keys = iterator.collect::<Vec<_>>();
            let mut orderbook: Vec<JsonOrder> = Vec::new();
            for order_list_key in order_list_keys {
                println!("{:?}", order_list_key);
                let order_list_key = OrderListKey::unserialize(order_list_key.0.to_vec());
                if order_list_key.sell_chain_id != sell_chain_id { break };

                println!("{:?}", order_list_key);

                let order_key = OrderKey {
                    chain_id: order_list_key.sell_chain_id,
                    adapter_id: order_list_key.sell_adapter_id,
                    order_id: order_list_key.order_id,
                };

                let order_static: OrderStatic = bincode::deserialize(&db.get_cf(&db.cf_handle("order_static").unwrap(), order_key.serialize()).unwrap().unwrap()).unwrap();
                println!("{:?}", order_static);

                orderbook.push(JsonOrder {
                    order_id: hex::encode(order_list_key.order_id),
                    seller: hex::encode(order_static.seller),
                    chain_id: order_static.chain_id,
                    adapter_id: order_static.adapter_id,
                    asset_id: hex::encode(order_static.asset_id),
                    price: order_static.price,
                    foreign_address: hex::encode(order_static.foreign_address),
                    value: order_list_key.value,
                });
            }

            let response = JsonResponseMessage::OrderBook {
                sell_chain_id: sell_chain_id,
                sell_asset_id: sell_asset_id,
                buy_chain_id: buy_chain_id,
                buy_asset_id: buy_asset_id,
                order_book: orderbook,
            };
            serde_json::to_string(&response).unwrap()
        },
        RequestMessage::GetOrder { sell_chain_id, sell_adapter_id, order_id } => {
            println!("getOrder");

            let order_id: [u8; 16] = vector_as_u8_16_array(&hex::decode(order_id).unwrap());
            let order_key = OrderKey {
                chain_id: sell_chain_id,
                adapter_id: sell_adapter_id,
                order_id: order_id,
            };
            let option = db.get_cf(&db.cf_handle("order_value").unwrap(), order_key.serialize()).unwrap();
            println!("order_value: {:?}", option);

            match option {
                Some(result) => {
                    let value = u128::from_be_bytes(vector_as_u8_16_array(&result));
                    println!("value: {:?}", value);

                    let order_static: OrderStatic = bincode::deserialize(&db.get_cf(&db.cf_handle("order_static").unwrap(), order_key.serialize()).unwrap().unwrap()).unwrap();
                    println!("{:?}", order_static);

                    let order = JsonOrder {
                        order_id: hex::encode(order_id),
                        seller: hex::encode(order_static.seller),
                        chain_id: order_static.chain_id,
                        adapter_id: order_static.adapter_id,
                        asset_id: hex::encode(order_static.asset_id),
                        price: order_static.price,
                        foreign_address: hex::encode(order_static.foreign_address),
                        value: value,
                    };

                    let order_lock_list_key = OrderLockListKey {
                        chain_id: sell_chain_id,
                        adapter_id: sell_adapter_id,
                        order_id: order_id,
                        value: u128::default(),
                        hashed_secret: <[u8; 32]>::default(),
                    };

                    let iterator = db.iterator_cf(&db.cf_handle("order_lock_list").unwrap(), IteratorMode::From(&order_lock_list_key.serialize(), Direction::Forward));
                    let mut locks: Vec<JsonLock> = Vec::new();

                    for (key, _value) in iterator {
                        let order_lock_list_key = OrderLockListKey::unserialize(key.to_vec());
                        if order_lock_list_key.order_id != order_id { break };
                        println!("hashed_secret: {:?}", order_lock_list_key.hashed_secret);

                        let lock_key = LockKey {
                            chain_id: 76,
                            adapter_id: 0,
                            hashed_secret: order_lock_list_key.hashed_secret,
                        };

                        let result = db.get_cf(&db.cf_handle("buy_lock").unwrap(), lock_key.serialize()).unwrap().unwrap();
                        let buy_lock: BuyLock = bincode::deserialize(&result).unwrap();
                        println!("buy_lock: {:?}", buy_lock);

                        let sell_lock: SellLock = match db.get_cf(&db.cf_handle("sell_lock").unwrap(), lock_key.serialize()).unwrap() {
                            Some(result) => bincode::deserialize(&result).unwrap(),
                            None => SellLock {
                                timeout: 0,
                                state: LockState::NotLocked,
                                secret: None,
                            }
                        };

                        println!("sell_lock: {:?}", sell_lock);

                        locks.push(JsonLock{
                            buyer: hex::encode(buy_lock.buyer),
                            hashed_secret: hex::encode(order_lock_list_key.hashed_secret),
                            buy_lock_value: buy_lock.value,
                            buy_lock_state: buy_lock.state.to_string(),
                            buy_lock_timeout: buy_lock.timeout,
                            buy_lock_foreign_address: hex::encode(buy_lock.foreign_address),
                            sell_lock_state: sell_lock.state.to_string(),
                            sell_lock_timeout: sell_lock.timeout,
                            secret: match sell_lock.secret {
                                Some(secret) => Some(hex::encode(secret)),
                                None => None,
                            },
                        });
                    }

                    let response = JsonResponseMessage::Order {
                        sell_chain_id: sell_chain_id,
                        sell_adapter_id: sell_adapter_id,
                        order: order,
                        locks: locks,
                    };
                    serde_json::to_string(&response).unwrap()
                },
                None => "".to_string(),
            }
        },
    }
}

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr, db: Arc<DB>, mut rx: broadcast::Receiver<RequestMessage>) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();


/*
    let mut iterator = db.iterator_cf(&db.cf_handle("order_value").unwrap(), IteratorMode::Start);
    let orders = iterator.collect::<Vec<_>>();
    println!("Orders: {:?}", orders);
    let mut iterator = db.iterator_cf(&db.cf_handle("order_static").unwrap(), IteratorMode::Start);
    let orders = iterator.collect::<Vec<_>>();
    println!("Orders: {:?}", orders);
*/
    loop {
        tokio::select! {
            Some(msg) = ws_receiver.next() => {
                let msg = msg.unwrap();
                if msg.is_text() || msg.is_binary() {
                    let json = process_msg(&db, serde_json::from_str(msg.to_text().unwrap()).unwrap()).await;
                    ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(json)).await.unwrap();
                }
            }
            msg = rx.recv() => {
                let json = process_msg(&db, msg.unwrap()).await;
                ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(json)).await.unwrap();
            }
        }
    }
}


pub async fn websockets_listen(db: Arc<DB>, tx: broadcast::Sender<RequestMessage>) {
    let addr = "127.0.0.1:8080".to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr, db.clone(), tx.subscribe()));
    }
}
