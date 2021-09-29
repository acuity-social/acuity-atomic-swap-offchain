use rocksdb::{DB, ColumnFamilyDescriptor, Options, IteratorMode, Direction};
use tokio::net::{TcpListener, TcpStream};
use tokio::join;
use std::{
    net::SocketAddr,
    sync::Arc,
    str::FromStr,
    fmt,
};
use web3::futures::{StreamExt, SinkExt};
use web3::contract::Contract;
use web3::types::{Address, FilterBuilder, U64};
use substrate_subxt::{
    balances::{
        AccountData,
        Balances,
        BalancesEventTypeRegistry,
    },
    session::{
        Session,
        SessionEventTypeRegistry,
    },
    staking::{
        Staking,
        StakingEventTypeRegistry,
    },
    sudo::{
        Sudo,
        SudoEventTypeRegistry,
    },
    system::{
        System,
        SystemEventTypeRegistry,
    },
    ClientBuilder, Client,
    EventSubscription,
    sp_runtime::traits::{
        AtLeast32Bit,
        MaybeSerialize,
        Member,
    },
    EventTypeRegistry,
    extrinsic::{
        DefaultExtra,
    },
    BasicSessionKeys,
    Runtime,
};
use std::fmt::Debug;

use sp_runtime::{
    generic::Header,
    traits::{
        BlakeTwo256,
        IdentifyAccount,
        Verify,
    },
    MultiSignature,
    OpaqueExtrinsic,
};

use sp_io::hashing::blake2_128;

use sp_core::storage::{StorageData, StorageKey};
use sp_core::{twox_128, H256};

use codec::{
    Codec,
    Decode,
    Encode,
};

use proc_macro::*;

use serde::{Serialize, Deserialize};
use bincode;
use hex;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AcuityRuntime;

impl Staking for AcuityRuntime {}

impl Runtime for AcuityRuntime {
    type Signature = MultiSignature;
    type Extra = DefaultExtra<Self>;

    fn register_type_sizes(event_type_registry: &mut EventTypeRegistry<Self>) {
        event_type_registry.with_system();
        event_type_registry.with_balances();
        event_type_registry.with_session();
        event_type_registry.with_staking();
        event_type_registry.with_sudo();
        substrate_subxt::register_default_type_sizes(event_type_registry);
    }
}

impl System for AcuityRuntime {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
    type Address = sp_runtime::MultiAddress<Self::AccountId, u32>;
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
    type Extrinsic = OpaqueExtrinsic;
    type AccountData = AccountData<<Self as Balances>::Balance>;
}

impl Balances for AcuityRuntime {
    type Balance = u128;
}

impl Session for AcuityRuntime {
    type ValidatorId = <Self as System>::AccountId;
    type Keys = BasicSessionKeys;
}

impl Sudo for AcuityRuntime {}

impl AtomicSwap for AcuityRuntime {
    type Balance = u128;
    type Moment = u64;
}



#[module]
pub trait AtomicSwap: System {
    type Balance: Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerialize
        + Debug
        + From<<Self as System>::BlockNumber>;
    type Moment: Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerialize
        + Debug
        + From<<Self as System>::BlockNumber>;
}

/// AddToOrder event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct AddToOrderEvent<T: AtomicSwap> {
    pub seller: <T as System>::AccountId,
    pub asset_id: [u8; 16],
    pub price: u128,
    pub foreign_address: [u8; 32],
    pub value: T::Balance,
}

/// RemoveFromOrder event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct RemoveFromOrderEvent<T: AtomicSwap> {
    pub seller: <T as System>::AccountId,
    pub asset_id: [u8; 16],
    pub price: u128,
    pub foreign_address: [u8; 32],
    pub value: T::Balance,
}

/// LockSell event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct LockSellEvent<T: AtomicSwap> {
    pub hashed_secret: [u8; 32],
    pub order_id: [u8; 16],
    pub value: T::Balance,
    pub timeout: T::Moment,
}

/// UnlockSell event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct UnlockSellEvent<T: AtomicSwap> {
    pub secret: [u8; 32],
    pub buyer: <T as System>::AccountId,
}

/// TimeoutSell event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct TimeoutSellEvent {
    pub hashed_secret: [u8; 32],
}

