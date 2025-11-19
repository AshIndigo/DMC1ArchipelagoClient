use crate::constants::ItemCategory;
use archipelago_rs::client::{ArchipelagoClient, ArchipelagoError};
use archipelago_rs::protocol::{ClientMessage, DataStorageOperation, Get, Set};
use minhook::{MinHook, MH_STATUS};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::{OnceLock, RwLock};
use tokio::sync::mpsc::{Sender};
use randomizer_utilities::archipelago_utilities::{SLOT_NUMBER, TEAM_NUMBER};
use crate::constants;

static BANK: OnceLock<RwLock<HashMap<&'static str, i32>>> = OnceLock::new();
pub static TX_BANK_MESSAGE: OnceLock<Sender<(&'static str, i32)>> = OnceLock::new();

pub(crate) fn get_bank_key(item: &str) -> String {
    format!(
        "team{}_slot{}_{}",
        TEAM_NUMBER.load(Ordering::SeqCst),
        SLOT_NUMBER.load(Ordering::SeqCst),
        item
    )
}

pub fn get_bank() -> &'static RwLock<HashMap<&'static str, i32>> {
    BANK.get_or_init(|| {
        RwLock::new(
            constants::get_items_by_category(ItemCategory::Consumable)
                .iter()
                .map(|name| (*name, 0))
                .collect(),
        )
    })
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
pub async fn modify_bank_message(item_name: &'static str, count: i32) {
    match TX_BANK_MESSAGE.get() {
        None => log::error!("Connect TX doesn't exist"),
        Some(tx) => {
            tx.send((item_name, count))
                .await
                .expect("Failed to send data");
        }
    }
}

pub(crate) async fn modify_bank_value(
    client: &mut ArchipelagoClient,
    item: (&'static str, i32),
) -> Result<(), ArchipelagoError> {
    client
        .send(ClientMessage::Set(Set {
            key: get_bank_key(item.0),
            default: Value::from(1),
            want_reply: true,
            operations: vec![DataStorageOperation::Add(Value::from(item.1))],
        }))
        .await
}

/// Reset the banks contents to nothing. Used for resetting the values if needed.
pub(crate) async fn _reset_bank(
    client: &mut ArchipelagoClient,
) -> Result<(), Box<dyn std::error::Error>> {
    get_bank().write()?.iter_mut().for_each(|(_k, v)| {
        *v = 0; // Set each bank item in the map to 0
    });
    for item in constants::get_items_by_category(ItemCategory::Consumable) {
        client
            .send(ClientMessage::Set(Set {
                key: get_bank_key(item),
                default: Value::from(0),
                want_reply: true,
                operations: vec![DataStorageOperation::Default],
            }))
            .await?;
    }
    Ok(())
}

pub(crate) async fn read_values(
    client: &mut ArchipelagoClient,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut keys = vec![];
    for item in constants::get_items_by_category(ItemCategory::Consumable) {
        keys.push(get_bank_key(item));
    }
    client.send(ClientMessage::Get(Get { keys })).await?;
    Ok(())
}