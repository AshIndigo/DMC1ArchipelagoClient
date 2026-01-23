use crate::compat::ddmk_hook::EVA_ADDRESS;
use crate::compat::inputs;
use crate::constants::BasicNothingFunc;
use imgui_sys::{ImGuiCond, ImGuiWindowFlags, ImVec2, cty};
use std::os::raw::c_char;
use std::sync::OnceLock;

pub type ImGuiBegin =
    extern "C" fn(name: *const cty::c_char, p_open: *mut bool, flags: ImGuiWindowFlags) -> bool;
pub type ImGuiButton = extern "C" fn(label: *const cty::c_char, size: &ImVec2) -> bool;
pub type ImGuiText = extern "C" fn(text: *const cty::c_char, text_end: *const cty::c_char);
pub type ImGuiNextWindowPos = extern "C" fn(pos: &ImVec2, cond: ImGuiCond, pivot: &ImVec2);

pub const BEGIN_FUNC_ADDR: usize = 0xb3d0;
pub const END_FUNC_ADDR: usize = 0x10a60;

pub const BUTTON_ADDR: usize = 0x4750;
// 5cd0
pub const TEXT_ADDR: usize = 0x4c8b0;

pub const NEXT_POS_FUNC_ADDR: usize = 0x208f0;

pub fn input_rs<T: AsRef<str>>(label: T, buf: &mut String) {
    inputs::InputText::new(label, buf).build();
}

pub fn text<T: AsRef<str>>(text: T) {
    let s = text.as_ref();
    unsafe {
        let start = s.as_ptr();
        let end = start.add(s.len());
        std::mem::transmute::<usize, ImGuiText>(*EVA_ADDRESS + TEXT_ADDR)(
            start as *const c_char,
            end as *const c_char,
        );
    }
}

static IMGUI_END: OnceLock<BasicNothingFunc> = OnceLock::new();
static IMGUI_BEGIN: OnceLock<ImGuiBegin> = OnceLock::new();
static IMGUI_BUTTON: OnceLock<ImGuiButton> = OnceLock::new();
static IMGUI_POS: OnceLock<ImGuiNextWindowPos> = OnceLock::new();

// Helpers to retrieve values
pub fn get_imgui_end() -> &'static BasicNothingFunc {
    IMGUI_END.get_or_init(|| unsafe {
        std::mem::transmute::<_, BasicNothingFunc>(*EVA_ADDRESS + END_FUNC_ADDR)
    })
}

pub fn get_imgui_begin() -> &'static ImGuiBegin {
    IMGUI_BEGIN.get_or_init(|| unsafe {
        std::mem::transmute::<_, ImGuiBegin>(*EVA_ADDRESS + BEGIN_FUNC_ADDR)
    })
}

pub fn get_imgui_button() -> &'static ImGuiButton {
    IMGUI_BUTTON.get_or_init(|| unsafe {
        std::mem::transmute::<_, ImGuiButton>(*EVA_ADDRESS + BUTTON_ADDR)
    })
}

pub fn get_imgui_next_pos() -> &'static ImGuiNextWindowPos {
    IMGUI_POS.get_or_init(|| unsafe {
        std::mem::transmute::<_, ImGuiNextWindowPos>(*EVA_ADDRESS + NEXT_POS_FUNC_ADDR)
    })
}
