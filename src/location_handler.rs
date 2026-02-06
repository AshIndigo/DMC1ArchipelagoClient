use crate::check_handler::{Location, LocationType};
use crate::constants::{ITEM_DATA_MAP, REMOTE_ID};
use crate::data::generated_locations;
use crate::game_manager::ItemData;
use crate::mapping::Mapping;
use crate::{constants, mapping};
use archipelago_rs::Client;
use std::error::Error;

pub fn get_location_name_by_data(
    location_data: &Location,
    client: &Client<Mapping>,
) -> Result<&'static str, Box<dyn Error>> {
    if location_data.location_type != LocationType::Standard
        && let Some(location) =
            generated_locations::ITEM_MISSION_MAP
                .iter()
                .find(|(key, _item_entry)| match location_data.location_type {
                    LocationType::Standard => unreachable!(),
                    LocationType::MissionComplete => {
                        *(*key) == format!("Mission #{} Complete", location_data.mission).as_str()
                    }
                    LocationType::SSRank => {
                        *(*key) == format!("Mission #{} SS Rank", location_data.mission).as_str()
                    }
                    LocationType::PurchaseItem => {
                        *(*key)
                            == format!(
                                "Purchase {}",
                                match location_data.item_category {
                                    constants::EXTRA_STORE => {
                                        match location_data.item_id {
                                            5 => format!("Blue Orb #{}", location_data.mission),
                                            6 => format!("Purple Orb #{}", location_data.mission),
                                            _ => unreachable!(),
                                        }
                                    }
                                    _ => unreachable!(),
                                }
                            )
                    }
                })
    {
        return Ok(location.0);
    }

    let filtered_locs =
        generated_locations::ITEM_MISSION_MAP
            .iter()
            .filter(|(_key, item_entry)| {
                (item_entry.room_number == location_data.room)
                    && (item_entry.track_number == location_data.track)
                    && ((!item_entry.coordinates.has_coords())
                        || item_entry.coordinates == location_data.coordinates)
            });
    for (key, entry) in filtered_locs {
        // Wew.
        if entry.item_id as i64
            == (if let Some(item_data) = constants::find_item_by_data(&ItemData {
                category: location_data.item_category,
                id: location_data.item_id as u8,
                count: 1,
            }) {
                if let Some(item) = client.this_game().item_by_name(item_data) {
                    item.id()
                } else {
                    -1
                }
            } else {
                log::debug!("Item isn't in constants, see pickup message");
                -1
            })
            || location_data.item_id == *REMOTE_ID
        {
            return Ok(key);
        }
    }
    Err(Box::from("No location found"))
}

pub fn get_mapped_data(location_name: &str) -> Result<ItemData, Box<dyn Error>> {
    let mut opt_item = None;
    let id = match mapping::CACHED_LOCATIONS.read() {
        Ok(cached_locations) => {
            if let Some(located_item) = cached_locations.get(location_name) {
                if located_item.sender() == located_item.receiver() {
                    opt_item = Some(located_item.clone());
                    located_item.item().id() as u32
                } else {
                    *REMOTE_ID
                }
            } else {
                log::error!(
                    "Location wasn't scouted: {}, defaulting to Remote ID",
                    location_name
                );
                *REMOTE_ID
            }
        }
        Err(err) => {
            log::error!("Unable to read scout cache: {}", err);
            *REMOTE_ID
        }
    };
    // Red Orbs
    if 43 >= id && id > 40 {
        return Ok(*ITEM_DATA_MAP.get("Red Orb - 1").unwrap());
    }

    // To set the displayed graphic to the corresponding weapon
    if id >= 100 {
        return Ok(match id {
            (100..=105) => *ITEM_DATA_MAP.get("Alastor").unwrap(),
            (107..=113) => *ITEM_DATA_MAP.get("Ifrit").unwrap(),
            _ => {
                log::error!("Unrecognized id {}, default to Remote", id);
                get_remote_data()
            }
        });
    }
    if let Some(opt_item) = opt_item {
        Ok(*ITEM_DATA_MAP.get(opt_item.item().name().as_str()).unwrap())
    } else {
        Ok(get_remote_data())
    }
}

pub fn get_remote_data() -> ItemData {
    *ITEM_DATA_MAP.get("Remote").unwrap()
}
