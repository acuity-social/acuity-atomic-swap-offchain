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

#[derive(Debug)]
pub struct ValueOrderId {
    pub value: u128,
    pub order_id: [u8; 16],
}

impl ValueOrderId {
    pub fn serialize(&self) -> Vec<u8> {
        [array_to_vec(&self.value.to_be_bytes()), self.order_id.to_vec()].concat()
    }

    pub fn unserialize(vec: Vec<u8>) -> ValueOrderId {
        ValueOrderId {
            value: u128::from_be_bytes(vector_as_u8_16_array(&vec[0..16].to_vec())),
            order_id: vector_as_u8_16_array(&vec[16..32].to_vec()),
        }
    }
}

pub struct OrderIdValueHashedSecret {
    pub order_id: [u8; 16],
    pub value: u128,
    pub hashed_secret: [u8; 32],
}

impl OrderIdValueHashedSecret {
    pub fn serialize(&self) -> Vec<u8> {
        [self.order_id.to_vec(), array_to_vec(&self.value.to_be_bytes()), self.hashed_secret.to_vec()].concat()
    }

    pub fn unserialize(vec: Vec<u8>) -> OrderIdValueHashedSecret {
        OrderIdValueHashedSecret {
            order_id: vector_as_u8_16_array(&vec[0..16].to_vec()),
            value: u128::from_be_bytes(vector_as_u8_16_array(&vec[16..32].to_vec())),
            hashed_secret: vector_as_u8_32_array(&vec[32..64].to_vec()),
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

#[derive(Serialize, Deserialize, Debug)]
pub struct BuyLock {
    pub hashed_secret: [u8; 32],
    pub value: u128,
    pub timeout: u128,
    pub buyer: [u8; 20],
}

#[derive(Display, Serialize, Deserialize, Debug)]
pub enum LockState {
    NotLocked,
    Locked,
    Unlocked,
    TimedOut,
    Invalid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SellLock {
    pub state: LockState,
    pub timeout: u128,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum RequestMessage {
    GetOrderBook,
    GetOrder {
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

/*
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
*/