/// LockBuy event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct LockBuyEvent<T: AtomicSwap> {
    pub hashed_secret: [u8; 32],
    pub asset_id: [u8; 16],
    pub order_id: [u8; 16],
    pub seller: <T as System>::AccountId,
    pub value: T::Balance,
    pub timeout: T::Moment,
}

/// UnlockBuy event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct UnlockBuyEvent {
    pub hashed_secret: [u8; 32],
}

/// TimeoutBuy event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct TimeoutBuyEvent {
    pub hashed_secret: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Default, Serialize, Deserialize)]
pub struct OrderStatic {
    pub seller: <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId,
    pub asset_id: [u8; 16],
    pub price: u128,
    pub foreign_address: [u8; 32],
}

impl OrderStatic {
    pub fn get_order_id(&self) -> [u8; 16] {
        blake2_128(&[self.seller.encode(), self.asset_id.encode(), self.price.to_ne_bytes().to_vec(), self.foreign_address.encode()].concat())
    }
}

fn array_to_vec(arr: &[u8]) -> Vec<u8> {
     let mut vector = Vec::new();
     for i in arr.iter() {
         vector.push(*i);
     }
     vector
}

fn vector_as_u8_32_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = vector[offset + i];
    }
    arr
}

fn vector_as_u8_32_array(vector: &Vec<u8>) -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = vector[i];
    }
    arr
}

fn vector_as_u8_20_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 20] {
    let mut arr = [0u8; 20];
    for i in 0..20 {
        arr[i] = vector[offset + i];
    }
    arr
}

fn vector_as_u8_16_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 16] {
    let mut arr = [0u8; 16];
    for i in 0..16 {
        arr[i] = vector[offset + i];
    }
    arr
}

fn vector_as_u8_16_array(vector: &Vec<u8>) -> [u8; 16] {
    let mut arr = [0u8; 16];
    for i in 0..16 {
        arr[i] = vector[i];
    }
    arr
}

fn vector_as_u8_8_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 8] {
    let mut arr = [0u8; 8];
    for i in 0..8 {
        arr[i] = vector[offset + i];
    }
    arr
}

fn vector_as_u8_8_array(vector: &Vec<u8>) -> [u8; 8] {
    let mut arr = [0u8; 8];
    for i in 0..8 {
        arr[i] = vector[i];
    }
    arr
}

#[derive(Debug)]
struct ValueOrderId {
    value: u128,
    order_id: [u8; 16],
}

impl ValueOrderId {
    fn serialize(&self) -> Vec<u8> {
        [array_to_vec(&self.value.to_be_bytes()), self.order_id.to_vec()].concat()
    }

    fn unserialize(vec: Vec<u8>) -> ValueOrderId {
        ValueOrderId {
            value: u128::from_be_bytes(vector_as_u8_16_array(&vec[0..16].to_vec())),
            order_id: vector_as_u8_16_array(&vec[16..32].to_vec()),
        }
    }
}

struct OrderIdValueHashedSecret {
    order_id: [u8; 16],
    value: u64,
    hashed_secret: [u8; 32],
}

impl OrderIdValueHashedSecret {
    fn serialize(&self) -> Vec<u8> {
        [self.order_id.to_vec(), array_to_vec(&self.value.to_be_bytes()), self.hashed_secret.to_vec()].concat()
    }

    fn unserialize(vec: Vec<u8>) -> OrderIdValueHashedSecret {
        OrderIdValueHashedSecret {
            order_id: vector_as_u8_16_array(&vec[0..16].to_vec()),
            value: u64::from_be_bytes(vector_as_u8_8_array(&vec[16..24].to_vec())),
            hashed_secret: vector_as_u8_32_array(&vec[24..56].to_vec()),
        }
    }
}

impl fmt::Debug for OrderIdValueHashedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderIdValueHashedSecret")
         .field("order_id", &hex::encode(&self.order_id))
         .field("value", &self.value)
         .field("hashed_secret", &hex::encode(&self.hashed_secret))
         .finish()
    }
}

