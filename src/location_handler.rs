use crate::check_handler::Location;
use crate::constants;
use crate::data::generated_locations;
use crate::utilities::get_item_name;
use std::error::Error;

pub fn get_location_name_by_data(location_data: &Location) -> Result<&'static str, Box<dyn Error>> {
    if location_data.room == -1 {
        let mission_loc: Vec<_> = generated_locations::ITEM_MISSION_MAP
            .iter()
            .filter(|(key, _item_entry)| {
                *(*key) == format!("Mission #{} Complete", location_data.mission).as_str()
            })
            .collect();
        return Ok(mission_loc[0].0);
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
        let item_name = get_item_name(entry.item_id as i64);
        // TODO Remote
        if let Some(item_name) = item_name {
            let item = constants::ITEM_DATA_MAP.get(item_name.as_str());
            if let Some(item) = item {
                if item.id == location_data.item_id && item.category == location_data.item_category
                {
                    return Ok(key);
                }
            }
        }
    }
    Err(Box::from("No location found"))
}
