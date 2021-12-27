use std::fmt;
use codec::{
    Decode,
    Encode,
};
use serde::{Serialize, Deserialize};
use sp_runtime::{
    traits::{
        IdentifyAccount,
        Verify,
    },
    MultiSignature,
};
use sp_io::hashing::blake2_128;
use strum_macros::Display;

pub struct OrderKey {
    pub chain_id: u32,
    pub adapter_id: u32,
    pub order_id: [u8; 16],
}

impl OrderKey {
    pub fn serialize(&self) -> Vec<u8> {
        [
            array_to_vec(&self.chain_id.to_be_bytes()),
            array_to_vec(&self.adapter_id.to_be_bytes()),
            self.order_id.to_vec(),
        ].concat()
    }
}

#[derive(Debug)]
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
        [
            array_to_vec(&self.sell_chain_id.to_be_bytes()),
            self.sell_asset_id.to_vec(),
            array_to_vec(&self.buy_chain_id.to_be_bytes()),
            self.buy_asset_id.to_vec(),
            array_to_vec(&self.value.to_be_bytes()),
            array_to_vec(&self.sell_adapter_id.to_be_bytes()),
            self.order_id.to_vec(),
        ].concat()
    }

    pub fn unserialize(vec: Vec<u8>) -> OrderListKey {
        OrderListKey {
            sell_chain_id: u32::from_be_bytes(vector_as_u8_4_array(&vec[0..4].to_vec())),
            sell_asset_id: vector_as_u8_8_array(&vec[4..12].to_vec()),
            buy_chain_id: u32::from_be_bytes(vector_as_u8_4_array(&vec[12..16].to_vec())),
            buy_asset_id: vector_as_u8_8_array(&vec[16..24].to_vec()),
            value: u128::from_be_bytes(vector_as_u8_16_array(&vec[24..40].to_vec())),
            sell_adapter_id: u32::from_be_bytes(vector_as_u8_4_array(&vec[40..44].to_vec())),
            order_id: vector_as_u8_16_array(&vec[44..60].to_vec()),
        }
    }
}

pub struct OrderLockListKey {
    pub chain_id: u32,
    pub adapter_id: u32,
    pub order_id: [u8; 16],
    pub value: u128,
    pub hashed_secret: [u8; 32],
}

impl OrderLockListKey {
    pub fn serialize(&self) -> Vec<u8> {
        [
            array_to_vec(&self.chain_id.to_be_bytes()),
            array_to_vec(&self.adapter_id.to_be_bytes()),
            self.order_id.to_vec(),
            array_to_vec(&self.value.to_be_bytes()),
            self.hashed_secret.to_vec(),
        ].concat()
    }

    pub fn unserialize(vec: Vec<u8>) -> OrderLockListKey {
        OrderLockListKey {
            chain_id: u32::from_be_bytes(vector_as_u8_4_array(&vec[0..4].to_vec())),
            adapter_id: u32::from_be_bytes(vector_as_u8_4_array(&vec[4..8].to_vec())),
            order_id: vector_as_u8_16_array(&vec[8..24].to_vec()),
            value: u128::from_be_bytes(vector_as_u8_16_array(&vec[24..40].to_vec())),
            hashed_secret: vector_as_u8_32_array(&vec[72..104].to_vec()),
        }
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

#[derive(Display, Serialize, Deserialize, Debug)]
pub enum LockState {
    NotLocked,
    Locked,
    Unlocked,
    TimedOut,
    Invalid,
}

pub struct LockKey {
    pub chain_id: u32,
    pub adapter_id: u32,
    pub hashed_secret: [u8; 32],
}

impl LockKey {
    pub fn serialize(&self) -> Vec<u8> {
        [
            array_to_vec(&self.chain_id.to_be_bytes()),
            array_to_vec(&self.adapter_id.to_be_bytes()),
            self.hashed_secret.to_vec(),
        ].concat()
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

pub fn vector_as_u8_32_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = vector[offset + i];
    }
    arr
}

pub fn vector_as_u8_32_array(vector: &Vec<u8>) -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = vector[i];
    }
    arr
}

pub fn vector_as_u8_20_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 20] {
    let mut arr = [0u8; 20];
    for i in 0..20 {
        arr[i] = vector[offset + i];
    }
    arr
}

pub fn vector_as_u8_16_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 16] {
    let mut arr = [0u8; 16];
    for i in 0..16 {
        arr[i] = vector[offset + i];
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


pub fn vector_as_u8_8_array_offset(vector: &Vec<u8>, offset: usize) -> [u8; 8] {
    let mut arr = [0u8; 8];
    for i in 0..8 {
        arr[i] = vector[offset + i];
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
