use crate::constants::{BasicNothingFunc, Coordinates, EMPTY_COORDINATES};
use crate::game_manager::{get_mission, get_room, get_track};
use crate::utilities::DMC1_ADDRESS;
use crate::{constants, create_hook};
use minhook::MinHook;
use minhook::MH_STATUS;
use randomizer_utilities::read_data_from_address;
use std::fmt::{Display, Formatter};
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

pub fn setup_check_hooks() -> Result<(), MH_STATUS> {
    log::debug!("Setting up check related hooks");
    unsafe {
        create_hook!(
            ITEM_PICKUP_ADDR,
            item_pickup,
            ORIGINAL_PICKUP,
            "Non event item"
        );
    }
    Ok(())
}

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
        constants::find_item_by_vals(id, category)
    );
    send_off_location_coords(
        Location {
            item_id: id,
            item_category: category,
            room: get_room(),
            track: get_track(),
            mission: get_mission() as u32,
            coordinates: EMPTY_COORDINATES,
        },
        2,
    );
    if let Some(func) = ORIGINAL_PICKUP.get() {
        unsafe { func() }
    }
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
