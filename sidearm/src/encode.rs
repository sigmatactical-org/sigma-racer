//! [`M7Signals`] → frame encoding.

use dbc_rs::{Dbc, Error};

use crate::M7Signals;
use crate::message::{
    CHASSIS_ELECTRICAL, ENGINE_STATUS, THROTTLE_GEAR, TRIP_ODOMETER, WHEEL_SPEED,
};

/// Encode the full dictionary into its five CAN frames `(id, 8-byte payload)`.
///
/// Returns an error if any value is outside its DBC-defined range.
pub fn encode_frames(dbc: &Dbc, s: &M7Signals) -> Result<[(u32, [u8; 8]); 5], Error> {
    Ok([
        encode_one(dbc, ENGINE_STATUS, &engine_signals(s))?,
        encode_one(dbc, THROTTLE_GEAR, &throttle_signals(s))?,
        encode_one(dbc, WHEEL_SPEED, &wheel_signals(s))?,
        encode_one(dbc, CHASSIS_ELECTRICAL, &chassis_signals(s))?,
        encode_one(dbc, TRIP_ODOMETER, &trip_signals(s))?,
    ])
}

fn encode_one(dbc: &Dbc, id: u32, signals: &[(&str, f64)]) -> Result<(u32, [u8; 8]), Error> {
    let payload = dbc.encode(id, signals, false)?;
    let mut frame = [0u8; 8];
    let len = payload.len().min(8);
    frame[..len].copy_from_slice(&payload.as_slice()[..len]);
    Ok((id, frame))
}

fn engine_signals(s: &M7Signals) -> [(&'static str, f64); 4] {
    [
        ("EngineRPM", f64::from(s.engine_rpm)),
        ("CoolantTemp", f64::from(s.coolant_c)),
        ("OilTemp", f64::from(s.oil_c)),
        ("Redline", f64::from(u8::from(s.redline))),
    ]
}

fn throttle_signals(s: &M7Signals) -> [(&'static str, f64); 4] {
    [
        ("ThrottlePos", f64::from(s.throttle_pct)),
        ("CurrentGear", f64::from(s.gear.max(0))),
        ("PerformanceMode", f64::from(s.performance_mode.to_raw())),
        ("SideStand", f64::from(u8::from(s.side_stand))),
    ]
}

fn wheel_signals(s: &M7Signals) -> [(&'static str, f64); 3] {
    [
        ("GroundSpeed", f64::from(s.ground_speed)),
        ("LeanAngle", f64::from(s.lean_angle)),
        ("LongAccel", f64::from(s.long_accel)),
    ]
}

fn chassis_signals(s: &M7Signals) -> [(&'static str, f64); 6] {
    [
        ("FuelLevel", f64::from(s.fuel_pct)),
        ("BatteryVoltage", f64::from(s.battery_v)),
        ("CanBusLoad", f64::from(s.can_load)),
        ("AbsActive", f64::from(u8::from(s.abs_active))),
        ("TcActive", f64::from(u8::from(s.tc_active))),
        ("DtcCount", f64::from(s.dtc_count)),
    ]
}

fn trip_signals(s: &M7Signals) -> [(&'static str, f64); 3] {
    [
        ("Odometer", f64::from(s.odometer)),
        ("Trip1", f64::from(s.trip1)),
        ("Trip2", f64::from(s.trip2)),
    ]
}
