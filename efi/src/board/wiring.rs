//! Logical cylinder → microRusEFI output mapping and profile validation.

use crate::board::{BoardPins, GpioPin, TleOutput};
use crate::engine::EngineProfile;
use crate::engine::MAX_CYLINDERS;

const INJECTORS: [TleOutput; MAX_CYLINDERS] = [
    TleOutput::Injector1,
    TleOutput::Injector2,
    TleOutput::Injector3,
    TleOutput::Injector4,
];

/// Fuel pump relay output.
pub const FUEL_PUMP: TleOutput = TleOutput::GpOut1;
/// Radiator fan relay output.
pub const RADIATOR_FAN: TleOutput = TleOutput::GpOut2;
/// Spare general-purpose output 3.
pub const AUX_GP_OUT_3: TleOutput = TleOutput::GpOut3;
/// Spare general-purpose output 4.
pub const AUX_GP_OUT_4: TleOutput = TleOutput::GpOut4;
/// VVT solenoid low-side output.
pub const VVT_SOLENOID: TleOutput = TleOutput::LowSide1;
/// Idle air-control low-side output.
pub const IDLE_IAC: TleOutput = TleOutput::LowSide2;

/// Crank trigger (VR/Hall) input pin.
pub fn trigger_crank(pins: &BoardPins) -> GpioPin {
    pins.trigger_crank
}

/// Cam trigger input pin.
pub fn trigger_cam(pins: &BoardPins) -> GpioPin {
    pins.trigger_cam
}

/// Injector output for a cylinder (1-based), if wired.
pub fn injector_for(cylinder: u8) -> Option<TleOutput> {
    INJECTORS.get(cylinder as usize).copied()
}

/// Ignition coil pin for a cylinder (1-based), if wired.
pub fn ignition_for(pins: &BoardPins, cylinder: u8) -> Option<GpioPin> {
    pins.ignition.get(cylinder as usize).copied()
}

/// Returns `Ok(())` when the profile fits microRusEFI (≤4 cylinders, outputs exist).
pub fn validate_profile(profile: &EngineProfile) -> Result<(), WiringError> {
    profile.validate()?;

    if profile.engine.cylinders as usize > MAX_CYLINDERS {
        return Err(WiringError::TooManyCylinders);
    }

    // Build the board pin map once, not once per cylinder.
    let pins = BoardPins::mre_f7();
    for &cyl in profile.engine.firing_sequence {
        if injector_for(cyl).is_none() || ignition_for(&pins, cyl).is_none() {
            return Err(WiringError::MissingOutput);
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WiringError {
    Profile(crate::engine::ProfileError),
    TooManyCylinders,
    MissingOutput,
}

impl From<crate::engine::ProfileError> for WiringError {
    fn from(err: crate::engine::ProfileError) -> Self {
        Self::Profile(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::yamaha_cp3;

    #[test]
    fn yamaha_cp3_fits_mre_outputs() {
        assert!(validate_profile(&yamaha_cp3::profile()).is_ok());
    }

    #[test]
    fn cylinder_index_maps_sequential_outputs() {
        use crate::board::{GpioPin, GpioPort, TleOutput};

        assert_eq!(injector_for(0), Some(TleOutput::Injector1));
        assert_eq!(injector_for(2), Some(TleOutput::Injector3));
        let pins = crate::board::BoardPins::mre_f7();
        assert_eq!(ignition_for(&pins, 1), Some(GpioPin::new(GpioPort::D, 3)));
    }

    #[test]
    fn rejects_profile_with_invalid_firing_index() {
        use crate::engine::{CYCLE_DEGREES_FOUR_STROKE, EngineProfile};
        use crate::engine::{EngineConfig, IgnitionMode, InjectionMode};
        use crate::trigger::{TriggerInputKind, TriggerSetup, TriggerWheel};

        let bad = EngineProfile {
            id: "bad",
            engine: EngineConfig {
                cylinders: 3,
                firing_sequence: &[0, 1, 4],
                injection_mode: InjectionMode::Sequential,
                ignition_mode: IgnitionMode::IndividualCoils,
                cranking_injection_mode: InjectionMode::Simultaneous,
                displacement_cc: 890,
                target_idle_rpm: 1_200,
            },
            trigger: TriggerSetup {
                crank_wheel: TriggerWheel::twelve_minus_one(),
                crank_input: TriggerInputKind::Hall,
                cam_input: TriggerInputKind::Hall,
                cam_required: true,
            },
            cycle_degrees: CYCLE_DEGREES_FOUR_STROKE,
            fire_intervals_deg: &[240, 240, 240],
            soft_rev_limit_rpm: 8_000,
            hard_rev_limit_rpm: 8_500,
            spark_plugs_per_cylinder: 1,
        };

        assert!(validate_profile(&bad).is_err());
    }
}
