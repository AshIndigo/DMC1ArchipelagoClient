use std::sync::atomic::{AtomicBool, Ordering};
use minhook::{MinHook, MH_STATUS};
use crate::check_handler;
use crate::check_handler::setup_check_hooks;
use crate::utilities::DMC1_ADDRESS;

static HOOKS_CREATED: AtomicBool = AtomicBool::new(false);

pub(crate) fn install_initial_functions() {
    if !HOOKS_CREATED.load(Ordering::SeqCst) {
        unsafe {
            match create_hooks() {
                Ok(_) => {
                    HOOKS_CREATED.store(true, Ordering::SeqCst);
                }
                Err(err) => {
                    log::error!("Failed to create hooks: {:?}", err);
                }
            }
        }
    }
    enable_hooks();
}


unsafe fn create_hooks() -> Result<(), MH_STATUS> {
    setup_check_hooks()?;
    Ok(())
}

fn enable_hooks() {
    let addresses: Vec<usize> = vec![
        check_handler::ITEM_PICKUP_ADDR
    ];
    addresses.iter().for_each(|addr| unsafe {
        match MinHook::enable_hook((*DMC1_ADDRESS + addr) as *mut _) {
            Ok(_) => {}
            Err(err) => {
                log::error!("Failed to enable {:#X} hook: {:?}", addr, err);
            }
        }
    })
}