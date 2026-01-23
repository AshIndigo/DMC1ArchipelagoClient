use crate::utilities::DMC1_ADDRESS;
use crate::{AP_CORE, create_hook};
use minhook::MH_STATUS;
use minhook::MinHook;
use randomizer_utilities::read_data_from_address;
use std::error::Error;
use std::fs;
use std::sync::{OnceLock, RwLock};

/// Pointer to where save file is in memory
const SAVE_FILE_PTR: usize = 0x5EAE78;
static SAVE_DATA: RwLock<Vec<u8>> = RwLock::new(vec![]);

pub fn get_save_path() -> Result<String, Box<dyn Error>> {
    if let Ok(core) = AP_CORE.get().unwrap().as_ref().lock()
        && let Some(client) = core.connection.client()
    {
        Ok(format!(
            "archipelago/dmc3_{}_{}.sav",
            client.seed_name(),
            client.this_player().name()
        ))
    } else {
        Err("Connecting unavailable".into())
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

pub(crate) fn add_hooks_to_list(addrs: &mut Vec<usize>) {
    addrs.push(LOAD_GAME_ADDR);
    addrs.push(SAVE_GAME_ADDR);
}

pub const SAVE_GAME_ADDR: usize = 0x443c0;
pub static ORIGINAL_SAVE_GAME: OnceLock<unsafe extern "C" fn(usize)> = OnceLock::new();

/// Called every frame while on the save game screen, if [param_1+2] == 2 though, then that means we're actually saving.
fn new_save_game(param_1: usize) {
    if let Some(original) = ORIGINAL_SAVE_GAME.get() {
        unsafe { original(param_1) }
    }
    match read_data_from_address::<u8>(param_1 + 2) {
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
    log::debug!(
        "Loading save slot selection screen!: {:#X} - {:#X}",
        param_1,
        param_2
    );
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
        let res = unsafe { original(param_1, param_2) };
        log::debug!("Loading game: {:#X}", param_1);
        res
    } else {
        panic!("Original Load game address not found");
    }
}

/// Get the save data to store in the SAVE_DATA global
fn get_save_data() -> Result<(), Box<dyn Error>> {
    *SAVE_DATA.write()? = fs::read(get_save_path()?)?;
    Ok(())
}
