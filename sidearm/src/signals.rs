//! The shared M7 signal dictionary in physical units.

use crate::PerformanceMode;

/// Neutral, `Copy` snapshot of the M7 signal dictionary in physical units.
///
/// This is the shared contract type: the codec decodes frames into it and
/// encodes frames from it, independent of any consumer's own state model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct M7Signals {
    pub engine_rpm: f32,
    pub coolant_c: i16,
    pub oil_c: i16,
    pub redline: bool,
    pub throttle_pct: f32,
    pub gear: i8,
    pub performance_mode: PerformanceMode,
    pub side_stand: bool,
    pub ground_speed: f32,
    pub lean_angle: f32,
    pub long_accel: f32,
    pub fuel_pct: f32,
    pub battery_v: f32,
    pub can_load: u8,
    pub abs_active: bool,
    pub tc_active: bool,
    pub dtc_count: u8,
    pub odometer: f32,
    pub trip1: f32,
    pub trip2: f32,
}

impl Default for M7Signals {
    fn default() -> Self {
        Self {
            engine_rpm: 0.0,
            coolant_c: 0,
            oil_c: 0,
            redline: false,
            throttle_pct: 0.0,
            gear: 0,
            performance_mode: PerformanceMode::Std,
            side_stand: false,
            ground_speed: 0.0,
            lean_angle: 0.0,
            long_accel: 0.0,
            fuel_pct: 0.0,
            battery_v: 0.0,
            can_load: 0,
            abs_active: false,
            tc_active: false,
            dtc_count: 0,
            odometer: 0.0,
            trip1: 0.0,
            trip2: 0.0,
        }
    }
}
