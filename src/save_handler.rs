use crate::archipelago::CONNECTED;
use crate::game_manager::{ARCHIPELAGO_DATA, ArchipelagoData};
use crate::utilities::DMC1_ADDRESS;
use crate::{AP_CORE, archipelago, create_hook};
use minhook::MH_STATUS;
use minhook::MinHook;
use randomizer_utilities::item_sync::{CURRENT_INDEX, SlotSyncInfo};
use randomizer_utilities::{item_sync, read_data_from_address};
use std::error::Error;
use std::io::ErrorKind;
use std::ptr::write;
use std::sync::atomic::Ordering;
use std::sync::{OnceLock, RwLock};
use std::{fs, io};

/// Pointer to where save file is in memory
const SAVE_FILE_PTR: usize = 0x60afc8;
static SAVE_DATA: RwLock<[u8; SAVE_LENGTH]> = RwLock::new([0; SAVE_LENGTH]);
const SAVE_LENGTH: usize = 0x5F64;

pub fn get_save_path() -> Result<String, Box<dyn Error>> {
    if let Ok(core) = AP_CORE.get().unwrap().as_ref().lock()
        && let Some(client) = core.connection.client()
    {
        Ok(format!(
            "archipelago/dmc1_{}_{}.sav",
            client.seed_name(),
            client.this_player().name()
        ))
    } else {
        Err("Connection unavailable".into())
    }
}

pub fn setup_save_hooks() -> Result<(), MH_STATUS> {
    log::debug!("Setting up save file related hooks");
    unsafe {
        create_hook!(
            LOAD_GAME_ADDR,
            new_load_game,
            ORIGINAL_LOAD_GAME,
            "Load game"
        );
        create_hook!(
            LOAD_SLOT_ADDR,
            load_save_slot,
            ORIGINAL_LOAD_SLOT,
            "Load save slot"
        );
        create_hook!(
            SAVE_GAME_ADDR,
            new_save_game,
            ORIGINAL_SAVE_GAME,
            "Save game"
        );
        create_hook!(
            SAVE_SLOT_ADDR,
            save_to_slot,
            ORIGINAL_SAVE_SLOT,
            "Save to slot"
        );
    }
    Ok(())
}

pub(crate) fn add_hooks_to_list(addrs: &mut Vec<usize>) {
    addrs.push(LOAD_GAME_ADDR);
    addrs.push(SAVE_GAME_ADDR);
    addrs.push(LOAD_SLOT_ADDR);
}

pub const SAVE_GAME_ADDR: usize = 0x443c0;
pub static ORIGINAL_SAVE_GAME: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();

/// Called every frame while on the save game screen, if [param_1+2] == 2 though, then that means we're actually saving.
fn new_save_game(param_1: usize) {
    if let Some(original) = ORIGINAL_SAVE_GAME.get() {
        unsafe { original(param_1) }
    }
    if read_data_from_address::<u8>(param_1 + 2) == 2 {
        unsafe {
            //let save_file_ptr = (param_1 + 0x70) as *const usize;
            let save_file = (param_1 + 0x70) as *const u8;

            let data = std::slice::from_raw_parts(save_file, SAVE_LENGTH).to_vec();

            fs::write(get_save_path().expect("Unable to get save path"), data)
                .expect("Unable to save game");
        }
    }
}

pub const LOAD_GAME_ADDR: usize = 0x24860;
pub static ORIGINAL_LOAD_GAME: OnceLock<
    unsafe extern "C" fn(param_1: usize, param_2: usize) -> i64,
> = OnceLock::new();

/// Hook for the games load game method
/// Triggers everytime the 10 save slots are displayed. Also ran when first loading the game
fn new_load_game(param_1: usize, param_2: usize) -> i64 {
    log::debug!(
        "Loading save slot selection screen!: {:#X} - {:#X}",
        param_1,
        param_2
    );
    if CONNECTED.load(Ordering::SeqCst) {
        return match get_save_data() {
            Ok(_) => {
                unsafe {
                    write(
                        (read_data_from_address::<usize>(*DMC1_ADDRESS + SAVE_FILE_PTR) + 0x70)
                            as *mut [u8; SAVE_LENGTH],
                        *SAVE_DATA.read().unwrap(),
                    );
                }
                1
            }
            Err(err) => {
                match err.downcast::<io::Error>() {
                    Ok(err) => match err.kind() {
                        ErrorKind::NotFound => {}
                        _ => {
                            log::error!("Error getting save data: {}", err);
                        }
                    },
                    Err(failed) => {
                        log::error!("Error getting save data: {}", failed);
                    }
                }
                -1
            }
        };
    }
    if let Some(original) = ORIGINAL_LOAD_GAME.get() {
        let res = unsafe { original(param_1, param_2) };
        log::debug!("Loading game: {:#X}", param_1);
        res
    } else {
        panic!("Original Load game address not found");
    }
}

