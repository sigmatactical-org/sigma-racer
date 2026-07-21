//! Runtime engine state and fuel/ignition scheduling hooks.

use crate::engine::EngineProfile;
use crate::engine::{EngineConfig, IgnitionMode, InjectionMode};
use crate::fuel::{PLACEHOLDER_INJECTOR, PLACEHOLDER_VE, SpeedDensityInputs, base_pulse_ms};
use crate::sensor::{CLT_NTC, IAT_NTC, VBATT_SCALING};

/// Closed-loop target for the base pulse; lambda trim owns the rest.
const STOICH_AFR: f32 = 14.7;

/// RPM below which the engine is treated as cranking (no sync or low speed).
const CRANKING_RPM_THRESHOLD: f32 = 100.0;

/// Base cranking pulse width when all injectors fire together (milliseconds).
const CRANKING_INJECTION_MS: f32 = 3.0;

/// Live engine runtime state updated by background tasks.
#[derive(Clone, Debug, PartialEq)]
pub struct EngineState {
    pub trigger: crate::trigger::TriggerState,
    /// Manifold absolute pressure in kPa (speed-density load input).
    pub map_kpa: f32,
    /// Coolant temperature in °C.
    pub clt_c: f32,
    /// Intake air temperature in °C.
    pub iat_c: f32,
    /// Battery voltage at connector (volts).
    pub vbatt: f32,
    /// Last computed base injection duration in milliseconds.
    pub base_injection_ms: f32,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            trigger: crate::trigger::TriggerState::default(),
            map_kpa: 100.0,
            clt_c: 20.0,
            iat_c: 20.0,
            vbatt: 12.0,
            base_injection_ms: 0.0,
        }
    }
}

impl EngineState {
    /// Below the cranking-to-running rpm threshold.
    pub fn is_cranking(&self) -> bool {
        !self.trigger.synced || self.trigger.rpm < CRANKING_RPM_THRESHOLD
    }

    /// Update battery voltage from a raw ADC reading.
    pub fn update_vbatt_from_adc(&mut self, adc_volts: f32) {
        self.vbatt = VBATT_SCALING.raw_to_volts(adc_volts);
    }

    /// Update coolant temperature from a raw ADC reading.
    pub fn update_clt_from_adc(&mut self, adc_volts: f32) {
        self.clt_c = CLT_NTC.volts_to_celsius(adc_volts);
    }

    /// Update intake-air temperature from a raw ADC reading.
    pub fn update_iat_from_adc(&mut self, adc_volts: f32) {
        self.iat_c = IAT_NTC.volts_to_celsius(adc_volts);
    }

    /// Fuel events scheduled per engine cycle for the configured injection mode.
    pub fn injection_events_per_cycle(config: &EngineConfig) -> u8 {
        match config.injection_mode {
            InjectionMode::Simultaneous => 1,
            InjectionMode::Sequential => config.cylinders,
            InjectionMode::Batch => config.cylinders.div_ceil(2),
        }
    }

    /// Spark events scheduled per engine cycle.
    pub fn ignition_events_per_cycle(profile: &EngineProfile) -> u8 {
        let coils = match profile.engine.ignition_mode {
            IgnitionMode::IndividualCoils => profile.engine.cylinders,
            // Wasted-spark pairs share coils; each coil fires once per 360° cam revolution.
            IgnitionMode::WastedSpark => profile.engine.cylinders.div_ceil(2),
        };
        coils.saturating_mul(profile.spark_plugs_per_cylinder)
    }

    /// Base fuel pulse width via the speed-density pipeline (`fueling`).
    ///
    /// One fuel math in the crate: VE table × ideal gas × injector model —
    /// calibrations are ⚠ [MEASURE] placeholders until the dyno (M6).
    pub fn compute_base_injection_ms(&mut self, config: &EngineConfig) -> f32 {
        if self.is_cranking() {
            self.base_injection_ms = match config.cranking_injection_mode {
                InjectionMode::Simultaneous => CRANKING_INJECTION_MS,
                InjectionMode::Sequential | InjectionMode::Batch => {
                    CRANKING_INJECTION_MS / config.cylinders.max(1) as f32
                }
            };
            return self.base_injection_ms;
        }

        let base = base_pulse_ms(
            &PLACEHOLDER_VE,
            &PLACEHOLDER_INJECTOR,
            &SpeedDensityInputs {
                rpm: self.trigger.rpm,
                map_kpa: self.map_kpa,
                iat_c: self.iat_c,
                displacement_per_cyl_cc: config.displacement_cc as f32
                    / config.cylinders.max(1) as f32,
                afr_target: STOICH_AFR,
                vbatt: self.vbatt,
            },
        );

        self.base_injection_ms = match config.injection_mode {
            InjectionMode::Simultaneous | InjectionMode::Sequential => base,
            // Batch fires fewer, longer events per cycle.
            InjectionMode::Batch => base * 2.0,
        };
        self.base_injection_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::yamaha_cp3;
    use crate::engine::{IgnitionMode, InjectionMode};

    #[test]
    fn cranking_injection_uses_cranking_mode() {
        let mut state = EngineState::default();
        let ms = state.compute_base_injection_ms(&yamaha_cp3::profile().engine);
        assert_eq!(ms, 3.0);
    }

    #[test]
    fn sequential_cranking_splits_pulse_per_cylinder() {
        let mut state = EngineState::default();
        let mut config = yamaha_cp3::profile().engine;
        config.cranking_injection_mode = InjectionMode::Sequential;

        let ms = state.compute_base_injection_ms(&config);
        assert!((ms - 1.0).abs() < 0.001);
    }

    #[test]
    fn vbatt_scales_from_adc() {
        let mut state = EngineState::default();
        state.update_vbatt_from_adc(1.457);
        assert!((state.vbatt - 12.0).abs() < 0.5);
    }

    #[test]
    fn injection_events_respect_mode() {
        let config = yamaha_cp3::profile().engine;
        assert_eq!(EngineState::injection_events_per_cycle(&config), 3);

        let mut batch = config.clone();
        batch.injection_mode = InjectionMode::Batch;
        assert_eq!(EngineState::injection_events_per_cycle(&batch), 2);

        let mut sim = config;
        sim.injection_mode = InjectionMode::Simultaneous;
        assert_eq!(EngineState::injection_events_per_cycle(&sim), 1);
    }

    #[test]
    fn ignition_events_respect_mode_and_plug_count() {
        let profile = yamaha_cp3::profile();
        assert_eq!(EngineState::ignition_events_per_cycle(&profile), 3);

        let mut wasted = profile.clone();
        wasted.engine.ignition_mode = IgnitionMode::WastedSpark;
        assert_eq!(EngineState::ignition_events_per_cycle(&wasted), 2);

        let mut twin_plug = profile;
        twin_plug.spark_plugs_per_cylinder = 2;
        assert_eq!(EngineState::ignition_events_per_cycle(&twin_plug), 6);
    }

    #[test]
    fn running_injection_scales_with_batch_mode() {
        let mut state = EngineState::default();
        state.trigger.synced = true;
        state.trigger.rpm = 3_000.0;

        let mut config = yamaha_cp3::profile().engine;
        config.injection_mode = InjectionMode::Sequential;
        let sequential = state.compute_base_injection_ms(&config);

        config.injection_mode = InjectionMode::Batch;
        let batch = state.compute_base_injection_ms(&config);

        assert!((batch - sequential * 2.0).abs() < 0.001);
    }
}