#[tokio::main]
async fn main() {
    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    db_opts.create_missing_column_families(true);
    let path = "database";
    let cf1 = ColumnFamilyDescriptor::new("order_static", Options::default());
    let cf2 = ColumnFamilyDescriptor::new("order_value", Options::default());
    let cf3 = ColumnFamilyDescriptor::new("order_list", Options::default());
    let cf4 = ColumnFamilyDescriptor::new("buy_lock_list", Options::default());
    let db = DB::open_cf_descriptors(&db_opts, path, vec![cf1, cf2, cf3, cf4]).unwrap();
    let db = Arc::new(db);
    // Spawn websockets task.
    let websockets_task = tokio::spawn(websockets_listen(db.clone()));
    // Spawn Acuity task.
    let acuity_task = tokio::spawn(acuity_listen(db.clone()));
    // Spawn Ethereum task.
    let ethereum_task = tokio::spawn(ethereum_listen(db.clone()));
    // Wait to exit.
    let _result = join!(websockets_task, acuity_task, ethereum_task);
}

struct AcuityApi {
    client: Client::<AcuityRuntime>,
}

impl AcuityApi {

    async fn get_storage_data(
        &self,
        module_name: &str,
        storage_name: &str,
        header_hash: H256,
    ) -> Result<StorageData, &str> {
        let mut storage_key = twox_128(module_name.as_bytes()).to_vec();
        storage_key.extend(twox_128(storage_name.as_bytes()).to_vec());

        let keys = vec![StorageKey(storage_key)];

        let change_sets = self
            .client
            .query_storage(keys, header_hash, Some(header_hash))
            .await.unwrap();
        for change_set in change_sets {
            for (_key, data) in change_set.changes {
                if let Some(data) = data {
                    return Ok(data);
                }
            }
        }

        Err("Data not found.")
    }

    async fn get_storage_data_map(
        &self,
        module_name: &str,
        storage_name: &str,
        key: &[u8; 16],
    ) -> Result<u128, &str> {
        let mut storage_key = twox_128(module_name.as_bytes()).to_vec();
        storage_key.extend(twox_128(storage_name.as_bytes()).to_vec());
        storage_key.extend(blake2_128(&key.encode()).to_vec());
        storage_key.extend(key.encode());

        let data = self
        .client
        .fetch_unhashed(StorageKey(storage_key), None)
        .await.unwrap();

        if let Some(data) = data {
            return Ok(data);
        }

        Err("Data not found.")
    }
}

async fn update_order(order_id: [u8; 16], db: Arc<DB>, client: Client::<AcuityRuntime>) {
    println!("order_id: {:?}", order_id);
    let option = db.get_cf(&db.cf_handle("order_value").unwrap(), order_id).unwrap();
    println!("order_value: {:?}", option);

    match option {
        Some(result) => {
            let value = u128::from_be_bytes(vector_as_u8_16_array(&result));
            println!("old value: {:?}", value);
            let value_order_id = ValueOrderId {
                value: value,
                order_id: order_id,
            };
            // Remove order from list.
            db.delete_cf(&db.cf_handle("order_list").unwrap(), value_order_id.serialize()).unwrap();
        }
        None => {},
    }

    let api = AcuityApi {
        client: client.clone()
    };

    let option = api.get_storage_data_map("AtomicSwap", "AcuityOrderIdValues", &order_id).await;

    match option {
        Ok(new_value) => {
            println!("new value: {:?}", new_value);

            // Add order back into list.
            let value_order_id = ValueOrderId {
                value: new_value,
                order_id: order_id,
            };
            db.put_cf(&db.cf_handle("order_list").unwrap(), value_order_id.serialize(), order_id).unwrap();

            // Store new value
            db.put_cf(&db.cf_handle("order_value").unwrap(), order_id, new_value.to_be_bytes()).unwrap();
        }
        Err(err) => {
            db.delete_cf(&db.cf_handle("order_value").unwrap(), order_id).unwrap();
        },
    }

}

