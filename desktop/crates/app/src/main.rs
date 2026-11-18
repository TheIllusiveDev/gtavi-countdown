//! GTA VI Countdown as a native Rust application.
//!
//! The window is drawn with egui on wgpu (Vulkan, Metal, DirectX 12 or
//! OpenGL ES). The logic that needs no user
//! interface lives in the `gtavi-core` crate, and the SNTP client is the
//! `timesync` crate in this workspace.

#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod backdrop;
mod clock;
mod countdown_view;
mod details;
mod icons;
mod platform;
mod screenshot;
mod settings_panel;
mod style;

use std::sync::Arc;

use eframe::egui;

fn main() -> eframe::Result {
    let state = platform::load_window_state();
    let icon = egui::IconData { rgba: icons::app_icon_rgba(256), width: 256, height: 256 };
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("GTA VI Countdown")
        .with_app_id("app.gtavi.countdown")
        .with_icon(Arc::new(icon))
        .with_min_inner_size([360.0, 240.0])
        .with_inner_size(state.map_or([1100.0, 760.0], |s| s.size));
    if let Some(state) = state {
        if let Some(position) = state.position {
            viewport = viewport.with_position(position);
        }
        viewport = viewport.with_maximized(state.maximized);
    }
    let options = eframe::NativeOptions {
        viewport,
        // wgpu picks Vulkan on Linux and Windows, Metal on macOS, and falls
        // back to DirectX 12 or OpenGL ES. WGPU_BACKEND overrides the choice.
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native("GTA VI Countdown", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}
