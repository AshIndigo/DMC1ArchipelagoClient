use std::mem::offset_of;
use crate::game_manager::{with_active_player_data, with_session, ItemData, PlayerData};
use randomizer_utilities::get_base_address;
use std::sync::LazyLock;
use randomizer_utilities::cache::DATA_PACKAGE;
use crate::constants::GAME_NAME;

pub static DMC1_ADDRESS: LazyLock<usize> = LazyLock::new(|| get_base_address("dmc1.exe"));

pub fn is_ddmk_loaded() -> bool {
    randomizer_utilities::is_library_loaded("Eva.dll")
}

pub fn insert_unique_item_into_inv(item_data: &ItemData) {
    with_session(|s| {
        for i in 0..s.item_count {
            let item = &mut s.item_data[i as usize];
            if item == item_data {
                log::debug!("Item found");
                item.count = item_data.count;
                //log::debug!("Item data: {:#?}", &s.item_data[0..20]);
                return;
            }
        }
        let item = &mut s.item_data[s.item_count as usize];
        item.category = item_data.category;
        item.id = item_data.id;
        item.count = item_data.count;
        s.item_count += 1;
        log::debug!("Item not found");
        //log::debug!("Item data: {:#?}", &s.item_data[0..20]);
    }).unwrap();
}

pub fn insert_item_into_inv(item_data: &ItemData) {
    with_session(|s| {
        for i in 0..s.item_count {
            let item = &mut s.item_data[i as usize];
            if item == item_data {
                log::debug!("Item found");
                item.count += item_data.count;
                log::debug!("Item data: {:#?}", &s.item_data[0..20]);
                return;
            }
        }
        let item = &mut s.item_data[s.item_count as usize];
        item.category = item_data.category;
        item.id = item_data.id;
        item.count = item_data.count;
        s.item_count += 1;
        log::debug!("Item not found");
        log::debug!("Item data: {:#?}", &s.item_data[0..20]);
    }).unwrap();
}

pub fn give_hp(hp: u8) {
    with_session(|s| {
        s.hp += hp;
    }).unwrap();
    with_active_player_data(|d| {
        log::debug!("Max HP found: {:#X}", offset_of!(PlayerData, max_hp));
        log::debug!("HP found: {:#X}", offset_of!(PlayerData, hp));
        d.hp += (hp as u16)*100;
        d.max_hp += (hp as u16)*100;
    }).unwrap();
}

pub(crate) fn clear_item_slot(item_data: &ItemData) {
    with_session(|s| {
        for i in 0..s.item_count {
            let item = &mut s.item_data[i as usize];
            if item == item_data {
                log::debug!("Item found");
                item.id = 0;
                item.category = 0;
                item.count = 0;
                return;
            }
        }
    }).unwrap();
}

pub fn get_item_name(id: i64) -> Option<String> {
    if let Some(cache) = DATA_PACKAGE.read().unwrap().as_ref() {
        return match cache.item_id_to_name.get(GAME_NAME) {
            None => None,
            Some(map) => {
                map.get(&id).cloned()
            }
        }
    }
    None
}