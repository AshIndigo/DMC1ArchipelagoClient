use std::sync::atomic::Ordering;
use std::thread;
use windows::core::BOOL;
use windows::Win32::Foundation::HINSTANCE;
use archipelago_rs::protocol::ClientStatus;
use randomizer_utilities::archipelago_utilities::{connect_local_archipelago_proxy, CLIENT, SLOT_NUMBER, TEAM_NUMBER};
use randomizer_utilities::exception_handler;
use randomizer_utilities::ui_utilities::Status;
use crate::archipelago::TX_DEATHLINK;
use crate::bank::TX_BANK_MESSAGE;
use crate::check_handler::TX_LOCATION;
use crate::connection_manager::{CONNECTION_STATUS, TX_CONNECT, TX_DISCONNECT};
use crate::constants::DMC1Config;
use crate::utilities::is_ddmk_loaded;

mod constants;
mod utilities;
mod compat;
mod config;
mod archipelago;
mod connection_manager;
mod mapping;
mod hook;
mod game_manager;
mod check_handler;
mod skill_manager;
mod bank;
mod text_handler;
mod location_handler;
mod data;
mod save_handler;

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
        //log::debug!("{name} hook created", name = $name);
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
            //ui::dx11_hooks::setup_overlay();
            randomizer_utilities::setup_logger("dmc1_rando");
            //let loader_status = unsafe { get_loader_status() };
            //log::debug!("loader_status: {loader_status:?}");
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

fn main_setup() {
    exception_handler::install_exception_handler();
    if is_ddmk_loaded() {
        log::info!("DDMK is loaded!");
        compat::ddmk_hook::setup_ddmk_hook();
    } else {
        log::info!("DDMK is not loaded!");
    }
    log::info!("DMC1 Base Address is: {:X}", *utilities::DMC1_ADDRESS);
    thread::Builder::new()
        .name("Archipelago Client".to_string())
        .spawn(move || {
            spawn_archipelago_thread();
        })
        .expect("Failed to spawn arch thread");
}


#[tokio::main]
pub(crate) async fn spawn_archipelago_thread() {
    let mut setup = false;
    let mut rx_connect = randomizer_utilities::setup_channel_pair(&TX_CONNECT, None);
    let mut rx_deathlink = randomizer_utilities::setup_channel_pair(&TX_DEATHLINK, None);
    let mut rx_bank = randomizer_utilities::setup_channel_pair(&TX_BANK_MESSAGE, None);
    let mut rx_location = randomizer_utilities::setup_channel_pair(&TX_LOCATION, None);
    let mut rx_disconnect = randomizer_utilities::setup_channel_pair(&TX_DISCONNECT, None);


    if !config::CONFIG.connections.disable_auto_connect {
        thread::spawn(|| {
            log::debug!("Starting auto connector");
            connection_manager::auto_connect();
        });
    }
    loop {
        // Wait for a connection request
        let Some(item) = rx_connect.recv().await else {
            log::warn!("Connect channel closed, exiting Archipelago thread.");
            break;
        };

        log::info!("Processing connection request");
        let mut client_lock = CLIENT.lock().await;

        match connect_local_archipelago_proxy::<DMC1Config>(item).await {
            Ok(cl) => {
                client_lock.replace(cl);
                CONNECTION_STATUS.store(Status::Connected.into(), Ordering::SeqCst);
            }
            Err(err) => {
                log::error!("Failed to connect to Archipelago: {err}");
                client_lock.take(); // Clear the client
                CONNECTION_STATUS.store(Status::Disconnected.into(), Ordering::SeqCst);
                SLOT_NUMBER.store(-1, Ordering::SeqCst);
                TEAM_NUMBER.store(-1, Ordering::SeqCst);
                continue; // Try again on next connection request
            }
        }

        // Client is successfully connected
        if let Some(ref mut client) = client_lock.as_mut() {
            if !setup && let Err(err) = archipelago::run_setup(client).await {
                log::error!("{err}");
            }

            if let Err(e) = client.status_update(ClientStatus::ClientReady).await {
                log::error!("Status update failed: {e}");
            }
            // This blocks until a reconnect or disconnect is triggered
            archipelago::handle_things(
                client,
                &mut rx_location,
                &mut rx_bank,
                &mut rx_connect,
                &mut rx_deathlink,
                &mut rx_disconnect
            )
                .await;
        }
        CONNECTION_STATUS.store(Status::Disconnected.into(), Ordering::SeqCst);
        setup = false;
        // Allow reconnection immediately without delay
    }
}