async fn acuity_listen(db: Arc<DB>) {
    let client = ClientBuilder::<AcuityRuntime>::new()
        .register_type_size::<[u8; 16]>("AcuityOrderId")
        .register_type_size::<[u8; 16]>("AcuityAssetId")
        .register_type_size::<[u8; 32]>("AcuityForeignAddress")
        .register_type_size::<[u8; 32]>("AcuityHashedSecret")
        .register_type_size::<[u8; 32]>("AcuitySecret")
        .register_type_size::<u64>("Timestamp")
        .register_type_size::<[u8; 20]>("EthereumAddress")
        .set_url("ws://127.0.0.1:9946").build().await.unwrap();

    let mut blocks = client.subscribe_blocks().await.unwrap();

    loop {
        let block = blocks.next().await.unwrap().unwrap();
        println!("Acuity block: {:?}", block.number);
        let block = client.block_hash(Some(block.number.into())).await.unwrap().unwrap();

        let sub = client.subscribe_events().await.unwrap();
        let decoder = client.events_decoder();
        let mut sub = EventSubscription::<AcuityRuntime>::new(sub, decoder);
        sub.filter_block(block);

        loop {
            let raw = sub.next().await;
            // Pattern match to retrieve the value
            match raw {
                Some(event) => {
                    let event = event.unwrap();
                    if event.module != "AtomicSwap" { continue; }

                    match event.variant.as_str() {
                        "AddToOrder" => {
                            let event = AddToOrderEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("AddToOrderEvent: {:?}", event);
                            let order = OrderStatic {
                                seller: event.seller,
                                asset_id: event.asset_id,
                                price: event.price,
                                foreign_address: event.foreign_address,
                            };
                            let order_id = order.get_order_id();
                            println!("order_id: {:?}", hex::encode(order_id));
                            db.put_cf(&db.cf_handle("order_static").unwrap(), order_id, bincode::serialize(&order).unwrap()).unwrap();
                            update_order(order_id, db.clone(), client.clone()).await;
                        },
                        "RemoveFromOrder" => {
                            let event = RemoveFromOrderEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("RemoveFromOrderEvent: {:?}", event);
                            let order = OrderStatic {
                                seller: event.seller,
                                asset_id: event.asset_id,
                                price: event.price,
                                foreign_address: event.foreign_address,
                            };
                            let order_id = order.get_order_id();
                            println!("order_id: {:?}", order_id);
                            update_order(order_id, db.clone(), client.clone()).await;
                        },
                        "LockSell" => {
                            let event = LockSellEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("LockSellEvent: {:?}", event);
                        },
                        "UnlockSell" => {
                            let event = UnlockSellEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("UnlockSellEvent: {:?}", event);
                        },
                        "TimeoutSell" => {
                            let event = TimeoutSellEvent::decode(&mut &event.data[..]).unwrap();
                            println!("TimeoutSellEvent: {:?}", event);
                        },
                        "LockBuy" => {
                            let event = LockBuyEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                            println!("LockBuyEvent: {:?}", event);
                        },
                        "UnlockBuy" => {
                            let event = UnlockBuyEvent::decode(&mut &event.data[..]).unwrap();
                            println!("UnlockBuyEvent: {:?}", event);
                        },
                        "TimeoutBuy" => {
                            let event = TimeoutBuyEvent::decode(&mut &event.data[..]).unwrap();
                            println!("TimeoutBuyEvent: {:?}", event);
                        },
                        _ => println!("variant: {:?}", event.variant),
                    }
                },
                None => break,
            }
        }
    }
}

async fn ethereum_listen(db: Arc<DB>) {
//    let ws = web3::transports::WebSocket::new("wss://mainnet.infura.io/ws/v3/9aa3d95b3bc440fa88ea12eaa4456161").await.unwrap();
    let ws = web3::transports::WebSocket::new("ws:/127.0.0.1:8546").await.unwrap();
    let web3 = web3::Web3::new(ws);

    let sell_addr = Address::from_str("0xd05647dd9D7B17aBEBa953fbF2dc8D8e87c19cb3").unwrap();
    let sell_contract = Contract::from_json(web3.eth(), sell_addr, include_bytes!("AcuityAtomicSwapSell.abi")).unwrap();

    let add_to_order = sell_contract.abi().event("AddToOrder").unwrap().signature();
    let remove_from_order = sell_contract.abi().event("RemoveFromOrder").unwrap().signature();

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
                            let value = U64::from(vector_as_u8_8_array_offset(&event.data.0, 120)).as_u64();
                            let timeout = U64::from(vector_as_u8_8_array_offset(&event.data.0, 152)).as_u64();
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

#[derive(Serialize, Deserialize, Debug)]
struct BuyLock {
    hashed_secret: String,
    value: u64,
    timeout: u64,
    buyer: String,
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
              _ => {}
          }

      } else if msg.is_close() {
          break;
      }
  }

//    println!("{} disconnected", &addr);
}

async fn websockets_listen(db: Arc<DB>) {
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
