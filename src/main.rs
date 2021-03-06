use tokio::join;
use tokio::sync::broadcast;
use rocksdb::{DB, ColumnFamilyDescriptor, Options};
use std::sync::Arc;

mod shared;
mod websockets;
mod acuity;
mod ethereum;
mod arbitrum;

use websockets::websockets_listen;
use acuity::acuity_listen;
use ethereum::ethereum_listen;
use arbitrum::arbitrum_listen;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    let mut db_opts = Options::default();
    db_opts.create_if_missing(true);
    db_opts.create_missing_column_families(true);
    let path = "database";
    let cf1 = ColumnFamilyDescriptor::new("order_static", Options::default());
    let cf2 = ColumnFamilyDescriptor::new("order_value", Options::default());
    let cf3 = ColumnFamilyDescriptor::new("order_list", Options::default());
    let cf4 = ColumnFamilyDescriptor::new("order_lock_list", Options::default());
    let cf5 = ColumnFamilyDescriptor::new("buy_lock", Options::default());
    let cf6 = ColumnFamilyDescriptor::new("sell_lock", Options::default());
    let db = DB::open_cf_descriptors(&db_opts, path, vec![cf1, cf2, cf3, cf4, cf5, cf6]).unwrap();
    let db = Arc::new(db);
    let (tx, _rx) = broadcast::channel(16);
    // Spawn Acuity task.
    let acuity_task = tokio::spawn(acuity_listen(db.clone(), tx.clone()));
    // Spawn Ethereum task.
    let ethereum_task = tokio::spawn(ethereum_listen(db.clone(), tx.clone()));
    // Spawn Ethereum task.
    let arbitrum_task = tokio::spawn(arbitrum_listen(db.clone(), tx.clone()));
    // Spawn websockets task.
    let websockets_task = tokio::spawn(websockets_listen(db.clone(), tx));
    // Wait to exit.
    let _result = join!(websockets_task, acuity_task, arbitrum_task, ethereum_task);
}
