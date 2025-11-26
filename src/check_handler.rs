use crate::constants::{
    find_item_by_vals, BasicNothingFunc, Coordinates, DMC1Config, Difficulty, Rank, EMPTY_COORDINATES,
    ITEM_DATA_MAP,
};
use crate::game_manager::{
    get_mission, get_room, get_track, with_session_read, ItemData, ARCHIPELAGO_DATA,
};
use crate::mapping::MAPPING;
use crate::utilities::{clear_item_slot, get_item_name, DMC1_ADDRESS};
use crate::{create_hook, hook, location_handler};
use minhook::MinHook;
use minhook::MH_STATUS;
use randomizer_utilities::{read_data_from_address, replace_single_byte};
use std::fmt::{Display, Formatter};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;
use tokio::sync::mpsc::Sender;

pub(crate) static TX_LOCATION: OnceLock<Sender<Location>> = OnceLock::new();
#[derive(Debug, Clone, Copy)]
pub(crate) struct Location {
    pub(crate) item_id: u8,
    pub(crate) room: i32,
    pub(crate) track: i32,
    pub(crate) mission: u32,
    pub coordinates: Coordinates,
    pub(crate) item_category: u8,
}

impl Display for Location {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Track: {} Room ID: {} Item ID: {} Item Category: {}",
            self.track, self.room, self.item_id, self.item_category
        )
    }
}

impl PartialEq for Location {
    fn eq(&self, other: &Self) -> bool {
        self.coordinates == other.coordinates
            && self.room == other.room
            && self.track == other.track
            && self.item_id == other.item_id
            && self.item_category == other.item_category
    }
}

pub const ITEM_PICKUP_ADDR: usize = 0x3d5d80;
static ORIGINAL_PICKUP: OnceLock<BasicNothingFunc> = OnceLock::new();

// This works, just has the issue of running 4 times for some reason
pub fn item_pickup() {
    // [[dmc1.exe+60ad10]+ac88]+88
    // Using the DDMK name since I don't have a better one
    const WEAPON_DATA: usize = 0x60AD10;
    const OFFSET_1: usize = 0xAC88;
    // Points to ItemData
    const CATEGORY_OFFSET: usize = 0x88;
    const ID_OFFSET: usize = 0x89;

    let data_addr: usize = read_data_from_address(*DMC1_ADDRESS + WEAPON_DATA);
    let pickup_offset: usize = read_data_from_address(data_addr + OFFSET_1);
    let category: u8 = read_data_from_address(pickup_offset + CATEGORY_OFFSET);
    let id: u8 = read_data_from_address(pickup_offset + ID_OFFSET);
    log::debug!(
        "Item pickup: Category: {} ID: {} - Item is: {:?}",
        category,
        id,
        find_item_by_vals(id, category)
    );
    // Gather location info
    let received_item = Location {
        item_id: id,
        item_category: category,
        room: get_room(),
        track: get_track(),
        mission: get_mission() as u32,
        coordinates: EMPTY_COORDINATES,
    };
    // Send off information
    send_off_location_coords(received_item, 2);

    // Figure out which location we are at for replacement purposes
    match location_handler::get_location_name_by_data(&received_item) {
        Ok(loc_key) => {
            if let Some(mappings) = MAPPING.read().unwrap().as_ref() {
                // Get the AP item data for that location
                let location_data = mappings.items.get(loc_key).unwrap();
                let item_name =
                    get_item_name(location_data.get_in_game_id::<DMC1Config>() as i64).unwrap();
                log::debug!("Actual item name is {item_name}");
                let data = ITEM_DATA_MAP.get(&item_name.as_str()).unwrap();
                unsafe {
                    replace_single_byte(pickup_offset + ID_OFFSET, data.id);
                    replace_single_byte(pickup_offset + CATEGORY_OFFSET, data.category);
                }
            }
        }
        Err(err) => {
            log::error!("Failed to get location key: {}", err);
        }
    }

    if let Some(func) = ORIGINAL_PICKUP.get() {
        unsafe { func() }
    }
}

