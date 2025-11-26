use std::collections::HashSet;
use crate::compat::imgui_bindings::{get_imgui_begin, get_imgui_button, get_imgui_end, get_imgui_next_pos, input_rs, text};
use crate::connection_manager::CONNECTION_STATUS;
use crate::constants::{BasicNothingFunc, ItemCategory};
use crate::{bank, config, constants, game_manager, utilities};
use imgui_sys::{ImGuiCond, ImGuiCond_Appearing, ImGuiWindowFlags, ImVec2};
use minhook::MinHook;
use randomizer_utilities::ui_utilities::get_status_text;
use randomizer_utilities::{get_base_address, read_data_from_address};
use std::os::raw::c_char;
use std::ptr::addr_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock};
use std::thread;
use crate::game_manager::{with_active_player_data, with_session_read, ItemData};

static SETUP: AtomicBool = AtomicBool::new(false);
pub static EVA_ADDRESS: LazyLock<usize> = LazyLock::new(|| get_base_address("Eva.dll"));

const MAIN_FUNC_ADDR: usize = 0x612a0;
const TIMESTEP_FUNC_ADDR: usize = 0x9900;
const DDMK_UI_ENABLED: usize = 0x829ba;

unsafe extern "C" fn hooked_timestep() {
    unsafe {
        if !SETUP.load(Ordering::SeqCst) {
            MinHook::enable_hook((*EVA_ADDRESS + MAIN_FUNC_ADDR) as _)
                .expect("Failed to enable hook");
            SETUP.store(true, Ordering::SeqCst);
        }
        match get_orig_timestep_func() {
            None => {
                panic!("ORIG_TIMESTEP_FUNC not initialized in hooked render");
            }
            Some(timestep_func) => {
                timestep_func();
            }
        }
    }
}

#[derive(Debug, Default)]
struct CustomDataHolder {
    category: String,
    id: String,
    count: String,
    hp_to_give: String,
}

impl CustomDataHolder {
    pub fn convert_to_data(&self) -> ItemData {
        ItemData {
            category: self.category.parse().unwrap_or(0),
            id: self.id.parse().unwrap_or(0),
            count: self.count.parse().unwrap_or(0),
        }
    }
}

static CUSTOM_ITEM: Mutex<CustomDataHolder> = Mutex::new(CustomDataHolder {
    category: String::new(),
    id: String::new(),
    count: String::new(),
    hp_to_give: String::new(),
});
unsafe extern "C" fn hooked_render() {
    unsafe {
        if !SETUP.load(Ordering::SeqCst) {
            return;
        }

        if !read_data_from_address::<bool>(DDMK_UI_ENABLED + *EVA_ADDRESS) {
            return;
        }

        archipelago_window(CUSTOM_ITEM.lock().unwrap()); // For the archipelago window
        tracking_window();
        bank_window();
        match get_orig_render_func() {
            None => {}
            Some(fnc) => {
                fnc();
            }
        }
    }
}

unsafe fn tracking_window() {
    unsafe {
        let flag = &mut true;
        get_imgui_next_pos()(
            &ImVec2 { x: 800.0, y: 320.0 }, // 300
            ImGuiCond_Appearing as ImGuiCond,
            &ImVec2 { x: 0.0, y: 0.0 },
        );
        get_imgui_begin()(
            c"Tracker".as_ptr() as *const c_char,
            flag as *mut bool,
            imgui_sys::ImGuiWindowFlags_AlwaysAutoResize as ImGuiWindowFlags,
        );

        match game_manager::ARCHIPELAGO_DATA.read() {
            Ok(data) => {
                for chunk in constants::get_items_by_category(ItemCategory::Key).chunks(3) {
                    let row_text = chunk
                        .iter()
                        .map(|&item| checkbox_text(item, &data.items))
                        .collect::<Vec<String>>()
                        .join("  ");
                    text(format!("{}\0", row_text));
                }
                text(format!(
                    "Blue Orbs: {}\0",
                    data.blue_orbs
                ));
                text(format!(
                    "Purple Orbs: {}\0",
                    data.purple_orbs
                ));
            }
            Err(err) => {
                log::error!("Failed to read ArchipelagoData: {:?}", err);
            }
        }

        get_imgui_end()();
    }
}

unsafe fn bank_window() {
    unsafe {
        let flag = &mut true;
        get_imgui_next_pos()(
            &ImVec2 { x: 800.0, y: 500.0 },
            ImGuiCond_Appearing as ImGuiCond,
            &ImVec2 { x: 0.0, y: 0.0 },
        );
        get_imgui_begin()(
            c"Bank".as_ptr() as *const c_char,
            flag as *mut bool,
            imgui_sys::ImGuiWindowFlags_AlwaysAutoResize as ImGuiWindowFlags,
        );
        let consumables = constants::get_items_by_category(ItemCategory::Consumable);
        for n in 0..constants::get_items_by_category(ItemCategory::Consumable).len() {
            // Special case for red orbs...
            let item = consumables.get(n).unwrap();
            text(format!(
                "{}: {}\0",
                item,
                bank::get_bank().read().unwrap().get(item).unwrap()
            ));
        }
        get_imgui_end()();
    }
}

