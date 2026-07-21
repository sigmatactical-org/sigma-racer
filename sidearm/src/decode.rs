//! Frame → [`M7Signals`] decoding.

use dbc_rs::Dbc;

use crate::message::{
    CHASSIS_ELECTRICAL, ENGINE_STATUS, THROTTLE_GEAR, TRIP_ODOMETER, WHEEL_SPEED,
};
use crate::{M7Signals, PerformanceMode};

/// Decode one CAN frame into `out`, updating only the fields carried by this
/// message. Returns `false` if `id` is not in the dictionary or the frame does
/// not decode.
pub fn decode_into(dbc: &Dbc, id: u32, data: &[u8], out: &mut M7Signals) -> bool {
    let decoded = match dbc.decode(id, data, false) {
        Ok(signals) if !signals.is_empty() => signals,
        _ => return false,
    };
    for signal in decoded.iter() {
        apply(id, signal.name, signal.value, out);
    }
    true
}

fn apply(id: u32, name: &str, value: f64, out: &mut M7Signals) {
    match (id, name) {
        (ENGINE_STATUS, "EngineRPM") => out.engine_rpm = value as f32,
        (ENGINE_STATUS, "CoolantTemp") => out.coolant_c = round(value) as i16,
        (ENGINE_STATUS, "OilTemp") => out.oil_c = round(value) as i16,
        (ENGINE_STATUS, "Redline") => out.redline = value != 0.0,
        (THROTTLE_GEAR, "ThrottlePos") => out.throttle_pct = value as f32,
        (THROTTLE_GEAR, "CurrentGear") => out.gear = round(value) as i8,
        (THROTTLE_GEAR, "PerformanceMode") => {
            out.performance_mode = PerformanceMode::from_raw(round(value) as u8)
        }
        (THROTTLE_GEAR, "SideStand") => out.side_stand = value != 0.0,
        (WHEEL_SPEED, "GroundSpeed") => out.ground_speed = value as f32,
        (WHEEL_SPEED, "LeanAngle") => out.lean_angle = value as f32,
        (WHEEL_SPEED, "LongAccel") => out.long_accel = value as f32,
        (CHASSIS_ELECTRICAL, "FuelLevel") => out.fuel_pct = value as f32,
        (CHASSIS_ELECTRICAL, "BatteryVoltage") => out.battery_v = value as f32,
        (CHASSIS_ELECTRICAL, "CanBusLoad") => out.can_load = round(value) as u8,
        (CHASSIS_ELECTRICAL, "AbsActive") => out.abs_active = value != 0.0,
        (CHASSIS_ELECTRICAL, "TcActive") => out.tc_active = value != 0.0,
        (CHASSIS_ELECTRICAL, "DtcCount") => out.dtc_count = round(value) as u8,
        (TRIP_ODOMETER, "Odometer") => out.odometer = value as f32,
        (TRIP_ODOMETER, "Trip1") => out.trip1 = value as f32,
        (TRIP_ODOMETER, "Trip2") => out.trip2 = value as f32,
        _ => {}
    }
}

/// Round half-away-from-zero without `std` float intrinsics (`f64::round` is
/// unavailable under `no_std`). Adequate for the small, bounded signal values
/// in this dictionary.
#[inline]
fn round(value: f64) -> i64 {
    if value >= 0.0 {
        (value + 0.5) as i64
    } else {
        (value - 0.5) as i64
    }
}
