//! Analog scaling helpers: linear front-end scaling and the NTC thermistor
//! model.

mod ntc_config;
pub use ntc_config::NtcConfig;

/// Analog front-end scaling applied after ADC conversion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnalogScaling {
    /// Multiply ADC pin volts by this factor to get connector volts.
    pub multiplier: f32,
    /// Fixed bias added after scaling (volts at connector).
    pub offset: f32,
}

impl AnalogScaling {
    /// Convert an ADC pin voltage to the sensed quantity's volts.
    pub const fn raw_to_volts(&self, adc_volts: f32) -> f32 {
        adc_volts * self.multiplier + self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ntc_at_bias_midpoint_is_near_room_temp() {
        // Node at half the bias supply ⇒ thermistor resistance equals the
        // bias resistor. Default config has R25 ≠ Rbias, so expect the
        // temperature where R(T) = 2.7 kΩ — just below 25 °C.
        let ntc = NtcConfig::default();
        let c = ntc.volts_to_celsius(2.5);
        assert!((c - 25.0).abs() < 5.0);
    }

    #[test]
    fn input_divider_reconstructs_node_volts() {
        let direct = NtcConfig::default();
        let divided = NtcConfig {
            input_divider: 1.68,
            ..NtcConfig::default()
        };
        // Same node voltage seen through the 1.68 divider at the pin.
        let c_direct = direct.volts_to_celsius(2.5);
        let c_divided = divided.volts_to_celsius(2.5 / 1.68);
        assert!((c_direct - c_divided).abs() < 0.1);
    }

    #[test]
    fn out_of_range_pin_volts_yield_nan() {
        let ntc = NtcConfig::default();
        assert!(ntc.volts_to_celsius(0.0).is_nan());
        assert!(ntc.volts_to_celsius(5.1).is_nan());
    }
}
