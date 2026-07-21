//! Engine configuration persisted in flash / tuned at runtime.

mod config_error;
mod ignition_mode;
mod injection_mode;
pub use config_error::ConfigError;
pub use ignition_mode::IgnitionMode;
pub use injection_mode::InjectionMode;

/// Number of cylinders supported by the core engine model (microRusEFI limit).
pub const MAX_CYLINDERS: usize = 4;

/// Top-level tunable engine configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineConfig {
    pub cylinders: u8,
    /// Cylinder indices (0-based) in firing order; length must equal `cylinders`.
    pub firing_sequence: &'static [u8],
    pub injection_mode: InjectionMode,
    pub ignition_mode: IgnitionMode,
    pub cranking_injection_mode: InjectionMode,
    /// Displacement in cubic centimeters (used for load modeling).
    pub displacement_cc: u16,
    /// Target idle RPM when closed-loop idle is active.
    pub target_idle_rpm: u16,
}

impl EngineConfig {
    /// Number of cylinders configured.
    pub const fn cylinder_count(&self) -> usize {
        self.cylinders as usize
    }

    /// Sanity-check the configuration at boot.
    pub const fn validate(&self) -> Result<(), ConfigError> {
        if self.cylinders == 0 || self.cylinders as usize > MAX_CYLINDERS {
            return Err(ConfigError::InvalidCylinderCount);
        }
        if self.firing_sequence.len() != self.cylinders as usize {
            return Err(ConfigError::FiringSequenceLength);
        }

        let mut seen = 0u8;
        let mut i = 0;
        while i < self.firing_sequence.len() {
            let idx = self.firing_sequence[i];
            if idx >= self.cylinders {
                return Err(ConfigError::InvalidFiringIndex);
            }
            let bit = 1u8 << idx;
            if seen & bit != 0 {
                return Err(ConfigError::DuplicateFiringIndex);
            }
            seen |= bit;
            i += 1;
        }

        let expected = (1u8 << self.cylinders) - 1;
        if seen != expected {
            return Err(ConfigError::IncompleteFiringSequence);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn firing_sequence_length_matches_cylinder_count() {
        let config = EngineConfig {
            cylinders: 4,
            firing_sequence: &[0, 2, 3, 1],
            injection_mode: InjectionMode::Sequential,
            ignition_mode: IgnitionMode::IndividualCoils,
            cranking_injection_mode: InjectionMode::Simultaneous,
            displacement_cc: 1_600,
            target_idle_rpm: 850,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn rejects_mismatched_firing_sequence() {
        let config = EngineConfig {
            cylinders: 3,
            firing_sequence: &[0, 2, 3, 1],
            injection_mode: InjectionMode::Sequential,
            ignition_mode: IgnitionMode::IndividualCoils,
            cranking_injection_mode: InjectionMode::Simultaneous,
            displacement_cc: 890,
            target_idle_rpm: 1_200,
        };
        assert_eq!(config.validate(), Err(ConfigError::FiringSequenceLength));
    }

    #[test]
    fn rejects_out_of_range_firing_index() {
        let config = EngineConfig {
            cylinders: 3,
            firing_sequence: &[0, 1, 4],
            injection_mode: InjectionMode::Sequential,
            ignition_mode: IgnitionMode::IndividualCoils,
            cranking_injection_mode: InjectionMode::Simultaneous,
            displacement_cc: 890,
            target_idle_rpm: 1_200,
        };
        assert_eq!(config.validate(), Err(ConfigError::InvalidFiringIndex));
    }

    #[test]
    fn rejects_duplicate_firing_index() {
        let config = EngineConfig {
            cylinders: 3,
            firing_sequence: &[0, 1, 0],
            injection_mode: InjectionMode::Sequential,
            ignition_mode: IgnitionMode::IndividualCoils,
            cranking_injection_mode: InjectionMode::Simultaneous,
            displacement_cc: 890,
            target_idle_rpm: 1_200,
        };
        assert_eq!(config.validate(), Err(ConfigError::DuplicateFiringIndex));
    }

    #[test]
    fn rejects_incomplete_firing_sequence() {
        let config = EngineConfig {
            cylinders: 3,
            firing_sequence: &[0, 1],
            injection_mode: InjectionMode::Sequential,
            ignition_mode: IgnitionMode::IndividualCoils,
            cranking_injection_mode: InjectionMode::Simultaneous,
            displacement_cc: 890,
            target_idle_rpm: 1_200,
        };
        assert_eq!(config.validate(), Err(ConfigError::FiringSequenceLength));
    }
}
