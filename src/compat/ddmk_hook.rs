use crate::archipelago::CONNECTED;
use crate::compat::imgui_bindings::input_rs;
use crate::constants::ItemCategory;
use crate::game_manager::{ItemData, with_active_player_data, with_session_read};
use crate::{config, constants, game_manager, utilities};
use imgui_sys::{ImGuiCond, ImGuiCond_Appearing, ImGuiWindowFlags, ImVec2};
use randomizer_utilities::dmc::common_ddmk;
use randomizer_utilities::dmc::common_ddmk::{
    SETUP, checkbox_text, get_orig_render_func, run_common_ddmk_code,
};
use randomizer_utilities::dmc::dmc_constants::DDMKHandler;
use randomizer_utilities::{get_base_address, read_data_from_address};
use std::os::raw::c_char;
use std::ptr::addr_of;
use std::sync::atomic::Ordering;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::thread;

pub static EVA_ADDRESS: LazyLock<usize> = LazyLock::new(|| get_base_address("Eva.dll"));
const DDMK_UI_ENABLED: usize = 0x829ba;

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
        common_ddmk::get_imgui_next_pos()(
            &ImVec2 { x: 800.0, y: 320.0 }, // 300
            ImGuiCond_Appearing as ImGuiCond,
            &ImVec2 { x: 0.0, y: 0.0 },
        );
        common_ddmk::get_imgui_begin()(
            c"Tracker".as_ptr() as *const c_char,
            flag as *mut bool,
            imgui_sys::ImGuiWindowFlags_AlwaysAutoResize as ImGuiWindowFlags,
        );

        match game_manager::ARCHIPELAGO_DATA.read() {
            Ok(data) => {
                for chunk in constants::get_items_by_category(ItemCategory::Key).chunks(3) {
                    let row_text = chunk
                        .iter()
                        .map(|&item| checkbox_text(&item.to_string(), &data.items))
                        .collect::<Vec<String>>()
                        .join("  ");
                    common_ddmk::text(format!("{}\0", row_text));
                }
                common_ddmk::text(format!("Blue Orbs: {}\0", data.blue_orbs));
                common_ddmk::text(format!("Purple Orbs: {}\0", data.purple_orbs));
            }
            Err(err) => {
                log::error!("Failed to read ArchipelagoData: {:?}", err);
            }
        }

        common_ddmk::get_imgui_end()();
    }
}

unsafe fn archipelago_window(mut custom_item_data: MutexGuard<CustomDataHolder>) {
    unsafe {
        let flag = &mut true;
        common_ddmk::get_imgui_next_pos()(
            &ImVec2 { x: 800.0, y: 100.0 },
            ImGuiCond_Appearing as ImGuiCond,
            &ImVec2 { x: 0.0, y: 0.0 },
        );
        common_ddmk::get_imgui_begin()(
            c"Archipelago".as_ptr() as *const c_char,
            flag as *mut bool,
            imgui_sys::ImGuiWindowFlags_AlwaysAutoResize as ImGuiWindowFlags,
        );
        common_ddmk::text(format!(
            "Status: {}\0",
            if CONNECTED.load(Ordering::SeqCst) {
                "Connected"
            } else {
                "Disconnected"
            }
        ));
        const DEBUG: bool = true;
        if DEBUG {
            input_rs("Category\0", &mut custom_item_data.category);
            input_rs("ID\0", &mut custom_item_data.id);
            input_rs("Count\0", &mut custom_item_data.count);
            if common_ddmk::get_imgui_button()(
                c"Give Custom Unique Item".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                let data = custom_item_data.convert_to_data();
                thread::spawn(move || {
                    log::debug!("Giving: {}", data);
                    utilities::insert_unique_item_into_inv(&data)
                });
            }
            if common_ddmk::get_imgui_button()(
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
            if common_ddmk::get_imgui_button()(
                c"Modify Health".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                let val = custom_item_data.hp_to_give.parse().unwrap_or(0);
                log::debug!("Modify Health: {}", val);
                thread::spawn(move || {
                    //utilities::give_hp(val);
                });
            }
            if common_ddmk::get_imgui_button()(
                c"Expertise Info".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                thread::spawn(move || {
                    with_session_read(|s| {
                        log::info!("Expertise is: {:?}", s.expertise);
                        log::info!("Expertise loc is: {:?}", addr_of!(s.expertise))
                    })
                    .unwrap();
                });
            }
            if common_ddmk::get_imgui_button()(
                c"Alter gun".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                thread::spawn(move || {
                    with_active_player_data(|data| {
                        data.gun = 6;
                    })
                    .unwrap();
                });
            }
            if common_ddmk::get_imgui_button()(
                c"Alter melee".as_ptr() as *const c_char,
                &ImVec2 { x: 0.0, y: 0.0 },
            ) {
                thread::spawn(move || {
                    with_active_player_data(|data| {
                        data.melee = 6;
                    })
                    .unwrap();
                });
            }
        }
        common_ddmk::get_imgui_end()();
    }
}

pub fn setup_ddmk_hook() {
    if !config::CONFIG.mods.disable_ddmk_hooks {
        log::info!("Starting up DDMK hook");
        log::info!("Eva base ADDR: {:X}", *EVA_ADDRESS);
        if common_ddmk::DDMK_INFO
            .set(DDMKHandler {
                ddmk_address: LazyLock::new(|| get_base_address("Eva.dll")),
                main_func_addr: 0x612a0,
                timestep_func_addr: 0x9900,
                ddmk_ui_enabled: DDMK_UI_ENABLED,
                hooked_render: hooked_render as _,
                text_addr: 0x4c8b0,
                end_addr: 0x10a60,
                begin_addr: 0xb3d0,
                button_addr: 0x4750,
                next_pos: 0x208f0,
            })
            .is_err()
        {
            log::error!("Failed to set DDMK info");
        }
        run_common_ddmk_code();
        log::info!("DDMK hook initialized");
    } else {
        log::info!("DDMK is detected but hooks will not be enabled")
    }
}
