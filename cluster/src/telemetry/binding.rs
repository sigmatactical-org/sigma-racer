//! Map producer [`VehicleState`] → UI [`ClusterTelemetry`].

use sigma_instrumentation::{ClusterTelemetry, GaugeScale, SigmaDashboard, apply_telemetry};
use sigma_racer_telemetry::VehicleState;

use crate::vehicle::XSR900_GP;

/// Convert decoded vehicle state into the UI-facing formatted message.
pub fn to_cluster(state: &VehicleState) -> ClusterTelemetry {
    ClusterTelemetry {
        speed_kmh: state.speed,
        rpm: state.rpm,
        gear: state.gear,
        at_redline: state.at_redline,
        side_stand: state.side_stand,
        riding_mode: state.riding_mode.clone(),
        fuel_pct: state.fuel_pct,
        coolant_c: state.coolant_c,
        oil_c: state.oil_c,
        odometer: state.odometer,
        trip1: state.trip1,
        trip2: state.trip2,
        lean_angle: state.lean_angle,
        gforce: state.gforce,
        battery_v: state.battery_v,
        can_load: state.can_load,
        dtc: state.dtc,
        abs_active: state.abs_active,
        tc_active: state.tc_active,
        heading: state.heading,
        elevation: state.elevation,
        signals_live: state.signals_live,
    }
}

/// Push vehicle state through the shared instrumentation binding.
pub fn apply_state(ui: &SigmaDashboard, state: &VehicleState) {
    let gauge = XSR900_GP.gauge_scale();
    apply_state_with_gauge(ui, state, &gauge);
}

/// Apply `state` to the UI together with the vehicle profile's gauge scale.
pub fn apply_state_with_gauge(ui: &SigmaDashboard, state: &VehicleState, gauge: &GaugeScale) {
    apply_telemetry(ui, &to_cluster(state), gauge);
}
