use crate::archipelago::CONNECTED;
use crate::check_handler::setup_check_hooks;
use crate::constants::{
    BasicNothingFunc, INITIAL_HP, INITIAL_MAGIC, ItemCategory, MAX_HP, MAX_MAGIC, MISSION_ITEM_MAP,
    get_items_by_category,
};
use crate::game_manager::{
    ARCHIPELAGO_DATA, CHANGE_EQUIPPED_GUN, UPDATE_WEAPONS, get_room, get_track,
    with_active_player_data, with_session, with_session_read,
};
use crate::save_handler::setup_save_hooks;
use crate::ui::text_handler;
use crate::ui::text_handler::ORIGINAL_DRAW_TEXT;
use crate::utilities::DMC1_ADDRESS;
use crate::{check_handler, constants, create_hook, save_handler, skill_manager, utilities};
use minhook::{MH_STATUS, MinHook};
use randomizer_utilities::{read_data_from_address, replace_single_byte};
use std::ptr::write;
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, OnceLock};

pub(crate) unsafe fn create_hooks() -> Result<(), MH_STATUS> {
    setup_check_hooks()?;
    setup_save_hooks()?;
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
        create_hook!(
            text_handler::DRAW_TEXT_ADDR,
            text_handler::draw_text_hook,
            ORIGINAL_DRAW_TEXT,
            "Draw text"
        );
        create_hook!(
            SET_PLAYER_DATA_ADRR,
            set_player_data,
            ORIGINAL_SET_PLAYER_DATA,
            "Set Player Data"
        );
    }
    Ok(())
}

static HOOK_ADDRESSES: LazyLock<Vec<usize>> = LazyLock::new(|| {
    let mut addrs = vec![
        LOAD_ROOM_ADDR,
        SETUP_NEW_SESSION_ADDR,
        SET_PLAYER_DATA_ADRR,
        text_handler::DRAW_TEXT_ADDR,
    ];
    check_handler::add_hooks_to_list(&mut addrs);
    save_handler::add_hooks_to_list(&mut addrs);
    addrs
});

pub fn disable_hooks() -> Result<(), MH_STATUS> {
    unsafe {
        for addr in HOOK_ADDRESSES.iter() {
            MinHook::disable_hook((*DMC1_ADDRESS + addr) as *mut _)?;
        }
    }
    Ok(())
}

pub(crate) fn enable_hooks() {
    for addr in HOOK_ADDRESSES.iter() {
        match unsafe { MinHook::enable_hook((*DMC1_ADDRESS + addr) as *mut _) } {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to enable {:#X} hook: {:?}", addr, err);
            }
        }
    }
}

const SET_PLAYER_DATA_ADRR: usize = 0x2c5590;
static ORIGINAL_SET_PLAYER_DATA: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();
pub fn set_player_data(player_data: usize) {
    if let Some(func) = ORIGINAL_SET_PLAYER_DATA.get() {
        unsafe { func(player_data) }
    }
    set_equipment();
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
    set_max_hp_and_magic();
    if let Some(func) = ORIGINAL_LOAD_ROOM.get() {
        unsafe { func() }
    }
    set_weapons_in_inv();
    if get_room() == 33 && get_track() == 1 {
        set_equipment();
    }
    set_relevant_key_items();
    skill_manager::set_skills(&ARCHIPELAGO_DATA.read().unwrap());
}

