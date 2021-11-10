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
use sp_io::hashing::{blake2_128, keccak_256};
use sp_core::storage::StorageKey;
use sp_core::twox_128;
use codec::{
    Codec,
    Decode,
    Encode,
};
use proc_macro::*;
use serde::{Serialize, Deserialize};
use std::fmt::Debug;
use std::{
    sync::Arc,
};
use rocksdb::{DB};
use tokio::sync::broadcast::Sender;
use scale_info::TypeInfo;

use crate::shared::*;

#[derive(Debug, Clone, Eq, PartialEq, TypeInfo, Serialize, Deserialize)]
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

struct AcuityApi {
    client: Client::<AcuityRuntime>,
}

impl AcuityApi {
/*
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
*/
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
    pub order_id: [u8; 16],
    pub hashed_secret: [u8; 32],
    pub timeout: T::Moment,
    pub value: T::Balance,
}

/// UnlockSell event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct UnlockSellEvent<T: AtomicSwap> {
    pub order_id: [u8; 16],
    pub secret: [u8; 32],
    pub buyer: <T as System>::AccountId,
}

/// TimeoutSell event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct TimeoutSellEvent {
    pub order_id: [u8; 16],
    pub hashed_secret: [u8; 32],
}

/// LockBuy event.
#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct LockBuyEvent<T: AtomicSwap> {
    pub buyer: <T as System>::AccountId,
    pub seller: <T as System>::AccountId,
    pub hashed_secret: [u8; 32],
    pub timeout: T::Moment,
    pub value: T::Balance,
    pub asset_id: [u8; 16],
    pub order_id: [u8; 16],
}

/// UnlockBuy event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct UnlockBuyEvent<T: AtomicSwap> {
    pub buyer: <T as System>::AccountId,
    pub hashed_secret: [u8; 32],
}

/// TimeoutBuy event.
#[derive(Debug, Decode, Eq, PartialEq)]
pub struct TimeoutBuyEvent<T: AtomicSwap> {
    pub buyer: <T as System>::AccountId,
    pub hashed_secret: [u8; 32],
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
        Err(_err) => {
            db.delete_cf(&db.cf_handle("order_value").unwrap(), order_id).unwrap();
        },
    }

}

pub async fn acuity_listen(db: Arc<DB>, tx: Sender<RequestMessage>) {
    let client = ClientBuilder::<AcuityRuntime>::new()
        .register_type_size::<[u8; 32]>("T::AccountId")
        .register_type_size::<[u8; 32]>("<T as frame_system::Config>::AccountId")
        .register_type_size::<u128>("T::Balance")
        .register_type_size::<u128>("BalanceOf<T>")
        .register_type_size::<u128>("BalanceOf<T, I>")
        .register_type_size::<u64>("T::Moment")
        .register_type_size::<[u8; 16]>("AcuityOrderId")
        .register_type_size::<[u8; 16]>("AcuityAssetId")
        .register_type_size::<[u8; 32]>("AcuityForeignAddress")
        .register_type_size::<[u8; 32]>("AcuityHashedSecret")
        .register_type_size::<[u8; 32]>("AcuitySecret")
        .register_type_size::<u64>("Timestamp")
        .register_type_size::<[u8; 20]>("EthereumAddress")
        .set_url("ws://127.0.0.1:9946")
        .skip_type_sizes_check()
        .build().await.unwrap();

    let sub = client.subscribe_events().await.unwrap();
    let decoder = client.events_decoder();
    let mut sub = EventSubscription::<AcuityRuntime>::new(sub, decoder);

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
                        tx.send(RequestMessage::GetOrderBook).unwrap();
                        tx.send(RequestMessage::GetOrder { order_id: hex::encode(order_id) } ).unwrap();
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
                        let sell_lock = SellLock {
                            state: LockState::Locked,
                            timeout: event.timeout.into(),
                            secret: None,
                        };
                        db.put_cf(&db.cf_handle("sell_lock").unwrap(), event.hashed_secret, bincode::serialize(&sell_lock).unwrap()).unwrap();
                        update_order(event.order_id, db.clone(), client.clone()).await;
                        tx.send(RequestMessage::GetOrder { order_id: hex::encode(event.order_id) } ).unwrap();
                    },
                    "UnlockSell" => {
                        let event = UnlockSellEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                        println!("UnlockSellEvent: {:?}", event);
                        let hashed_secret = keccak_256(&event.secret);

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
                        sell_lock.secret = Some(event.secret);
                        db.put_cf(&db.cf_handle("sell_lock").unwrap(), hashed_secret, bincode::serialize(&sell_lock).unwrap()).unwrap();
                        tx.send(RequestMessage::GetOrder { order_id: hex::encode(event.order_id) } ).unwrap();
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
                        let event = UnlockBuyEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                        println!("UnlockBuyEvent: {:?}", event);
                    },
                    "TimeoutBuy" => {
                        let event = TimeoutBuyEvent::<AcuityRuntime>::decode(&mut &event.data[..]).unwrap();
                        println!("TimeoutBuyEvent: {:?}", event);
                    },
                    _ => println!("variant: {:?}", event.variant),
                }
            },
            None => break,
        }
    }
}