pub fn setup_check_hooks() -> Result<(), MH_STATUS> {
    log::debug!("Setting up check related hooks");
    unsafe {
        create_hook!(
            ITEM_PICKUP_ADDR,
            item_pickup,
            ORIGINAL_PICKUP,
            "Non event item"
        );
        create_hook!(
            ADD_ITEM_ADDR,
            add_item_to_inv,
            ORIGINAL_ADD_ITEM,
            "Add item to inventory"
        );
        create_hook!(
            DISPLAY_ITEMS_ADDR,
            display_inventory,
            ORIGINAL_DISPLAY_ITEMS,
            "???"
        );
        create_hook!(
            MISSION_COMPLETE_ADDR,
            mission_complete,
            ORIGINAL_MISSION_COMPLETE,
            "Mission Complete"
        );
    }
    Ok(())
}

pub(crate) const ADD_ITEM_ADDR: usize = 0x3d78d0;
static ORIGINAL_ADD_ITEM: OnceLock<unsafe extern "C" fn(u8, u8, u16)> = OnceLock::new();
pub fn add_item_to_inv(category: u8, id: u8, count: u16) {
    if let Some(func) = ORIGINAL_ADD_ITEM.get() {
        unsafe { func(category, id, count) };
    }
    let name = find_item_by_vals(id, category).unwrap();
    if !hook::is_item_relevant_to_mission(name) {
        log::debug!("{name} isn't relevant to current mission");
        LAST_ID.store(id, Ordering::Relaxed);
        LAST_CATEGORY.store(category, Ordering::Relaxed);
    }
}

const NOTHING: u8 = u8::MAX;
static LAST_ID: AtomicU8 = AtomicU8::new(NOTHING);
static LAST_CATEGORY: AtomicU8 = AtomicU8::new(NOTHING);

pub const DISPLAY_ITEMS_ADDR: usize = 0x3d7690;
static ORIGINAL_DISPLAY_ITEMS: OnceLock<BasicNothingFunc> = OnceLock::new();

// Runs when the inventory is opened or displayed (i.e picking up an item)
// Cleans out unneeded items/weapons to prevent potential skips or issues
pub fn display_inventory() {
    let id = LAST_ID.load(Ordering::Relaxed);
    let category = LAST_CATEGORY.load(Ordering::Relaxed);
    if id != NOTHING || category != NOTHING {
        clear_item_slot(&ItemData {
            category,
            id,
            count: 0,
        });
        LAST_ID.store(NOTHING, Ordering::Relaxed);
        LAST_CATEGORY.store(NOTHING, Ordering::Relaxed);
    }
    if let Some(func) = ORIGINAL_DISPLAY_ITEMS.get() {
        unsafe {
            func();
        }
    }
}

// Give Mission Specific Achievements
// 0x24500 and 0x3e21f0 are called every frame on the mission complete screen
pub const MISSION_COMPLETE_ADDR: usize = 0x256f20;
static ORIGINAL_MISSION_COMPLETE: OnceLock<BasicNothingFunc> = OnceLock::new();
fn mission_complete() {
    if let Some(func) = ORIGINAL_MISSION_COMPLETE.get() {
        unsafe {
            func();
        }
    }
    with_session_read(|session| {
        log::debug!(
            "Mission {} Complete - Difficulty: {} - Rank: {}",
            session.mission,
            Difficulty::from_repr(session.difficulty as usize).unwrap(),
            Rank::from_repr(session.rank as usize).unwrap()
        );
        send_off_location_coords(
            Location {
                item_id: u8::MAX,
                room: -1,
                track: -1,
                mission: session.mission as u32,
                coordinates: EMPTY_COORDINATES,
                item_category: 0,
            },
            u32::MAX,
        );
    })
    .unwrap();

}

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn send_off_location_coords(loc: Location, to_display: u32) {
    if let Some(tx) = TX_LOCATION.get() {
        tx.send(loc).await.expect("Failed to send Location!");
        if to_display != u32::MAX {
            // clear_high_roller();
            // text_handler::LAST_OBTAINED_ID.store(to_display as u8, SeqCst);
            //take_away_received_item(loc.item_id);
        }
    }
}