fn set_max_hp_and_magic() {
    match ARCHIPELAGO_DATA.read() {
        Ok(data) => {
            with_session(|s| {
                s.hp = u8::min(INITIAL_HP + data.blue_orbs as u8, MAX_HP);
                // TODO DT Unlock option
                if data.dt_unlocked {
                    s.magic = u8::min(data.purple_orbs as u8, MAX_MAGIC);
                } else {
                    s.magic = INITIAL_MAGIC
                }
            })
            .unwrap();
            let something = read_data_from_address::<usize>(*DMC1_ADDRESS + 0x60b0d8);
            let something_hp = something + 0x98;
            unsafe {
                write(something_hp as *mut u8, INITIAL_HP + data.blue_orbs as u8);
            }
            let something_magic = something + 0xA3;
            unsafe {
                write(
                    something_magic as *mut u8,
                    INITIAL_MAGIC + data.purple_orbs as u8,
                );
            }
            with_active_player_data(|d| {
                d.max_hp = u8::min(INITIAL_HP + data.blue_orbs as u8, MAX_HP) as u16 * 100;
                if data.dt_unlocked {
                    d.max_magic_human = u8::min(data.purple_orbs as u8, MAX_MAGIC) as u16 * 120;
                    d.max_magic_demon = u8::min(data.purple_orbs as u8, MAX_MAGIC) as u16 * 200;
                } else {
                    d.max_magic_human = INITIAL_MAGIC as u16;
                    d.max_magic_demon = INITIAL_MAGIC as u16;
                }
            })
            .unwrap();
        }
        Err(err) => {
            log::error!("Failed to read data from ARCHIPELAGO_DATA: {}", err);
        }
    }
}

// TODO I think this is still a bit weird. Need to do a proper test of all starting melees. Guns are fine I think
fn set_equipment() {
    let data = ARCHIPELAGO_DATA.read().unwrap();
    with_active_player_data(|d| {
        if !data.items.contains(
            *constants::GUN_MAP
                .get_by_right(&d.gun)
                .unwrap_or_else(|| panic!("Unexpected gun value: {}", d.gun)),
        ) {
            // Set the actor data and make sure to update the equipped gun, otherwise weirdness happens (I.e double wielding shotguns)
            for (gun, idx) in constants::GUN_MAP.iter() {
                if data.items.contains(*gun) {
                    d.gun = *idx;
                    CHANGE_EQUIPPED_GUN(d.gun as u32);
                    break;
                }
            }
        }
        if !data.items.contains(
            *constants::MELEE_MAP
                .get_by_right(&d.melee)
                .unwrap_or_else(|| panic!("Unexpected melee value: {}", d.melee)),
        ) {
            // Set actor data then update melee
            for (melee, idx) in constants::MELEE_MAP.iter() {
                if data.items.contains(*melee) {
                    d.melee = *idx;
                    //CHANGE_EQUIPPED_MELEE(d.melee, 0);
                    if *melee == "Sparda" {
                        d.melee_form = 1;
                    }
                    // Set updateMeleeWeapon TODO Remove and replace with CHANGE_EQUIPPED_MELEE/CHANGE_MELEE_FORM?
                    const WEAPON_DATA: usize = 0x60AD10;
                    let data_addr: usize = read_data_from_address(*DMC1_ADDRESS + WEAPON_DATA);
                    unsafe { replace_single_byte(data_addr + 0xACA1, 1) };
                    break;
                }
            }
            UPDATE_WEAPONS();
            log::debug!("Setting actor melee to: {}", d.melee)
        }
    })
    .unwrap();
}

fn set_weapons_in_inv() {
    if let Ok(data) = ARCHIPELAGO_DATA.read() {
        for weapon in get_items_by_category(ItemCategory::Weapon) {
            if data.items.contains(weapon) {
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
    if !CONNECTED.load(Ordering::Relaxed) {
        return;
    }

    if let Ok(data) = ARCHIPELAGO_DATA.read() {
        with_session(|s| {
            match MISSION_ITEM_MAP.get(&(s.mission)) {
                None => {} // No items for the mission
                Some(item_list) => {
                    for item in item_list.iter() {
                        if data.items.contains(*item) {
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
    // Key items need to be checked, not others
    if !get_items_by_category(ItemCategory::Key).contains(&item_name) {
        return true;
    }
    let mut res = false;
    with_session(|s| {
        match MISSION_ITEM_MAP.get(&(s.mission)) {
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
