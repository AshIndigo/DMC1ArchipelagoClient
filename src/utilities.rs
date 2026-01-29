use crate::game_manager::{ItemData, with_session};
use randomizer_utilities::get_base_address;
use std::sync::LazyLock;

pub static DMC1_ADDRESS: LazyLock<usize> = LazyLock::new(|| get_base_address("dmc1.exe"));

pub fn is_ddmk_loaded() -> bool {
    randomizer_utilities::is_library_loaded("Eva.dll")
}

pub fn insert_unique_item_into_inv(item_data: &ItemData) {
    with_session(|s| {
        for i in 0..s.item_count {
            let item = &mut s.item_data[i as usize];
            if item == item_data {
                item.count = item_data.count;
                return;
            }
        }
        let item = &mut s.item_data[s.item_count as usize];
        item.category = item_data.category;
        item.id = item_data.id;
        item.count = item_data.count;
        s.item_count += 1;
    })
    .unwrap();
}

pub fn insert_item_into_inv(item_data: &ItemData) {
    with_session(|s| {
        for i in 0..s.item_count {
            let item = &mut s.item_data[i as usize];
            if item == item_data {
                item.count += item_data.count;
                return;
            }
        }
        let item = &mut s.item_data[s.item_count as usize];
        item.category = item_data.category;
        item.id = item_data.id;
        item.count = item_data.count;
        s.item_count += 1;
    })
    .unwrap();
}

pub(crate) fn clear_item_slot(item_data: &ItemData) {
    with_session(|s| {
        for i in 0..s.item_count {
            let item = &mut s.item_data[i as usize];
            if item == item_data {
                // Swap in last element
                s.item_data[i as usize] = s.item_data[(s.item_count - 1) as usize];
                s.item_count -= 1;
                return;
            }
        }
    })
    .unwrap();
}

pub(crate) fn is_on_main_menu() -> bool {
    true
}
