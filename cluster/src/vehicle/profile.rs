//! Yamaha XSR900 GP vehicle profile — Sigma Racer product constants.

use sigma_instrumentation::GaugeScale;

/// Tuned triple calibration for the Sigma Racer demo / product line.
#[allow(dead_code)]
pub struct VehicleProfile {
    pub idle_rpm: f32,
    pub rev_limit_rpm: f32,
    pub max_speed_kmh: f32,
    pub redline_rpm: f32,
    /// Full-scale RPM on the tach sweep (may exceed the rev limit for headroom).
    pub gauge_max_rpm: f32,
}

impl VehicleProfile {
    /// The tach/speed gauge ranges for this vehicle.
    pub fn gauge_scale(&self) -> GaugeScale {
        GaugeScale::new(self.gauge_max_rpm, self.redline_rpm)
    }
}
