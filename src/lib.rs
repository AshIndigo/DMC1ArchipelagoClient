use crate::archipelago::ArchipelagoCore;
use crate::constants::{BasicNothingFunc, DMC1Config};
use crate::utilities::{DMC1_ADDRESS, is_ddmk_loaded};
use archipelago_rs::{Connection, ConnectionOptions, ItemHandling};
use minhook::{MH_STATUS, MinHook};
use randomizer_utilities::dmc::dmc_constants::GameConfig;
use randomizer_utilities::exception_handler;
use std::sync::{Arc, Mutex, OnceLock};
use std::{panic, thread};
use windows::Win32::Foundation::HINSTANCE;
use windows::core::BOOL;

mod archipelago;
mod check_handler;
mod compat;
mod config;
mod constants;
mod data;
mod game_manager;
mod hook;
mod location_handler;
mod mapping;
mod save_handler;
mod skill_manager;
mod ui;
mod utilities;

#[macro_export]
/// Does not enable the hook, that needs to be done separately
macro_rules! create_hook {
    ($offset:expr, $detour:expr, $storage:ident, $name:expr) => {{
        let target = (*DMC1_ADDRESS + $offset) as *mut _;
        let detour_ptr = ($detour as *const ()) as *mut std::ffi::c_void;
        let original = MinHook::create_hook(target, detour_ptr)?;
        $storage
            .set(std::mem::transmute(original))
            .expect(concat!($name, " hook already set"));
    }};
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    fdw_reason: u32,
    _lpv_reserved: *mut std::os::raw::c_void,
) -> BOOL {
    const DLL_PROCESS_ATTACH: u32 = 1;
    const DLL_PROCESS_DETACH: u32 = 0;
    const DLL_THREAD_ATTACH: u32 = 2;
    const DLL_THREAD_DETACH: u32 = 3;

    match fdw_reason {
        DLL_PROCESS_ATTACH => {
            randomizer_utilities::setup_logger("dmc1_randomizer");
            panic::set_hook(Box::new(|info| {
                log::error!("Panic occurred: {info}");
            }));
            ui::dx11_hooks::setup_overlay();
            // Loader status
            thread::spawn(randomizer_utilities::dmc::loader_parser::set_loader_status);

            thread::spawn(|| {
                main_setup();
            });
        }
        DLL_PROCESS_DETACH => {
            // For cleanup
        }
        DLL_THREAD_ATTACH | DLL_THREAD_DETACH => {
            // Normally ignored if DisableThreadLibraryCalls is used
        }
        _ => {}
    }

    BOOL(1)
}

fn setup_main_loop_hook() -> Result<(), MH_STATUS> {
    unsafe {
        create_hook!(
            MAIN_LOOP_ADDR,
            main_loop_hook,
            MAIN_LOOP_ORIGINAL,
            "Main loop hook"
        );
        MinHook::enable_hook((*DMC1_ADDRESS + MAIN_LOOP_ADDR) as *mut _)?;
    }
    Ok(())
}

pub static AP_CORE: OnceLock<Arc<Mutex<ArchipelagoCore>>> = OnceLock::new();

static MAIN_LOOP_ORIGINAL: OnceLock<BasicNothingFunc> = OnceLock::new();
const MAIN_LOOP_ADDR: usize = 0x262660;
fn main_loop_hook() {
    // Run original game code
    if let Some(func) = MAIN_LOOP_ORIGINAL.get() {
        unsafe {
            func();
        }
    }

    if !config::CONFIG.connections.disable_auto_connect
        && let Ok(mut core) = AP_CORE
            .get_or_init(|| {
                ArchipelagoCore::new(
                    config::CONFIG.connections.get_url(),
                    DMC1Config::GAME_NAME.parse().unwrap(),
                )
                .map(|core| Arc::new(Mutex::new(core)))
                .unwrap()
            })
            .lock()
        && let Err(err) = core.update()
    {
        log::error!("{}", err);
        log::debug!("Attempting to reconnect");
        core.connection = Connection::new(
            config::CONFIG.connections.get_url(),
            DMC1Config::GAME_NAME,
            "",
            ConnectionOptions::new().receive_items(ItemHandling::OtherWorlds {
                own_world: true,
                starting_inventory: true,
            }),
        );
    }
}

fn main_setup() {
    exception_handler::install_exception_handler("dmc1_randomizer_latest.log");
    if is_ddmk_loaded() {
        log::info!("DDMK is loaded!");
        compat::ddmk_hook::setup_ddmk_hook();
    } else {
        log::info!("DDMK is not loaded!");
    }
    log::info!("DMC1 Base Address is: {:X}", *DMC1_ADDRESS);
    setup_main_loop_hook().unwrap();
}
