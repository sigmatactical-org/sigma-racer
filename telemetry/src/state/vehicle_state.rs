//! Tier-1 vehicle state keyed by VSS paths.

use serde::{Deserialize, Serialize};

/// XSR900 GP redline; drives the `at_redline` derived flag.
pub const REDLINE_RPM: f32 = 11_250.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VehicleState {
    pub speed: f32,
    pub rpm: f32,
    pub gear: i8,
    pub at_redline: bool,
    /// Redline indicator from the M7 CAN bit (combined with RPM in `refresh_derived`).
    pub redline_can: bool,
    pub throttle_pct: f32,
    pub side_stand: bool,
    pub riding_mode: String,
    pub fuel_pct: f32,
    pub coolant_c: i16,
    pub oil_c: i16,
    pub odometer: f32,
    pub trip1: f32,
    pub trip2: f32,
    pub lean_angle: f32,
    pub gforce: f32,
    pub battery_v: f32,
    pub can_load: u8,
    pub dtc: u8,
    pub abs_active: bool,
    pub tc_active: bool,
    pub heading: f32,
    pub elevation: i32,
    /// True when the signal source is actively updating (CAN frames or sim stepping).
    pub signals_live: bool,
}

impl VehicleState {
    /// Engine-off defaults (all gauges at rest).
    pub fn idle() -> Self {
        Self {
            speed: 0.0,
            rpm: 1_200.0,
            gear: 0,
            at_redline: false,
            redline_can: false,
            throttle_pct: 0.0,
            side_stand: true,
            riding_mode: "SPORT".into(),
            fuel_pct: 62.0,
            coolant_c: 42,
            oil_c: 52,
            odometer: 1_245.0,
            trip1: 137.4,
            trip2: 42.1,
            lean_angle: 0.0,
            gforce: 0.0,
            battery_v: 13.1,
            can_load: 8,
            dtc: 0,
            abs_active: false,
            tc_active: false,
            heading: 0.0,
            elevation: 667,
            signals_live: false,
        }
    }

    /// Recompute derived fields (`at_redline`, …) after raw updates.
    pub fn refresh_derived(&mut self) {
        self.at_redline = self.rpm >= REDLINE_RPM || self.redline_can;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redline_combines_rpm_and_can_bit() {
        let mut state = VehicleState::idle();
        state.rpm = 5_000.0;
        state.redline_can = true;
        state.refresh_derived();
        assert!(state.at_redline);

        state.redline_can = false;
        state.rpm = REDLINE_RPM;
        state.refresh_derived();
        assert!(state.at_redline);
    }
}
