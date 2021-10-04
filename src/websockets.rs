use std::{
    net::SocketAddr,
    sync::Arc,
};
use rocksdb::{DB, IteratorMode, Direction};
use tokio::net::{TcpListener, TcpStream};
use serde::{Serialize, Deserialize};
use web3::futures::{StreamExt, SinkExt};

use crate::shared::*;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum RequestMessage {
    GetOrderBook,
    GetOrder {
        order_id: String,
    },
}

#[derive(Serialize, Debug)]
struct Order {
    order_id: String,
    order_static: OrderStatic,
    value: u128,
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
enum ResponseMessage {
    OrderBook {
        order_book: Vec<Order>,
    },
    Order {
        order: Order,
        buy_locks: Vec<BuyLock>,
    },
}

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr, db: Arc<DB>) {
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


    while let Some(msg) = ws_receiver.next().await {

      let msg = msg.unwrap();
      if msg.is_text() ||msg.is_binary() {
          let msg: RequestMessage = serde_json::from_str(msg.to_text().unwrap()).unwrap();
          println!("msg: {:?}", msg);

          match msg {
              RequestMessage::GetOrderBook => {
                  println!("getOrderBook");
                  let iterator = db.iterator_cf(&db.cf_handle("order_list").unwrap(), IteratorMode::Start);
                  let orders = iterator.collect::<Vec<_>>();
                  let mut orderbook: Vec<Order> = Vec::new();
                  for order in orders {
                      println!("{:?}", order);
                      let order = ValueOrderId::unserialize(order.0.to_vec());
                      println!("{:?}", order);
                      let order_static: OrderStatic = bincode::deserialize(&db.get_cf(&db.cf_handle("order_static").unwrap(), order.order_id).unwrap().unwrap()).unwrap();
                      println!("{:?}", order_static);

                      orderbook.push(Order {
                          order_id: hex::encode(order.order_id),
                          order_static: order_static,
                          value: order.value,
                      });
                  }

                  let response = ResponseMessage::OrderBook {
                      order_book: orderbook,
                  };
                  let json = serde_json::to_string(&response).unwrap();
                  println!("{:?}", json);
                  ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(json)).await.unwrap();
              },
              RequestMessage::GetOrder { order_id } => {
                  println!("getOrder");

                  let order_id: [u8; 16] = vector_as_u8_16_array(&hex::decode(order_id).unwrap());

                  let option = db.get_cf(&db.cf_handle("order_value").unwrap(), order_id).unwrap();
                  println!("order_value: {:?}", option);

                  match option {
                      Some(result) => {
                          let value = u128::from_be_bytes(vector_as_u8_16_array(&result));
                          println!("value: {:?}", value);
                          let order_static: OrderStatic = bincode::deserialize(&db.get_cf(&db.cf_handle("order_static").unwrap(), order_id).unwrap().unwrap()).unwrap();
                          println!("{:?}", order_static);

                          let order = Order {
                              order_id: hex::encode(order_id),
                              order_static: order_static,
                              value: value,
                          };

                          let iterator = db.iterator_cf(&db.cf_handle("buy_lock_list").unwrap(), IteratorMode::From(&order_id, Direction::Forward));
                          let mut buy_locks: Vec<BuyLock> = Vec::new();

                          for (key, value) in iterator {
                              let order_id_value_hashed_secret = OrderIdValueHashedSecret::unserialize(key.to_vec());
                              if order_id_value_hashed_secret.order_id != order_id { break };
                              buy_locks.push(bincode::deserialize(&value).unwrap());
                          }

                          let response = ResponseMessage::Order {
                              order: order,
                              buy_locks: buy_locks,
                          };
                          let json = serde_json::to_string(&response).unwrap();
                          println!("{:?}", json);
                          ws_sender.send(tokio_tungstenite::tungstenite::Message::Text(json)).await.unwrap();
                      }
                      None => {},
                  }
              },
          }

      } else if msg.is_close() {
          break;
      }
  }

//    println!("{} disconnected", &addr);
}


pub async fn websockets_listen(db: Arc<DB>) {
    let addr = "127.0.0.1:8080".to_string();

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr, db.clone()));
    }
}
