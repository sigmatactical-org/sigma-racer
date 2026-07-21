//! Wall-clock readout on the dashboard header.

use chrono::Local;
use sigma_instrumentation::SigmaDashboard;
use slint::ComponentHandle;
use slint::SharedString;
use std::time::Duration;

/// Start the 1 Hz wall-clock timer that feeds the status-bar time.
pub fn start(ui: &SigmaDashboard) {
    let ui_weak = ui.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(1),
        move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_clock(SharedString::from(Local::now().format("%H:%M").to_string()));
            }
        },
    );
}