fn checkbox_text(item: &str, list: &HashSet<&str>) -> String {
    format!("{} [{}]", item, if list.contains(&item) { "X" } else { " " })
}


unsafe fn archipelago_window(mut custom_item_data: MutexGuard<CustomDataHolder>) {
    unsafe {
        let flag = &mut true;
        get_imgui_next_pos()(
            &ImVec2 { x: 800.0, y: 100.0 },
            ImGuiCond_Appearing as ImGuiCond,
            &ImVec2 { x: 0.0, y: 0.0 },
        );
        get_imgui_begin()(
            c"Archipelago".as_ptr() as *const c_char,
            flag as *mut bool,
            imgui_sys::ImGuiWindowFlags_AlwaysAutoResize as ImGuiWindowFlags,
        );
        text(format!(
            "Status: {}\0",
            get_status_text(CONNECTION_STATUS.load(Ordering::SeqCst))
        ));
        const DEBUG: bool = true;
        if DEBUG {
            input_rs("Category\0", &mut custom_item_data.category);
            input_rs("ID\0", &mut custom_item_data.id);
            input_rs("Count\0", &mut custom_item_data.count);
            if get_imgui_button()(
                c"Give Custom Unique Item".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                let data = custom_item_data.convert_to_data();
                thread::spawn(move || {
                    log::debug!("Giving: {}", data);
                    utilities::insert_unique_item_into_inv(&data)
                });
            }
            if get_imgui_button()(
                c"Clear Item Slot".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                let data = custom_item_data.convert_to_data();
                thread::spawn(move || {
                    log::debug!("Clearing: {}", data);
                    utilities::clear_item_slot(&data)
                });
            }
            input_rs("Added HP\0", &mut custom_item_data.hp_to_give);
            if get_imgui_button()(
                "Modify Health\0".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                let val = custom_item_data.hp_to_give.parse().unwrap_or(0);
                log::debug!("Modify Health: {}", val);
                thread::spawn(move || {
                    //utilities::give_hp(val);
                });
            }
            if get_imgui_button()(
                "Expertise Info\0".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                thread::spawn(move || {
                    with_session_read(|s| {
                        log::info!("Expertise is: {:?}", s.expertise);
                        log::info!("Expertise loc is: {:?}", addr_of!(s.expertise))
                    }).unwrap();

                });
            }
            if get_imgui_button()(
                "Alter gun\0".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                thread::spawn(move || {
                    with_active_player_data(|data| {
                        data.gun = 6;
                    }).unwrap();
                });
            }
            if get_imgui_button()(
                "Alter melee\0".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                thread::spawn(move || {
                    with_active_player_data(|data| {
                        data.melee = 6;
                    }).unwrap();
                });
            }
        }
        get_imgui_end()();
    }
}

pub fn setup_ddmk_hook() {
    if !config::CONFIG.mods.disable_ddmk_hooks {
        log::info!("Starting up DDMK hook");
        log::info!("Eva base ADDR: {:X}", *EVA_ADDRESS);
        init_render_func();
        init_timestep_func();
        unsafe {
            MinHook::enable_hook((*EVA_ADDRESS + TIMESTEP_FUNC_ADDR) as _)
                .expect("Failed to enable timestep hook");
        }
        log::info!("DDMK hook initialized");
    } else {
        log::info!("DDMK is detected but hooks will not be enabled")
    }
}

static ORIG_RENDER_FUNC: OnceLock<Option<BasicNothingFunc>> = OnceLock::new();

fn init_render_func() {
    ORIG_RENDER_FUNC.get_or_init(|| {
        Some(unsafe {
            std::mem::transmute::<_, BasicNothingFunc>(
                MinHook::create_hook((*EVA_ADDRESS + MAIN_FUNC_ADDR) as _, hooked_render as _)
                    .expect("Failed to create hook"),
            )
        })
    });
}

fn get_orig_render_func() -> Option<BasicNothingFunc> {
    *ORIG_RENDER_FUNC.get().unwrap_or(&None)
}

static ORIG_TIMESTEP_FUNC: OnceLock<Option<BasicNothingFunc>> = OnceLock::new();

fn init_timestep_func() {
    ORIG_TIMESTEP_FUNC.get_or_init(|| {
        Some(unsafe {
            std::mem::transmute::<_, BasicNothingFunc>(
                MinHook::create_hook(
                    (*EVA_ADDRESS + TIMESTEP_FUNC_ADDR) as _,
                    hooked_timestep as _,
                )
                .expect("Failed to create timestep hook"),
            )
        })
    });
}

fn get_orig_timestep_func() -> Option<BasicNothingFunc> {
    *ORIG_TIMESTEP_FUNC.get().unwrap_or(&None)
}
