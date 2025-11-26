use crate::check_handler::setup_check_hooks;
use crate::connection_manager::CONNECTION_STATUS;
use crate::constants::{
    get_items_by_category, BasicNothingFunc, ItemCategory, INITIAL_HP, INITIAL_MAGIC, MAX_HP, MAX_MAGIC,
    MISSION_ITEM_MAP,
};
use crate::game_manager::{
    with_active_player_data, with_session, with_session_read,
    ARCHIPELAGO_DATA, CHANGE_EQUIPPED_GUN, CHANGE_EQUIPPED_MELEE,
};
use crate::mapping::MAPPING;
use crate::utilities::DMC1_ADDRESS;
use crate::{
    check_handler, constants, create_hook, skill_manager, utilities,
};
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
        create_hook!(
            SETUP_NEW_SESSION_ADDR,
            setup_new_session_data,
            ORIGINAL_SETUP_NEW_SESSION,
            "Setup new session data"
        );
    }
    Ok(())
}

fn enable_hooks() {
    let addresses: Vec<usize> = vec![
        check_handler::ITEM_PICKUP_ADDR,
        LOAD_ROOM_ADDR,
        check_handler::ADD_ITEM_ADDR,
        check_handler::DISPLAY_ITEMS_ADDR,
        SETUP_NEW_SESSION_ADDR,
    ];
    addresses.iter().for_each(|addr| unsafe {
        match MinHook::enable_hook((*DMC1_ADDRESS + addr) as *mut _) {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to enable {:#X} hook: {:?}", addr, err);
            }
        }
    })
}

// 0x3c8600 - I think this is maybe just inventory stuff?
const SETUP_NEW_SESSION_ADDR: usize = 0x2c3b30;
static ORIGINAL_SETUP_NEW_SESSION: OnceLock<BasicNothingFunc> = OnceLock::new();
pub fn setup_new_session_data() {
    if let Some(func) = ORIGINAL_SETUP_NEW_SESSION.get() {
        unsafe { func() }
    }
    set_max_hp_and_magic();
    set_weapons_in_inv();
    set_equipment();
    with_active_player_data(|d| {
        with_session_read(|s| {
            d.hp = s.hp as u16 * 100;
            d.magic_human = s.magic as u16 * 120;
            d.magic_demon = s.magic as u16 * 200;
        })
        .unwrap();
    })
    .unwrap();
}

const LOAD_ROOM_ADDR: usize = 0x255cc0;
static ORIGINAL_LOAD_ROOM: OnceLock<BasicNothingFunc> = OnceLock::new();

// So everything will be appropriately set when loading into a mission
fn load_room() {
    log::info!("Loading room!");
    if let Some(func) = ORIGINAL_LOAD_ROOM.get() {
        unsafe { func() }
    }
    set_max_hp_and_magic();
    set_weapons_in_inv();
    set_equipment();
    set_relevant_key_items();
    skill_manager::set_skills(&ARCHIPELAGO_DATA.read().unwrap());
}

fn set_max_hp_and_magic() {
    with_session(|s| {
        match ARCHIPELAGO_DATA.read() {
            Ok(data) => {
                s.hp = u8::min(INITIAL_HP + data.blue_orbs as u8, MAX_HP);
                // TODO DT Unlock option
                if data.dt_unlocked {
                    s.magic = u8::min(data.purple_orbs as u8, MAX_MAGIC);
                } else {
                    s.magic = INITIAL_MAGIC
                }
            }
            Err(err) => {
                log::error!("Failed to read data from ARCHIPELAGO_DATA: {}", err);
            }
        }
    })
    .unwrap();
}

fn set_equipment() {
    let data = ARCHIPELAGO_DATA.read().unwrap();
    if let Some(mapping) = MAPPING.read().unwrap().as_ref() {
        with_active_player_data(|d| {
            if !data
                .items
                .contains(constants::GUN_MAP.get_by_right(&d.gun).unwrap())
            {
                // Set the actor data and make sure to update the equipped gun, otherwise weirdness happens (I.e double wielding shotguns)
                d.gun = *constants::GUN_MAP
                    .get_by_left(mapping.start_gun.as_str())
                    .unwrap();
                CHANGE_EQUIPPED_GUN(d.gun as u32);
            }
            if !data
                .items
                .contains(constants::MELEE_MAP.get_by_right(&d.melee).unwrap())
            {
                // Set actor data then update melee
                d.melee = *constants::MELEE_MAP
                .get_by_left(mapping.start_melee.as_str())
                .unwrap();
                CHANGE_EQUIPPED_MELEE(d.melee as u32, 0);
            }
        })
        .unwrap();
    }
}

fn set_weapons_in_inv() {
    if let Ok(data) = ARCHIPELAGO_DATA.read() {
        for weapon in get_items_by_category(ItemCategory::Weapon) {
            if data.items.contains(&weapon) {
                let wep = constants::ITEM_DATA_MAP.get(weapon).unwrap();
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

/// False if it isn't, true if it is
pub fn is_item_relevant_to_mission(item_name: &str) -> bool {
    if get_items_by_category(ItemCategory::Consumable).contains(&item_name) {
        return true;
    }
    let mut res = false;
    with_session(|s| {
        match MISSION_ITEM_MAP.get(&(s.mission as u32)) {
            None => {} // No items for the mission
            Some(item_list) => {
                for item in item_list.iter() {
                    if item.eq(&item_name) {
                        res = true;
                    }
                }
            }
        }
    })
    .unwrap();
    res
}
