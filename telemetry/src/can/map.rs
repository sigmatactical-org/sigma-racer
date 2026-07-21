//! Map between [`VehicleState`] and neutral M7 CAN signals.

use crate::state::VehicleState;
use sigma_racer_sidearm::{M7Signals, PerformanceMode};

/// Project the VSS state onto the M7 wire signal set (encode direction).
pub fn to_signals(state: &VehicleState) -> M7Signals {
    M7Signals {
        engine_rpm: state.rpm,
        coolant_c: state.coolant_c,
        oil_c: state.oil_c,
        redline: state.at_redline,
        throttle_pct: state.throttle_pct,
        gear: state.gear,
        performance_mode: PerformanceMode::from_label(&state.riding_mode).unwrap_or_default(),
        side_stand: state.side_stand,
        ground_speed: state.speed,
        lean_angle: state.lean_angle,
        long_accel: state.gforce,
        fuel_pct: state.fuel_pct,
        battery_v: state.battery_v,
        can_load: state.can_load,
        abs_active: state.abs_active,
        tc_active: state.tc_active,
        dtc_count: state.dtc,
        odometer: state.odometer,
        trip1: state.trip1,
        trip2: state.trip2,
    }
}

/// Apply decoded M7 wire signals to the VSS state (decode direction).
pub fn from_signals(s: &M7Signals, state: &mut VehicleState) {
    state.rpm = s.engine_rpm;
    state.coolant_c = s.coolant_c;
    state.oil_c = s.oil_c;
    state.redline_can = s.redline;
    state.throttle_pct = s.throttle_pct;
    state.gear = s.gear;
    state.riding_mode = s.performance_mode.as_str().into();
    state.side_stand = s.side_stand;
    state.speed = s.ground_speed;
    state.lean_angle = s.lean_angle;
    state.gforce = s.long_accel;
    state.fuel_pct = s.fuel_pct;
    state.battery_v = s.battery_v;
    state.can_load = s.can_load;
    state.abs_active = s.abs_active;
    state.tc_active = s.tc_active;
    state.dtc = s.dtc_count;
    state.odometer = s.odometer;
    state.trip1 = s.trip1;
    state.trip2 = s.trip2;
}
