use crate::check_handler::{Location, LocationType};
use crate::constants::REMOTE_ID;
use crate::data::generated_locations;
use std::error::Error;

pub fn get_location_name_by_data(location_data: &Location) -> Result<&'static str, Box<dyn Error>> {
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
                    // TODO This ain't right
                    LocationType::PurchaseItem => {
                        *(*key)
                            == format!(
                                "Purchase {}",
                                match location_data.item_id {
                                    0x07 => format!("Blue Orb #{}", location_data.mission),
                                    0x08 => format!("Purple Orb #{}", location_data.mission),
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
                    && ((!item_entry.coordinates.has_coords())
                        || item_entry.coordinates == location_data.coordinates)
            });
    for (key, entry) in filtered_locs {
        if entry.item_id == location_data.item_id || location_data.item_id == *REMOTE_ID {
            return Ok(key);
        }
    }
    Err(Box::from("No location found"))
}
