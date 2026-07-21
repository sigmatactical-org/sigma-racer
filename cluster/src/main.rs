//! Sigma Racer — production instrument cluster for Wingman / i.MX 8M Plus.
//!
//! Live vehicle data replaces the idle loop via CAN-FD from the M7 safety core.

#![forbid(unsafe_code)]

mod app;
mod connectivity;
mod log;
mod telemetry;
mod vehicle;

use sigma_instrumentation::{
    DisplayConfig, SigmaDashboard, UpdatesConfig, configure_window, ensure_panel_geometry,
    force_panel_scale_factor, init_gauge_art, start_signal_blink, start_updates_client, theme,
};
use slint::ComponentHandle;
use std::time::Duration;

use crate::vehicle::XSR900_GP;

fn main() -> Result<(), slint::PlatformError> {
    // Must run before the window is created — winit reads SLINT_SCALE_FACTOR
    // during adapter setup. ensure_panel_geometry then letterboxes 800×480.
    force_panel_scale_factor();

    let ui = SigmaDashboard::new()?;
    let gauge = XSR900_GP.gauge_scale();
    let kiosk = cfg!(sigma_racer_wingman_embedded)
        || std::env::var("SLINT_FULLSCREEN")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    theme::init_from_env(&ui);
    configure_window(&ui, DisplayConfig::embedded(kiosk));
    init_gauge_art(&ui, &gauge);

    ui.set_current_window(0);
    app::push_idle(&ui);
    telemetry::attach(&ui);
    app::start_clock(&ui);
    let _signal_blink = start_signal_blink(&ui);
    start_updates_client(&ui, UpdatesConfig::from_env());

    // Weston/winit may remap the surface after first map — letterbox again.
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(100), move || {
        if let Some(ui) = weak.upgrade() {
            ensure_panel_geometry(&ui, kiosk);
        }
    });
    // One more pass after Weston settles (mode / scale).
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(500), move || {
        if let Some(ui) = weak.upgrade() {
            ensure_panel_geometry(&ui, kiosk);
        }
    });

    ui.run()
}
