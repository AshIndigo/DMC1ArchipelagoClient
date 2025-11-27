use crate::utilities::DMC1_ADDRESS;
use crate::mapping::MAPPING;
use crate::{create_hook};
use minhook::MinHook;
use minhook::MH_STATUS;
use std::error::Error;
use std::io::ErrorKind;
use std::sync::{OnceLock, RwLock};
use std::{fs, io};
use randomizer_utilities::mapping_utilities::get_own_slot_name;
use randomizer_utilities::read_data_from_address;

/// Pointer to where save file is in memory
const SAVE_FILE_PTR: usize = 0x5EAE78;
static SAVE_DATA: RwLock<Vec<u8>> = RwLock::new(vec![]);

pub fn get_save_path() -> Result<String, io::Error> {
    // Load up the mappings to get the seed
    if let Some(mappings) = MAPPING.read().unwrap().as_ref() {
        Ok(format!("archipelago/dmc1_{}_{}.sav", &mappings.seed, get_own_slot_name().unwrap()))
    } else {
        Err(io::Error::other("Mappings not available"))
    }
}

pub fn get_new_save_path() -> Result<String, Box<dyn Error>> {
    // Load up the mappings to get the seed
    if let Some(mappings) = MAPPING.read()?.as_ref() {
        Ok(format!(
            "archipelago/dmc3_{}_{}.sav",
            &mappings.seed,
            &get_own_slot_name()?
        ))
    } else {
        Err("Mappings not available".into())
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
            SAVE_GAME_ADDR,
            new_save_game,
            ORIGINAL_SAVE_GAME,
            "Save game"
        );
    }
    Ok(())
}

pub unsafe fn disable_save_hooks(base_address: usize) -> Result<(), MH_STATUS> {
    log::debug!("Disabling save related hooks");
    unsafe {
        MinHook::disable_hook((base_address + LOAD_GAME_ADDR) as *mut _)?;
        MinHook::disable_hook((base_address + SAVE_GAME_ADDR) as *mut _)?;
    }
    Ok(())
}

pub const SAVE_GAME_ADDR: usize = 0x443c0;
pub static ORIGINAL_SAVE_GAME: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();

/// Called every frame while on the save game screen, if [param_1+2] == 2 though, then that means we're actually saving.
fn new_save_game(param_1: usize) {

    if let Some(original) = ORIGINAL_SAVE_GAME.get() {
        unsafe { original(param_1) }
    }
    match read_data_from_address::<u8>(param_1+2) {
        2 => {
            log::debug!("Saving game: {}", param_1);
        }
        _ => {}
    }
}

pub const LOAD_GAME_ADDR: usize = 0x24860;
pub static ORIGINAL_LOAD_GAME: OnceLock<
    unsafe extern "C" fn(param_1: usize, param_2: usize) -> usize,
> = OnceLock::new();

/// Hook for the games load game method
/// Triggers everytime the 10 save slots are displayed. Also ran when first loading the game
fn new_load_game(param_1: usize, param_2: usize) -> usize {
    log::debug!("Loading save slot selection screen!");
  /*  if CONNECTION_STATUS.load(Ordering::SeqCst) == 1 {
        return match get_save_data() {
            Ok(..) => {
                unsafe {
                    write(
                        (*DMC1_ADDRESS + SAVE_FILE_PTR) as *mut usize,
                        SAVE_DATA.read().unwrap().as_ptr().addr(),
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
    }*/
    if let Some(original) = ORIGINAL_LOAD_GAME.get() {
        unsafe { original(param_1, param_2) }
    } else {
        panic!("Original Load game address not found");
    }
}

/// Get the save data to store in the SAVE_DATA global
fn get_save_data() -> Result<(), Box<dyn Error>> {
    match fs::read(get_new_save_path()?) {
        Ok(bytes) => {
            *SAVE_DATA.write()? = bytes;
            Ok(())
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => match fs::read(get_save_path()?) {
                Ok(bytes) => {
                    *SAVE_DATA.write()? = bytes;
                    Ok(())
                }
                Err(err) => Err(Box::new(err)),
            },
            _ => Err(Box::new(err)),
        },
    }
}
