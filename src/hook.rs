use crate::check_handler::setup_check_hooks;
use crate::connection_manager::CONNECTION_STATUS;
use crate::constants::{get_items_by_category, BasicNothingFunc, ItemCategory, MISSION_ITEM_MAP};
use crate::game_manager::{with_session, ARCHIPELAGO_DATA};
use crate::utilities::DMC1_ADDRESS;
use crate::{check_handler, constants, create_hook, skill_manager, utilities};
use minhook::{MinHook, MH_STATUS};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static HOOKS_CREATED: AtomicBool = AtomicBool::new(false);

pub(crate) fn install_initial_functions() {
    if !HOOKS_CREATED.load(Ordering::SeqCst) {
        unsafe {
            match create_hooks() {
                Ok(_) => {
                    HOOKS_CREATED.store(true, Ordering::SeqCst);
                }
                Err(err) => {
                    log::error!("Failed to create hooks: {:?}", err);
                }
            }
        }
    }
    enable_hooks();
}

unsafe fn create_hooks() -> Result<(), MH_STATUS> {
    setup_check_hooks()?;
    unsafe {
        create_hook!(
            LOAD_ROOM_ADDR,
            load_room,
            ORIGINAL_LOAD_ROOM,
            "Non event item"
        );
    }
    Ok(())
}

fn enable_hooks() {
    let addresses: Vec<usize> = vec![check_handler::ITEM_PICKUP_ADDR, LOAD_ROOM_ADDR];
    addresses.iter().for_each(|addr| unsafe {
        match MinHook::enable_hook((*DMC1_ADDRESS + addr) as *mut _) {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to enable {:#X} hook: {:?}", addr, err);
            }
        }
    })
}

const LOAD_ROOM_ADDR: usize = 0x255cc0;
static ORIGINAL_LOAD_ROOM: OnceLock<BasicNothingFunc> = OnceLock::new();

// So everything will be appropriately set when loading into a mission
fn load_room() {
    log::info!("Loading room!");
    if let Some(func) = ORIGINAL_LOAD_ROOM.get() {
        unsafe { func() }
    }
    set_weapons_in_inv();
    set_relevant_key_items();
    skill_manager::set_skills(&ARCHIPELAGO_DATA.read().unwrap());
}

fn set_weapons_in_inv() {
    if let Ok(data) = ARCHIPELAGO_DATA.read() {
        for weapon in get_items_by_category(ItemCategory::Weapon) {
            if data.items.contains(&weapon) {
                let wep = constants::ITEM_DATA_MAP.get(weapon).unwrap();
                log::debug!("Adding this wep to inv {wep}");
                utilities::insert_unique_item_into_inv(wep);
                log::debug!("Adding weapon to inventory {}", weapon);
            } else {
                utilities::clear_item_slot(constants::ITEM_DATA_MAP.get(weapon).unwrap());
            }
        }
    }
}

fn set_relevant_key_items() {
    if CONNECTION_STATUS.load(Ordering::Relaxed) != 1 {
        return;
    }

    if let Ok(data) = ARCHIPELAGO_DATA.read() {
        with_session(|s| {
            match MISSION_ITEM_MAP.get(&(s.mission as u32)) {
                None => {} // No items for the mission
                Some(item_list) => {
                    for item in item_list.iter() {
                        if data.items.contains(item) {
                            utilities::insert_unique_item_into_inv(
                                constants::ITEM_DATA_MAP.get(item).unwrap(),
                            );
                            log::debug!("Item relevant to mission #{} - {}", s.mission, *item);
                        } else {
                            utilities::clear_item_slot(constants::ITEM_DATA_MAP.get(item).unwrap());
                        }
                    }
                }
            }
        })
        .unwrap();
    }
}
