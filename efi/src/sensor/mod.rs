//! Sensor domain: channel identifiers, analog scaling, the MRE front-end
//! constants, and the stage-1 sweep frame.
//!
//! - [`channel`] — [`AdcChannel`] logical connector channels
//! - [`scaling`] — [`AnalogScaling`], [`NtcConfig`] thermistor model
//! - [`mre`] — microRusEFI ADC constants + channel mapping
//! - [`sweep`] — [`RawSweep`] / [`SensorFrame`] for the data logger

pub mod channel;
pub mod mre;
pub mod scaling;
pub mod sweep;

pub use channel::AdcChannel;
pub use mre::{
    ADC_VREF, ANALOG_INPUT_DIVIDER, CLT_NTC, IAT_NTC, MreAdcInput, VBATT_DIVIDER, VBATT_SCALING,
    mre_adc_input,
};
pub use scaling::{AnalogScaling, NtcConfig};
pub use sweep::{ADC_FULL_SCALE, RawSweep, SensorFrame, counts_to_pin_volts};
