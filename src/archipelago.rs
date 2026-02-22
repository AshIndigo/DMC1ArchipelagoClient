use crate::check_handler::{Location, TX_LOCATION};
use crate::constants::*;
use crate::game_manager::{ARCHIPELAGO_DATA, ArchipelagoData, get_mission, with_session};
use crate::mapping::{DeathlinkSetting, Goal, MAPPING, Mapping, OVERLAY_INFO, OverlayInfo};
use crate::ui::overlay;
use crate::ui::overlay::{MessageSegment, MessageType, OverlayMessage};
use crate::{game_manager, hook, location_handler, mapping, skill_manager, utilities};
use archipelago_rs::{
    AsItemId, Client, ClientStatus, Connection, ConnectionOptions, ConnectionState, CreateAsHint,
    DeathLinkOptions, Event, ItemHandling,
};
use randomizer_utilities::archipelago_utilities::{DeathLinkData, handle_print};
use randomizer_utilities::item_sync::CURRENT_INDEX;
use randomizer_utilities::ui::font_handler::{WHITE, YELLOW};
use randomizer_utilities::{archipelago_utilities, item_sync, setup_channel_pair};
use std::error::Error;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;

pub(crate) static CONNECTED: AtomicBool = AtomicBool::new(false);
pub static TX_DEATHLINK: OnceLock<Sender<DeathLinkData>> = OnceLock::new();

pub struct ArchipelagoCore {
    pub connection: Connection<Mapping>,
    hooks_installed: bool,
    hooks_enabled: bool,

    location_receiver: Receiver<Location>,
    deathlink_receiver: Receiver<DeathLinkData>,
}

impl ArchipelagoCore {
    pub fn new(url: String, game_name: String) -> anyhow::Result<Self> {
        Ok(Self {
            connection: Connection::new(
                url,
                game_name,
                "",
                ConnectionOptions::new().receive_items(ItemHandling::OtherWorlds {
                    own_world: true,
                    starting_inventory: true,
                }),
            ),
            hooks_installed: false,
            hooks_enabled: false,
            location_receiver: setup_channel_pair(&TX_LOCATION),
            deathlink_receiver: setup_channel_pair(&TX_DEATHLINK),
        })
    }

    pub fn update(&mut self) -> Result<(), Box<dyn Error>> {
        for event in self.connection.update() {
            match event {
                Event::Connected => {
                    log::info!("Connected!");
                    log::debug!("Mod version: {}", env!("CARGO_PKG_VERSION"));
                    let mapping = self.connection.client().unwrap().slot_data();
                    let mut overlay_info = OVERLAY_INFO.write()?;
                    log::info!("Running in randomizer mode");
                    overlay_info.generated_version = mapping.generated_version;
                    overlay_info.client_version = mapping.client_version;
                    MAPPING.write()?.replace(mapping.clone());
                    item_sync::send_offline_checks(self.connection.client_mut().unwrap())?;
                    if !self.hooks_installed {
                        // Hooks needed to modify the game
                        unsafe {
                            match hook::create_hooks() {
                                Ok(_) => {
                                    log::debug!("Created DMC1 Hooks");
                                    self.hooks_installed = true;
                                }
                                Err(err) => {
                                    log::error!("Failed to create hooks: {:?}", err);
                                }
                            }
                        }
                    }
                    if self.hooks_installed && !self.hooks_enabled {
                        hook::enable_hooks();
                        self.hooks_enabled = true;
                    }
                    run_setup(self.connection.client_mut().unwrap())?;

                    // Print out version info
                    log::debug!(
                        "Client version: {}",
                        if let Some(cv) = overlay_info.client_version {
                            cv.to_string()
                        } else {
                            "Unknown".to_string()
                        }
                    );

                    log::debug!(
                        "Generated version: {}",
                        if let Some(gv) = overlay_info.generated_version {
                            gv.to_string()
                        } else {
                            "Unknown".to_string()
                        }
                    );
                }
                Event::Updated(_) => {}
                Event::Print(print) => {
                    let str = handle_print(print);
                    log::info!("Print from server: {}", str);
                }
                Event::ReceivedItems(idx) => {
                    handle_received_items_packet(idx, self.connection.client_mut().unwrap())?;
                }
                Event::Error(err) => log::error!("{}", err),
                Event::Bounce {
                    games: _,
                    slots: _,
                    tags: _,
                    data: _,
                } => {}
                Event::DeathLink {
                    games: _,
                    slots: _,
                    tags: _,
                    time: _,
                    cause,
                    source,
                } => {
                    overlay::add_message(OverlayMessage::new(
                        vec![MessageSegment::new(
                            format!("{}: {}", source, cause.unwrap_or_default()),
                            WHITE,
                        )],
                        Duration::from_secs(3),
                        // TODO May want to adjust position, currently added to the 'notification list' so it's in the upper right queue
                        0.0,
                        0.0,
                        MessageType::Notification,
                    ));

                    match self.connection.client().unwrap().slot_data().death_link {
                        DeathlinkSetting::DeathLink => {
                            game_manager::kill_dante();
                        }
                        DeathlinkSetting::HurtLink => {
                            game_manager::hurt_dante();
                        }
                        DeathlinkSetting::Off => {}
                    }
                }
                Event::KeyChanged {
                    key: _,
                    old_value: _,
                    new_value: _,
                    player: _,
                } => {}
            }
        }
        match self.connection.state() {
            ConnectionState::Connecting(_) => {}
            ConnectionState::Connected(_) => {
                CONNECTED.store(true, Ordering::SeqCst);
            }
            ConnectionState::Disconnected(state) => {
                CONNECTED.store(false, Ordering::SeqCst);
                *OVERLAY_INFO.write()? = OverlayInfo::default();
                disconnect(&mut self.hooks_enabled);
                return Err(format!("Disconnected from server: {:?}", state).into());
            }
        }
        self.handle_channels()?;
        Ok(())
    }

