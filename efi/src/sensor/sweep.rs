//! Stage-1 sensor sweep — one scaled frame from the analog channels.
//!
//! The passive data logger (mule runbook Phase 1) samples the analog
//! channels and scales them with the MRE front-end constants. Host-testable
//! `no_std` logic; the Embassy task in `src/bin/tasks/` is a thin wrapper.

mod sensor_frame;
pub use sensor_frame::SensorFrame;

use crate::sensor::ADC_VREF;

/// Full-scale ADC counts at 12-bit resolution.
pub const ADC_FULL_SCALE: f32 = 4095.0;

/// Convert raw 12-bit ADC counts to volts at the MCU pin.
pub fn counts_to_pin_volts(raw: u16) -> f32 {
    raw as f32 * ADC_VREF / ADC_FULL_SCALE
}

/// Pin volts from one sampling sweep, before any connector scaling.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RawSweep {
    pub vbatt: f32,
    pub clt: f32,
    pub iat: f32,
    pub tps_map: f32,
    pub an_volt1: f32,
    pub an_volt2: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::ANALOG_INPUT_DIVIDER;

    #[test]
    fn counts_scale_to_pin_volts() {
        assert!((counts_to_pin_volts(0) - 0.0).abs() < 1e-6);
        assert!((counts_to_pin_volts(4095) - ADC_VREF).abs() < 1e-6);
        assert!((counts_to_pin_volts(2048) - ADC_VREF / 2.0).abs() < 0.002);
    }

    #[test]
    fn sensor_frame_scales_vbatt_to_connector_volts() {
        let frame = SensorFrame::from_sweep(
            42,
            RawSweep {
                vbatt: 1.457,
                ..Default::default()
            },
        );
        assert_eq!(frame.t_us, 42);
        assert!((frame.vbatt_v - 12.0).abs() < 0.1);
    }

    #[test]
    fn sensor_frame_applies_input_divider_to_volt_channels() {
        let frame = SensorFrame::from_sweep(
            0,
            RawSweep {
                tps_map: 1.0,
                an_volt1: 2.0,
                ..Default::default()
            },
        );
        assert!((frame.tps_map_v - ANALOG_INPUT_DIVIDER).abs() < 1e-6);
        assert!((frame.an_volt1_v - 2.0 * ANALOG_INPUT_DIVIDER).abs() < 1e-6);
    }
}
