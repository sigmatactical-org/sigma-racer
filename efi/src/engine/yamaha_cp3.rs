//! Yamaha CP3 — 890 cc inline triple (Crossplane Concept 3-cylinder).

use crate::engine::firing;
use crate::engine::{CYCLE_DEGREES_FOUR_STROKE, EngineProfile};
use crate::engine::{EngineConfig, IgnitionMode, InjectionMode};
use crate::trigger::{TriggerInputKind, TriggerSetup, TriggerWheel};

/// Profile identifier reported over comms.
pub const ID: &str = "Yamaha CP3";

/// The Yamaha CP3 (XSR900) engine profile.
pub fn profile() -> EngineProfile {
    EngineProfile {
        id: ID,
        engine: EngineConfig {
            cylinders: 3,
            firing_sequence: firing::INLINE_3_123,
            injection_mode: InjectionMode::Sequential,
            ignition_mode: IgnitionMode::IndividualCoils,
            cranking_injection_mode: InjectionMode::Simultaneous,
            displacement_cc: 890,
            target_idle_rpm: 1_200,
        },
        // ⚠ [MEASURE] — PLACEHOLDER trigger geometry, not characterization.
        // Wheel pattern and sensor types (VR vs Hall!) must be read off the
        // actual engine in mule Phase 1; the project rule is characterize,
        // don't invent. The stage-1 data logger never consumes these; the
        // decoder must refuse to run until they are replaced with measured
        // values. Firmware warns about this at boot.
        trigger: TriggerSetup {
            crank_wheel: TriggerWheel {
                teeth: 12,
                missing: 1,
            },
            crank_input: TriggerInputKind::Hall,
            cam_input: TriggerInputKind::Hall,
            cam_required: true,
        },
        cycle_degrees: CYCLE_DEGREES_FOUR_STROKE,
        // Even-firing triple: 240° spacing is engine architecture, not a guess.
        fire_intervals_deg: &[240, 240, 240],
        // ⚠ [MEASURE] — placeholder limits near the factory ~10.5k redline;
        // confirm against Yamaha service data before any control stage.
        soft_rev_limit_rpm: 10_400,
        hard_rev_limit_rpm: 11_000,
        spark_plugs_per_cylinder: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::ProfileError;

    #[test]
    fn profile_is_valid() {
        assert!(profile().validate().is_ok());
    }

    #[test]
    fn rejects_invalid_cycle_degrees() {
        let mut profile = profile();
        profile.cycle_degrees = 0;
        assert_eq!(profile.validate(), Err(ProfileError::InvalidCycleDegrees));
    }

    #[test]
    fn rejects_fire_interval_sum_mismatch() {
        let mut profile = profile();
        profile.fire_intervals_deg = &[240, 240, 240, 240];
        assert_eq!(profile.validate(), Err(ProfileError::FireIntervalCount));
    }
}
