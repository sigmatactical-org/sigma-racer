//! Fallback values until the first Snapshot arrives from sigma-racer-vehicle.

use sigma_instrumentation::{ClusterTelemetry, SigmaDashboard, apply_telemetry};

use crate::vehicle::XSR900_GP;

/// Push the idle (engine-off) telemetry snapshot into the UI.
pub fn push(ui: &SigmaDashboard) {
    let gauge = XSR900_GP.gauge_scale();
    let mut msg = ClusterTelemetry::idle();
    msg.rpm = XSR900_GP.idle_rpm;
    msg.signals_live = false;
    apply_telemetry(ui, &msg, &gauge);
    ui.set_nav_blocked_hint(false);
}
