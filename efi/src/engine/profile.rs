//! Engine profile — tunable metadata independent of ECU board.

mod profile_error;
pub use profile_error::ProfileError;

use crate::engine::EngineConfig;
use crate::trigger::TriggerSetup;

/// Full engine cycle in crank degrees for a four-stroke Otto cycle.
pub const CYCLE_DEGREES_FOUR_STROKE: u16 = 720;

/// Complete configuration for one engine type (factory specs + trigger pattern).
#[derive(Clone, Debug, PartialEq)]
pub struct EngineProfile {
    /// Human-readable name, e.g. `"Yamaha CP3"`.
    pub id: &'static str,
    pub engine: EngineConfig,
    pub trigger: TriggerSetup,
    /// Crank degrees for one full engine cycle (720° four-stroke, 360° two-stroke).
    pub cycle_degrees: u16,
    /// Crank degrees between consecutive power strokes; length must match cylinder count.
    pub fire_intervals_deg: &'static [u16],
    pub soft_rev_limit_rpm: u16,
    pub hard_rev_limit_rpm: u16,
    pub spark_plugs_per_cylinder: u8,
}

impl EngineProfile {
    /// Sanity-check the profile at boot.
    pub fn validate(&self) -> Result<(), ProfileError> {
        self.engine.validate()?;

        if self.cycle_degrees == 0 {
            return Err(ProfileError::InvalidCycleDegrees);
        }

        if self.fire_intervals_deg.len() != self.engine.cylinders as usize {
            return Err(ProfileError::FireIntervalCount);
        }

        let sum: u16 = self.fire_intervals_deg.iter().sum();
        if sum != self.cycle_degrees {
            return Err(ProfileError::FireIntervalSum);
        }

        if self.spark_plugs_per_cylinder == 0 {
            return Err(ProfileError::InvalidSparkPlugCount);
        }

        // Rev limits are safety-critical: both must be non-zero and the soft
        // (fuel/spark-cut warning) limit must not exceed the hard (absolute) limit.
        if self.soft_rev_limit_rpm == 0 || self.hard_rev_limit_rpm == 0 {
            return Err(ProfileError::InvalidRevLimit);
        }
        if self.soft_rev_limit_rpm > self.hard_rev_limit_rpm {
            return Err(ProfileError::RevLimitOrder);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::yamaha_cp3;

    #[test]
    fn reference_profile_is_valid() {
        assert_eq!(yamaha_cp3::profile().validate(), Ok(()));
    }

    #[test]
    fn rejects_zero_rev_limit() {
        let mut p = yamaha_cp3::profile();
        p.soft_rev_limit_rpm = 0;
        assert_eq!(p.validate(), Err(ProfileError::InvalidRevLimit));
    }

    #[test]
    fn rejects_soft_above_hard_rev_limit() {
        let mut p = yamaha_cp3::profile();
        p.soft_rev_limit_rpm = p.hard_rev_limit_rpm + 100;
        assert_eq!(p.validate(), Err(ProfileError::RevLimitOrder));
    }
}