/// Get the save data to store in the SAVE_DATA global
fn get_save_data() -> Result<(), Box<dyn Error>> {
    *SAVE_DATA.write()? = <[u8; 24420]>::try_from(fs::read(get_save_path()?)?).unwrap();
    Ok(())
}

const LOAD_SLOT_ADDR: usize = 0x255a10;
pub static ORIGINAL_LOAD_SLOT: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();
fn load_save_slot(param_1: usize) {
    let save_index = read_data_from_address::<u8>(
        read_data_from_address::<usize>(*DMC1_ADDRESS + SAVE_FILE_PTR) + 6,
    );
    log::debug!("Loading save slot: {}", save_index);
    if let Some(original) = ORIGINAL_LOAD_SLOT.get() {
        unsafe { original(param_1) }
    } else {
        panic!("Load save slot not found");
    }
    match AP_CORE.get().unwrap().lock() {
        Ok(mut core) => {
            let client = core.connection.client_mut().unwrap();
            match item_sync::read_save_data() {
                Ok(sync_data) => {
                    match sync_data.room_sync_info.get(&item_sync::get_sync_file_key(
                        client.seed_name(),
                        client.this_player().name().into(),
                    )) {
                        None => {
                            // Doesn't exist so 0
                            CURRENT_INDEX.store(0, Ordering::SeqCst);
                        }
                        Some(arr) => {
                            CURRENT_INDEX
                                .store(arr.sync_index[save_index as usize], Ordering::SeqCst);
                            *ARCHIPELAGO_DATA.write().unwrap() = ArchipelagoData::default();
                            if let Err(e) = archipelago::handle_received_items_packet(
                                arr.sync_index[save_index as usize] as usize,
                                client,
                            ) {
                                log::error!("Failed to handle received items: {:?}", e);
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("Error getting sync data: {}", err);
                }
            }
        }
        Err(err) => {
            log::error!("Error locking core while writing sync data: {}", err);
        }
    }
}

pub const SAVE_SLOT_ADDR: usize = 0x255220;
pub static ORIGINAL_SAVE_SLOT: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();
fn save_to_slot(param_1: usize) {
    let save_index = read_data_from_address::<u8>(
        read_data_from_address::<usize>(*DMC1_ADDRESS + SAVE_FILE_PTR) + 6,
    );
    if let Some(orig) = ORIGINAL_SAVE_SLOT.get() {
        unsafe {
            orig(param_1);
        }
    }
    log::debug!("Saving to slot {}", save_index);
    match AP_CORE.get().unwrap().lock() {
        Ok(core) => {
            let client = core.connection.client().unwrap();
            match item_sync::read_save_data() {
                Ok(mut sync_data) => {
                    let key = item_sync::get_sync_file_key(
                        client.seed_name(),
                        client.this_player().name().into(),
                    );
                    match sync_data.room_sync_info.get_mut(&key) {
                        None => {
                            // Doesn't exist, need to add
                            let mut sync_info = SlotSyncInfo::default();
                            sync_info.sync_index[save_index as usize] =
                                CURRENT_INDEX.load(Ordering::SeqCst);
                            sync_info.offline_checks =
                                item_sync::OFFLINE_CHECKS.lock().unwrap().clone();
                            sync_data.room_sync_info.insert(key, sync_info);
                        }
                        Some(sync_info) => {
                            sync_info.sync_index[save_index as usize] =
                                CURRENT_INDEX.load(Ordering::SeqCst);
                            sync_info.offline_checks =
                                item_sync::OFFLINE_CHECKS.lock().unwrap().clone();
                        }
                    }
                    item_sync::OFFLINE_CHECKS.lock().unwrap().clear();
                    if let Err(e) = item_sync::write_sync_data_file(sync_data) {
                        log::error!("Error writing sync data: {}", e);
                    }
                }
                Err(err) => {
                    log::error!("Error getting sync data: {}", err);
                }
            }
        }
        Err(err) => {
            log::error!("Error locking core while writing sync data: {}", err);
        }
    }
}
