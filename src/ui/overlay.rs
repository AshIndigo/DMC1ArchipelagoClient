use crate::archipelago::CONNECTED;
use crate::{mapping, utilities};
use randomizer_utilities::dmc::loader_parser::LOADER_STATUS;
use randomizer_utilities::ui::dx11_state::{D3D11State, get_resources, update_screen_size};
use randomizer_utilities::ui::dx11_state_guard;
use randomizer_utilities::ui::dx11_types::ORIGINAL_PRESENT;
use randomizer_utilities::ui::font_handler::{
    FontAtlas, FontColorCB, GREEN, RED, WHITE, draw_string,
};
use randomizer_utilities::ui::overlay_messages::{
    ACTIVE_MESSAGES, draw_colored_message, pop_buffer_message,
};
use std::sync::RwLockReadGuard;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;

pub(crate) static CANT_PURCHASE: AtomicBool = AtomicBool::new(false);
pub(crate) unsafe extern "system" fn present_hook(
    orig_swap_chain: IDXGISwapChain,
    sync_interval: u32,
    flags: u32,
) -> i32 {
    let (screen_width, screen_height) = update_screen_size(&orig_swap_chain);
    let state = get_resources(&orig_swap_chain);
    match state.read() {
        Ok(state) => {
            let original_state = dx11_state_guard::DX11OverlayBackup::new(&state.context);
            draw_overlay(screen_width, screen_height, &state);
            original_state.restore(&state.context);
        }
        Err(err) => {
            log::error!("Failed to get resources: {:?}", err);
        }
    }

    unsafe { ORIGINAL_PRESENT.get().unwrap()(orig_swap_chain, sync_interval, flags) }
}

fn draw_overlay(screen_width: f32, screen_height: f32, state: &RwLockReadGuard<D3D11State>) {
    unsafe {
        state
            .context
            .OMSetRenderTargets(Some(std::slice::from_ref(&state.rtv)), None);
        state.context.RSSetViewports(Some(&[D3D11_VIEWPORT {
            TopLeftX: 0.0,
            TopLeftY: 0.0,
            Width: screen_width,
            Height: screen_height,
            MinDepth: 0.0,
            MaxDepth: 1.0,
        }]));
    }

    if (utilities::is_on_main_menu() || should_display_anyway())
        && let Some(atlas) = &state.atlas
    {
        const STATUS: &str = "Status: ";
        draw_string(
            state,
            STATUS,
            0.0,
            0.0,
            screen_width,
            screen_height,
            get_default_color(),
        );
        let connected = CONNECTED.load(Ordering::SeqCst);
        draw_string(
            state,
            if connected {
                "Connected"
            } else {
                "Disconnected"
            },
            STATUS.chars().map(|c| atlas.glyph_advance(c)).sum::<f32>(),
            0.0,
            screen_width,
            screen_height,
            &if connected { GREEN } else { RED },
        );
        draw_version_info(state, screen_width, screen_height, atlas);
    }
    if CANT_PURCHASE.load(Ordering::SeqCst)
        && let Some(atlas) = &state.atlas
    {
        // TODO Modify this text
        const NO_PURCHASE: &str = "Cannot purchase upgrades";
        const NO_PURCHASE_L2: &str = "due to world settings";
        draw_string(
            state,
            NO_PURCHASE,
            480.0
                + (NO_PURCHASE
                    .chars()
                    .map(|c| atlas.glyph_advance(c))
                    .sum::<f32>()
                    / 2.0),
            70.0,
            screen_width,
            screen_height,
            &WHITE,
        );
        draw_string(
            state,
            NO_PURCHASE_L2,
            480.0
                + (NO_PURCHASE
                    .chars()
                    .map(|c| atlas.glyph_advance(c))
                    .sum::<f32>()
                    / 2.0),
            106.0,
            screen_width,
            screen_height,
            &WHITE,
        );
        CANT_PURCHASE.store(false, Ordering::SeqCst);
    }

    pop_buffer_message();

    let now = Instant::now();
    if let Ok(mut active) = ACTIVE_MESSAGES.lock() {
        // If it hasn't expired, keep it around
        active.retain(|msg| msg.expiration > now);

        const PADDING: f32 = 12.0;
        const LINE_HEIGHT: f32 = 24.0;

        let mut y = PADDING;
        for msg in active.iter().rev() {
            draw_colored_message(state, msg, screen_width, screen_height, y);
            y += LINE_HEIGHT + PADDING;
        }
    }
}

fn draw_version_info(
    state: &RwLockReadGuard<D3D11State>,
    screen_width: f32,
    screen_height: f32,
    atlas: &FontAtlas,
) {
    const MOD_VERSION: &str = "Mod Version:";
    const AP_VERSION: &str = "AP Client Version:";
    const ROOM_VERSION: &str = "Room Version:";
    const GAME_VERSION: &str = "Game Version:";
    const ADDITIONAL_MODS: &str = "Additional Mods:";
    // TODO Maybe at some point I'd want to have the mod poke github on launch?
    draw_string(
        state,
        &format!("{} {}", MOD_VERSION, env!("CARGO_PKG_VERSION")),
        0.0,
        //VERSION.chars().map(|c| atlas.glyph_advance(c)).sum::<f32>(),
        100.0,
        screen_width,
        screen_height,
        get_default_color(),
    );

    if CONNECTED.load(Ordering::SeqCst)
        && let Ok(mapping) = mapping::OVERLAY_INFO.read()
    {
        if let Some(cv) = &mapping.client_version {
            draw_string(
                state,
                &format!("{} {}", AP_VERSION, cv),
                0.0,
                //VERSION.chars().map(|c| atlas.glyph_advance(c)).sum::<f32>(),
                150.0,
                screen_width,
                screen_height,
                get_default_color(),
            );
        }
        if let Some(gv) = &mapping.generated_version {
            draw_string(
                state,
                &format!("{} {}", ROOM_VERSION, gv),
                0.0,
                //VERSION.chars().map(|c| atlas.glyph_advance(c)).sum::<f32>(),
                200.0,
                screen_width,
                screen_height,
                get_default_color(),
            );
        }
    }
    if let Some(status) = LOADER_STATUS.get() {
        draw_string(
            state,
            GAME_VERSION,
            0.0,
            250.0,
            screen_width,
            screen_height,
            &WHITE,
        );
        draw_string(
            state,
            &format!(" {}", status.game_information.description),
            GAME_VERSION
                .chars()
                .map(|c| atlas.glyph_advance(c))
                .sum::<f32>(),
            250.0,
            screen_width,
            screen_height,
            if status.game_information.valid_for_use {
                &GREEN
            } else {
                &RED
            },
        );
        draw_string(
            state,
            ADDITIONAL_MODS,
            0.0,
            300.0,
            screen_width,
            screen_height,
            &WHITE,
        );
        for (i, mod_info) in status.mod_information.iter().enumerate() {
            let base = 350;
            draw_string(
                state,
                mod_info.description,
                0.0,
                (base + (i * 50)) as f32,
                screen_width,
                screen_height,
                if mod_info.valid_for_use { &GREEN } else { &RED },
            );
        }
    }
}

fn get_default_color() -> &'static FontColorCB {
    &WHITE
}

fn should_display_anyway() -> bool {
    // TODO Use this to display if we are connected, then disconnected
    // Or if version mismatch?

    false
}
