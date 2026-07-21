//! [`SensorChannels`].

#[allow(unused_imports)]
use super::*;
use embassy_stm32::adc::AnyAdcChannel;
use embassy_stm32::peripherals::ADC1;

/// ADC channels for the stage-1 sensor sweep (MRE default wiring).
pub struct SensorChannels {
    pub vbatt: AnyAdcChannel<'static, ADC1>,
    pub clt: AnyAdcChannel<'static, ADC1>,
    pub iat: AnyAdcChannel<'static, ADC1>,
    pub tps_map: AnyAdcChannel<'static, ADC1>,
    pub an_volt1: AnyAdcChannel<'static, ADC1>,
    pub an_volt2: AnyAdcChannel<'static, ADC1>,
}
