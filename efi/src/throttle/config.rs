//! Ride-by-wire monitor configuration and per-tick inputs.
//!
//! Sensor calibrations are ⚠ [MEASURE] — dual-sensor slopes and valid
//! windows come off the actual CP3 twistgrip and throttle body in mule
//! Phase 1 (items 8–9), never assumed.

mod rbw_inputs;
mod sensor_cal;
pub use rbw_inputs::RbwInputs;
pub use sensor_cal::SensorCal;

/// ⚠ [MEASURE] placeholder cal — a generic 0.5–4.5 V sensor. Replaced per
/// sensor by mule Phase-1 measurements.
pub const PLACEHOLDER_CAL: SensorCal = SensorCal {
    min_valid_v: 0.25,
    max_valid_v: 4.75,
    v_at_0: 0.5,
    v_at_100: 4.5,
};

/// Monitor configuration. Defaults carry ⚠ [MEASURE] placeholders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RbwConfig {
    pub app_a: SensorCal,
    pub app_b: SensorCal,
    pub tps_a: SensorCal,
    pub tps_b: SensorCal,
    /// Max |a − b| after normalization, percent points.
    pub pair_disagree_pct: f32,
    /// Disagreement must persist this long to trip (µs).
    pub pair_disagree_hold_us: u64,
    /// Max |commanded − actual| envelope, percent points.
    pub tracking_err_pct: f32,
    /// Tracking error must persist this long to trip (µs).
    pub tracking_hold_us: u64,
    /// Demand at or below this is "idle" for re-arm / start permit.
    pub idle_demand_pct: f32,
}

impl Default for RbwConfig {
    fn default() -> Self {
        Self {
            app_a: PLACEHOLDER_CAL,
            app_b: PLACEHOLDER_CAL,
            tps_a: PLACEHOLDER_CAL,
            tps_b: PLACEHOLDER_CAL,
            pair_disagree_pct: 5.0,
            pair_disagree_hold_us: 20_000,
            tracking_err_pct: 10.0,
            tracking_hold_us: 100_000,
            idle_demand_pct: 2.0,
        }
    }
}
