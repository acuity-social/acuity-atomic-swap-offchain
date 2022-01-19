use std::fmt;
use codec::{
    Decode,
    Encode,
};
use serde::{Serialize, Deserialize};
use bincode::Options;
use sp_io::hashing::blake2_128;
use strum_macros::Display;

#[derive(Serialize)]
pub struct OrderKey {
    pub chain_id: u32,      // selling chain
    pub adapter_id: u32,    // selling adapter
    pub order_id: [u8; 16],
}

impl OrderKey {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::options().with_big_endian().with_fixint_encoding().serialize(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
//#[derive(Debug)]
pub struct OrderListKey {
    pub sell_chain_id: u32,
    pub sell_asset_id: [u8; 8],
    pub buy_chain_id: u32,
    pub buy_asset_id: [u8; 8],
    pub value: u128,

    pub sell_adapter_id: u32,
    pub order_id: [u8; 16],
}

impl OrderListKey {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::options().with_big_endian().with_fixint_encoding().serialize(&self).unwrap()
    }

    pub fn unserialize(vec: Vec<u8>) -> OrderListKey {
        bincode::options().with_big_endian().with_fixint_encoding().deserialize(&vec).unwrap()
    }
}

#[derive(Serialize, Deserialize)]
pub struct OrderLockListKey {
    pub chain_id: u32,      // selling chain
    pub adapter_id: u32,    // selling adapter
    pub order_id: [u8; 16],
    pub value: u128,

    pub hashed_secret: [u8; 32],
}

impl OrderLockListKey {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::options().with_big_endian().with_fixint_encoding().serialize(&self).unwrap()
    }

    pub fn unserialize(vec: Vec<u8>) -> OrderLockListKey {
        bincode::options().with_big_endian().with_fixint_encoding().deserialize(&vec).unwrap()
    }
}

impl fmt::Debug for OrderLockListKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrderIdValueHashedSecret")
         .field("order_id", &hex::encode(&self.order_id))
         .field("value", &self.value)
         .field("hashed_secret", &hex::encode(&self.hashed_secret))
         .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, Default, Serialize, Deserialize)]
pub struct OrderStatic {
    pub seller: [u8; 32],
    pub chain_id: u32,          // buying chain
    pub adapter_id: u32,        // buying adapter
    pub asset_id: [u8; 8],      // buying asset
    pub price: u128,
    pub foreign_address: [u8; 32],
}

impl OrderStatic {
    pub fn get_order_id(&self) -> [u8; 16] {
        blake2_128(&[self.seller.encode(), self.chain_id.encode(), self.adapter_id.encode(), self.asset_id.encode(), self.price.to_ne_bytes().to_vec(), self.foreign_address.encode()].concat())
    }
}

#[derive(Display, Serialize, Deserialize, Debug)]
pub enum LockState {
    NotLocked,
    Locked,
    Unlocked,
    TimedOut,
    Invalid,
}

#[derive(Serialize)]
pub struct LockKey {
    pub chain_id: u32,      // selling chain
    pub adapter_id: u32,    // selling adapter
    pub hashed_secret: [u8; 32],
}

impl LockKey {
    pub fn serialize(&self) -> Vec<u8> {
        bincode::options().with_big_endian().with_fixint_encoding().serialize(&self).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BuyLock {
    pub order_id: [u8; 16],
    pub value: u128,
    pub timeout: u128,
    pub buyer: [u8; 32],
    pub foreign_address: [u8; 32],
    pub state: LockState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SellLock {
    pub state: LockState,
    pub timeout: u128,
    pub secret: Option<[u8; 32]>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum RequestMessage {
    GetOrderBook {
        sell_chain_id: u32,
        sell_asset_id: String,
        buy_chain_id: u32,
        buy_asset_id: String,
    },
    GetOrder {
        sell_chain_id: u32,
        sell_adapter_id: u32,
        order_id: String,
    },
}

pub fn array_to_vec(arr: &[u8]) -> Vec<u8> {
     let mut vector = Vec::new();
     for i in arr.iter() {
         vector.push(*i);
     }
     vector
}

pub fn vector_as_u8_32_array(vector: &Vec<u8>) -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = vector[i];
    }
    arr
}

pub fn vector_as_u8_16_array(vector: &Vec<u8>) -> [u8; 16] {
    let mut arr = [0u8; 16];
    for i in 0..16 {
        arr[i] = vector[i];
    }
    arr
}

pub fn vector_as_u8_8_array(vector: &Vec<u8>) -> [u8; 8] {
    let mut arr = [0u8; 8];
    for i in 0..8 {
        arr[i] = vector[i];
    }
    arr
}

pub fn vector_as_u8_4_array(vector: &Vec<u8>) -> [u8; 4] {
    let mut arr = [0u8; 4];
    for i in 0..4 {
        arr[i] = vector[i];
    }
    arr
}