    pub fn handle_channels(&mut self) -> Result<(), Box<dyn Error>> {
        match self.location_receiver.try_recv() {
            Ok(location) => {
                if let Some(client) = self.connection.client_mut() {
                    handle_item_receive(client, location)?;
                } else {
                    log::error!(
                        "Received location check while client was None: {}",
                        location
                    );
                }
            }
            Err(err) => {
                if err == TryRecvError::Disconnected {
                    return Err("Disconnected from location receiver".into());
                }
            }
        }

        match self.deathlink_receiver.try_recv() {
            Ok(dl_data) => self
                .connection
                .client_mut()
                .unwrap()
                .death_link(DeathLinkOptions::new().cause(dl_data.cause))?,
            Err(err) => {
                if err == TryRecvError::Disconnected {
                    return Err("Disconnected from DeathLink receiver".into());
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn handle_received_items_packet(
    index: usize,
    client: &mut Client<Mapping>,
) -> Result<(), Box<dyn Error>> {
    if index == 0 {
        *ARCHIPELAGO_DATA.write()? = ArchipelagoData::default();
    }

    match ARCHIPELAGO_DATA.write() {
        Ok(mut data) => {
            for item in client.received_items().iter() {
                // Display overlay text if we're not at the main menu
                if !utilities::is_on_main_menu()
                    && item.index() >= CURRENT_INDEX.load(Ordering::SeqCst) as usize
                {
                    let rec_msg: Vec<MessageSegment> = vec![
                        MessageSegment::new("Received ".to_string(), WHITE),
                        MessageSegment::new(
                            item.item().name().to_string(),
                            overlay::get_color_for_item(item.as_ref()),
                        ),
                        MessageSegment::new(" from ".to_string(), WHITE),
                        MessageSegment::new(item.sender().alias().parse()?, YELLOW),
                    ];
                    overlay::add_message(OverlayMessage::new(
                        rec_msg,
                        Duration::from_secs(3),
                        0.0,
                        0.0,
                        MessageType::Notification,
                    ));
                }

                match item.item().as_item_id() {
                    41..=43 => {
                        if item.index() >= CURRENT_INDEX.load(Ordering::SeqCst) as usize {
                            let orbs = match item.item().as_item_id() {
                                41 => 100,
                                42 => 150,
                                43 => 200,
                                _ => unreachable!(),
                            };
                            game_manager::give_red_orbs(orbs);
                        }
                    }
                    1..=5 => {
                        // Guns
                        utilities::insert_unique_item_into_inv(
                            ITEM_DATA_MAP.get(&&*item.item().name()).unwrap(),
                        )
                    }
                    6 => {
                        data.add_blue_orb();
                        //ADD_ORB_FUNC(0);
                        game_manager::give_hp(1);
                    }
                    7 => {
                        data.add_purple_orb();
                        //ADD_ORB_FUNC(1);
                        game_manager::give_magic(1, &data);
                    }
                    8..=11 => {
                        // Weapons
                        utilities::insert_unique_item_into_inv(
                            ITEM_DATA_MAP.get(&&*item.item().name()).unwrap(),
                        )
                    }
                    12..=16 => {
                        // Don't add duplicate consumables
                        if item.index() >= CURRENT_INDEX.load(Ordering::SeqCst) as usize {
                            if item.item().id() == 15 {
                                with_session(|session| {
                                    session.yellow_orbs += 1;
                                })
                                .unwrap();
                            } else {
                                utilities::insert_item_into_inv(
                                    ITEM_DATA_MAP.get(&item.item().name().as_str()).unwrap(),
                                )
                            }
                        }
                    }
                    39 => {
                        // DT Unlock
                        data.add_dt();
                        // for _ in 0..3 {
                        //     ADD_ORB_FUNC(1);
                        // }
                        game_manager::give_magic(3, &data);
                    }
                    18..=38 => {
                        // For key items
                        log::debug!("Setting newly acquired key items");
                        match MISSION_ITEM_MAP.get(&(get_mission())) {
                            None => {} // No items for the mission
                            Some(item_list) => {
                                if item_list.contains(&&*item.item().name()) {
                                    utilities::insert_unique_item_into_inv(
                                        ITEM_DATA_MAP.get(&&*item.item().name()).unwrap(),
                                    )
                                }
                            }
                        }
                    }
                    17 => {
                        log::debug!("Giving Bangle of time");
                        utilities::insert_unique_item_into_inv(
                            ITEM_DATA_MAP.get(&&*item.item().name()).unwrap(),
                        )
                    }
                    100..=113 => {
                        // For skills
                        if client.slot_data().randomize_skills {
                            skill_manager::add_skill(item.item().id() as usize, &mut data);
                            skill_manager::set_skills(&data); // Hacky...
                        }
                    }
                    _ => {
                        log::warn!(
                            "Unhandled item ID: {} ({})",
                            item.item().name(),
                            item.item().id()
                        )
                    }
                }
                data.add_item(item.item().name().into());
                if item.index() >= CURRENT_INDEX.load(Ordering::SeqCst) as usize {
                    CURRENT_INDEX.store((item.index() + 1) as i64, Ordering::SeqCst);
                }
            }
        }
        Err(err) => {
            log::error!("{}", err);
        }
    }

    Ok(())
}

fn handle_item_receive(
    client: &mut Client<Mapping>,
    received_item: Location,
) -> Result<(), Box<dyn Error>> {
    // See if there's an item!
    log::info!("Processing item: {}", received_item);
    let location_key = location_handler::get_location_name_by_data(&received_item, client)?;
    // Then see if the item picked up matches the specified in the map
    match archipelago_utilities::CACHED_LOCATIONS
        .read()?
        .get(location_key)
    {
        Some(located_item) => {
            if let Err(arch_err) = client.mark_checked(vec![located_item.location()]) {
                log::error!("Failed to check location: {}", arch_err);
                item_sync::add_offline_check(located_item.location().id());
            }
            let name = located_item.item().name();
            let in_game_id = if located_item.sender() == located_item.receiver() {
                located_item.item().as_item_id() as u32
            } else {
                *REMOTE_ID
            };
            if let Ok(mut archipelago_data) = ARCHIPELAGO_DATA.write()
                //&& in_game_id > 0x14
                && in_game_id != *REMOTE_ID
            {
                archipelago_data.add_item(located_item.item().name().to_string());
            }

            log::info!(
                "Location check successful: {}, Item: {}",
                location_key,
                name
            );
        }
        None => Err(anyhow::anyhow!("Location not found: {}", location_key))?,
    }
    // Add to checked locations
    if has_reached_goal(client) {
        client.set_status(ClientStatus::Goal)?
    }
    Ok(())
}

fn has_reached_goal(client: &mut Client<Mapping>) -> bool {
    let mut chk = client.checked_locations();
    match client.slot_data().goal {
        Goal::Standard => chk.any(|loc| loc.name() == "Mission #20 Complete"),
        Goal::All => {
            for i in 1..20 {
                // If we are missing a mission complete check then we cannot goal
                if !chk.any(|loc| loc.name() == format!("Mission #{} Complete", i).as_str()) {
                    return false;
                }
            }
            // If we have them all, goal
            true
        }
        Goal::RandomOrder => {
            if let Some(order) = &client.slot_data().mission_order {
                return chk
                    .any(|loc| loc.name() == format!("Mission #{} Complete", order[19]).as_str());
            }
            false
        }
    }
}

const GENERIC_CHECKS: u32 = 40;

/// This is run when a there is a valid connection to a room.
pub fn run_setup(client: &mut Client<Mapping>) -> Result<(), Box<dyn Error>> {
    log::info!("Running setup");
    // Shop checks
    // TODO Hint options
    mapping::run_scouts_for_mission(client, GENERIC_CHECKS, CreateAsHint::New);
    mapping::run_scouts_for_mission(client, NO_MISSION, CreateAsHint::No);
    for i in 1..=23 {
        mapping::run_scouts_for_mission(client, i, CreateAsHint::No);
    }
    mapping::run_scouts_for_secret_mission(client);
    Ok(())
}

fn disconnect(hooks_enabled: &mut bool) {
    log::info!("Disconnecting and restoring game");
    if *hooks_enabled {
        match hook::disable_hooks() {
            Ok(_) => {
                log::debug!("Disabled hooks");
                *hooks_enabled = false;
            }
            Err(e) => {
                log::error!("Failed to disable hooks: {:?}", e);
            }
        }
    }

    MAPPING.write().unwrap().take(); // Clear mappings
    *ARCHIPELAGO_DATA.write().unwrap() = ArchipelagoData::default(); // Reset Data (Probably not needed)
    log::info!("Game restored to default state");
}
