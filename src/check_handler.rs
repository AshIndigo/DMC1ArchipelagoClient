use crate::constants::{
    BasicNothingFunc, Coordinates, Difficulty, EMPTY_COORDINATES, Rank, find_item_by_vals,
};
use crate::game_manager::{ItemData, get_mission, get_room, get_track, with_session_read};
use crate::mapping::CACHED_LOCATIONS;
use crate::ui::text_handler;
use crate::ui::text_handler::REPLACE_TEXT;
use crate::utilities::{DMC1_ADDRESS, clear_item_slot};
use crate::{constants, create_hook, hook, location_handler};
use minhook::MH_STATUS;
use minhook::MinHook;
use randomizer_utilities::read_data_from_address;
use std::fmt::{Display, Formatter};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;

pub(crate) static TX_LOCATION: OnceLock<Sender<Location>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LocationType {
    Standard,
    MissionComplete,
    // TODO No concept of SS Ranks in DMC1, should I replace with S Rank checks? Or drop it
    SSRank,
    PurchaseItem,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Location {
    pub(crate) location_type: LocationType,
    pub(crate) item_id: u32,
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
// 3c50b0; - Doesn't work because its also called when inv is opened
// 0x3c5390 - Also called from inv.
static ORIGINAL_PICKUP: OnceLock<BasicNothingFunc> = OnceLock::new();

static CALL_COUNT: AtomicU8 = AtomicU8::new(0);

const IGNORED_ITEMS: [ItemData; 2] = [
    // Red Orbs - 1
    ItemData {
        category: 2,
        id: 0,
        count: 0,
    },
    // Red Orbs - 5
    ItemData {
        category: 2,
        id: 1,
        count: 0,
    },
];

// This works, just has the issue of running 4 times for some reason
pub fn item_pickup() {
    // [[dmc1.exe+60ad10]+ac88]+88
    // Using the DDMK name since I don't have a better one
    const WEAPON_DATA: usize = 0x60AD10;
    const OFFSET_1: usize = 0xAC88;
    // Points to ItemData
    const CATEGORY_OFFSET: usize = 0x88;
    const ID_OFFSET: usize = 0x89;
    let call_count = CALL_COUNT.load(Ordering::Relaxed);
    if call_count == 0 {
        let data_addr: usize = read_data_from_address(*DMC1_ADDRESS + WEAPON_DATA);
        let pickup_offset: usize = read_data_from_address(data_addr + OFFSET_1);
        let category: u8 = read_data_from_address(pickup_offset + CATEGORY_OFFSET);
        let id: u8 = read_data_from_address(pickup_offset + ID_OFFSET);
        let item_data = ItemData {
            id,
            category,
            count: 0,
        };
        if !IGNORED_ITEMS.contains(&item_data) {
            log::debug!(
                "Item pickup: Category: {} ID: {} - Item is: {:?}\nMission: {}, Room: {}, Track: {}",
                category,
                id,
                find_item_by_vals(id, category),
                get_mission(),
                get_room(),
                get_track()
            );
            // Gather location info
            let received_item = Location {
                location_type: LocationType::Standard,
                item_id: id as u32,
                item_category: category,
                room: get_room(),
                track: get_track(),
                mission: get_mission() as u32,
                coordinates: EMPTY_COORDINATES,
            };
            // Send off information
            send_off_location_coords(received_item);

            // Figure out which location we are at for replacement purposes
            match crate::AP_CORE.get().unwrap().lock() {
                Ok(core) => {
                    if let Some(client) = core.connection.client() {
                        match location_handler::get_location_name_by_data(&received_item, client) {
                            Ok(loc_key) => {
                                // Get the AP item data for that location
                                let map = CACHED_LOCATIONS.read().unwrap();
                                let located_item = map.get(loc_key).unwrap();
                                log::debug!("Actual item name is {}", located_item.item().name());
                                let data = location_handler::get_mapped_data(loc_key).unwrap();
                                unsafe {
                                    randomizer_utilities::replace_single_byte(
                                        pickup_offset + ID_OFFSET,
                                        data.id,
                                    );
                                    randomizer_utilities::replace_single_byte(
                                        pickup_offset + CATEGORY_OFFSET,
                                        data.category,
                                    );
                                }
                                REPLACE_TEXT.store(true, Ordering::Relaxed);
                                if let Ok(mut txt) = text_handler::FOUND_ITEM.write() {
                                    *txt = Some(located_item.clone());
                                }
                            }
                            Err(err) => {
                                log::error!("Failed to get location key: {}", err);
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("Failed to get core: {}", err);
                }
            }
        }
    }

    // Only want this to call once, no need to check the same loc
    if call_count >= 3 {
        log::debug!("Resetting call count");
        CALL_COUNT.store(0, Ordering::Relaxed);
    } else {
        CALL_COUNT.fetch_add(1, Ordering::Relaxed);
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
            SORT_INVENTORY,
            sort_inventory,
            ORIGINAL_SORT_INVENTORY,
            "Sorts the inventory after adding an item"
        );
        create_hook!(
            MISSION_COMPLETE_ADDR,
            mission_complete,
            ORIGINAL_MISSION_COMPLETE,
            "Mission Complete"
        );
        create_hook!(
            PURCHASE_ITEM_ADDR,
            purchase_item,
            ORIGINAL_PURCHASE_ITEM,
            "Purchase Item"
        );
    }
    Ok(())
}

pub(crate) fn add_hooks_to_list(addrs: &mut Vec<usize>) {
    const ADDRESSES: [usize; 5] = [
        ITEM_PICKUP_ADDR,
        ADD_ITEM_ADDR,
        SORT_INVENTORY,
        MISSION_COMPLETE_ADDR,
        PURCHASE_ITEM_ADDR,
    ];
    for a in ADDRESSES.iter() {
        addrs.push(*a);
    }
}

pub(crate) const ADD_ITEM_ADDR: usize = 0x3d78d0;
static ORIGINAL_ADD_ITEM: OnceLock<unsafe extern "C" fn(u8, u8, u16)> = OnceLock::new();
pub fn add_item_to_inv(category: u8, id: u8, count: u16) {
    if let Some(func) = ORIGINAL_ADD_ITEM.get() {
        unsafe { func(category, id, count) };
    }
    if (category == 2 && id == 8) || (category == 5 && id == 4) {
        log::debug!("Taking un-needed item");
        LAST_ID.store(id, Ordering::Relaxed);
        LAST_CATEGORY.store(category, Ordering::Relaxed);
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

pub const SORT_INVENTORY: usize = 0x3d7690;
static ORIGINAL_SORT_INVENTORY: OnceLock<BasicNothingFunc> = OnceLock::new();

// Runs when the inventory is opened or displayed (i.e picking up an item)
// Cleans out unneeded items/weapons to prevent potential skips or issues
pub fn sort_inventory() {
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
    if let Some(func) = ORIGINAL_SORT_INVENTORY.get() {
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
            session.mission - 1,
            Difficulty::from_repr(session.difficulty as usize).unwrap(),
            Rank::from_repr(session.rank as usize).unwrap()
        );
        send_off_location_coords(Location {
            location_type: LocationType::MissionComplete,
            item_id: u32::MAX,
            room: -1,
            track: -1,
            mission: (session.mission - 1) as u32,
            coordinates: EMPTY_COORDINATES,
            item_category: 0,
        });
    })
    .unwrap();
}

static PURCHASE_ITEM_ADDR: usize = 0x3DF5B0; // Called every attempted purchase
static ORIGINAL_PURCHASE_ITEM: OnceLock<BasicNothingFunc> = OnceLock::new();

pub fn purchase_item() {
    const WEAPON_DATA: usize = 0x60AD10;
    const PURCHASE_IDX_OFFSET: usize = 0xAA6D;
    const PURCHASE_MENU_IDX_OFFSET: usize = 0xAA6C;
    let orig_red_orbs = with_session_read(|session| session.red_orbs).unwrap();
    if let Some(orig) = ORIGINAL_PURCHASE_ITEM.get() {
        unsafe {
            orig();
        }
    }
    if with_session_read(|session| session.red_orbs).unwrap() < orig_red_orbs {
        let data_addr: usize = read_data_from_address(*DMC1_ADDRESS + WEAPON_DATA);
        let idx: u8 = read_data_from_address(data_addr + PURCHASE_IDX_OFFSET);
        let category: u8 = read_data_from_address(data_addr + PURCHASE_MENU_IDX_OFFSET);
        match category {
            constants::EXTRA_STORE => {
                let count = match idx {
                    5 => with_session_read(|s| s.bought_hp).unwrap(),
                    6 => with_session_read(|s| s.bought_magic).unwrap(),
                    // Ignored
                    _ => u8::MAX,
                };
                if count != u8::MAX {
                    send_off_location_coords(Location {
                        location_type: LocationType::PurchaseItem,
                        item_id: idx as u32,
                        mission: count as u32,
                        room: 0,
                        coordinates: EMPTY_COORDINATES,
                        track: 0,
                        item_category: category,
                    });
                }
            }
            // TODO Buying skill checks
            constants::ALASTOR_STORE => {
                // Skill purchases do not differentiate between skill levels
                log::debug!("Alastor skill purchase: {idx}");
            }
            constants::IFRIT_STORE => {
                // Skill purchases do not differentiate between skill levels
                log::debug!("Ifrit skill purchase: {idx}");
            }
            _ => unreachable!(),
        }
    }
}

fn send_off_location_coords(loc: Location) {
    if let Some(tx) = TX_LOCATION.get() {
        tx.send(loc).expect("Failed to send Location!");
    }
}
