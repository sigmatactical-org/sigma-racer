//! microRusEFI analog front-end constants and ADC channel mapping.
//!
//! From rusEFI `setupVbatt()` and the MRE sensor defaults.

use crate::sensor::{AdcChannel, AnalogScaling, NtcConfig};

/// ADC reference voltage on microRusEFI.
pub const ADC_VREF: f32 = 3.3;

/// Global divider: 6.8 kΩ high / 10 kΩ low → 1.68 ratio at the input network.
pub const ANALOG_INPUT_DIVIDER: f32 = 16.8 / 10.0;

/// Battery sense: 39 kΩ / 10 kΩ divider, scaled by `ANALOG_INPUT_DIVIDER`.
pub const VBATT_DIVIDER: f32 = (49.0 / 10.0) * ANALOG_INPUT_DIVIDER;

/// Battery-voltage channel scaling.
pub const VBATT_SCALING: AnalogScaling = AnalogScaling {
    multiplier: VBATT_DIVIDER,
    offset: 0.0,
};

/// rusEFI default CLT/IAT pull-up on microRusEFI (2.7 kΩ).
///
/// ⚠ [MEASURE] — `bias_supply_volts` (wiki says the AT pull-up goes to 5 V)
/// and `input_divider` (whether the AT path passes the 1.68 divider) are
/// placeholders until the Session-0 warmup fit / resistor check settles them
/// (see the stage-1 wiring doc in the specs repo).
pub const CLT_NTC: NtcConfig = NtcConfig {
    bias_resistor_ohms: 2_700.0,
    beta: 3_500.0,
    resistance_at_25c: 2_500.0,
    bias_supply_volts: 5.0,
    input_divider: 1.0,
};

/// Intake-air temperature NTC (datasheet curve).
pub const IAT_NTC: NtcConfig = NtcConfig {
    bias_resistor_ohms: 2_700.0,
    beta: 3_500.0,
    resistance_at_25c: 2_500.0,
    bias_supply_volts: 5.0,
    input_divider: 1.0,
};

/// Maps logical channels to STM32 ADC inputs (EFI_ADC_x from rusEFI naming).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MreAdcInput {
    Adc0,
    Adc1,
    Adc2,
    Adc3,
    Adc4,
    Adc6,
    Adc7,
    Adc8,
    Adc9,
    Adc10,
    Adc11,
    Adc12,
    Adc13,
    Adc14,
    Adc15,
}

/// ADC input descriptor for an MRE analog channel.
pub fn mre_adc_input(channel: AdcChannel) -> MreAdcInput {
    match channel {
        AdcChannel::Battery => MreAdcInput::Adc11,
        AdcChannel::AnalogVolt1 => MreAdcInput::Adc10,
        AdcChannel::AnalogVolt2 => MreAdcInput::Adc6,
        AdcChannel::AnalogVolt3 => MreAdcInput::Adc7,
        AdcChannel::AnalogVolt4 => MreAdcInput::Adc12,
        AdcChannel::AnalogVolt5 => MreAdcInput::Adc13,
        AdcChannel::AnalogVolt6 => MreAdcInput::Adc14,
        AdcChannel::AnalogVolt7 => MreAdcInput::Adc15,
        AdcChannel::CoolantTemp => MreAdcInput::Adc0,
        AdcChannel::IntakeTemp => MreAdcInput::Adc1,
        AdcChannel::AuxTemp3 => MreAdcInput::Adc2,
        AdcChannel::AuxTemp4 => MreAdcInput::Adc3,
        AdcChannel::Tps | AdcChannel::Map => MreAdcInput::Adc4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vbatt_scaling_converts_adc_to_connector_volts() {
        let scaled = VBATT_SCALING.raw_to_volts(1.457);
        assert!((scaled - 12.0).abs() < 0.1);
    }
}
