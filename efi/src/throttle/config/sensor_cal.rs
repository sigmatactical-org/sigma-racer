//! [`SensorCal`].

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorCal {
    /// Lowest electrically-plausible voltage (below ⇒ open circuit / sag).
    pub min_valid_v: f32,
    /// Highest electrically-plausible voltage (above ⇒ short to supply).
    pub max_valid_v: f32,
    /// Voltage at 0 %.
    pub v_at_0: f32,
    /// Voltage at 100 %.
    pub v_at_100: f32,
}
impl SensorCal {
    /// Whether a reading is inside the plausible electrical range.
    pub fn in_range(&self, volts: f32) -> bool {
        volts >= self.min_valid_v && volts <= self.max_valid_v
    }

    /// Normalize volts to percent (unclamped slope; caller range-checks
    /// first). Handles inverted slopes (v_at_100 < v_at_0) transparently,
    /// which is how mirrored second sensors normalize without special cases.
    pub fn to_pct(&self, volts: f32) -> f32 {
        let span = self.v_at_100 - self.v_at_0;
        if span == 0.0 {
            return 0.0;
        }
        (volts - self.v_at_0) / span * 100.0
    }
}
