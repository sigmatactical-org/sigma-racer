//! [`SensorFrame`].

#[allow(unused_imports)]
use super::*;
use crate::sensor::{ANALOG_INPUT_DIVIDER, CLT_NTC, IAT_NTC, VBATT_SCALING};

/// One scaled sensor frame, ready to log or publish.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SensorFrame {
    /// Microseconds since boot.
    pub t_us: u64,
    /// Battery voltage at the connector.
    pub vbatt_v: f32,
    /// Coolant temperature (NTC beta model), °C.
    pub clt_c: f32,
    /// Intake air temperature, °C.
    pub iat_c: f32,
    /// TPS/MAP shared input (MRE default wiring), volts at connector.
    pub tps_map_v: f32,
    /// AN volt 1 (connector pin 27), volts at connector.
    pub an_volt1_v: f32,
    /// AN volt 2 (connector pin 26), volts at connector.
    pub an_volt2_v: f32,
}
impl SensorFrame {
    /// Scale one raw sweep using the MRE analog front-end constants.
    pub fn from_sweep(t_us: u64, sweep: RawSweep) -> Self {
        Self {
            t_us,
            vbatt_v: VBATT_SCALING.raw_to_volts(sweep.vbatt),
            clt_c: CLT_NTC.volts_to_celsius(sweep.clt),
            iat_c: IAT_NTC.volts_to_celsius(sweep.iat),
            tps_map_v: sweep.tps_map * ANALOG_INPUT_DIVIDER,
            an_volt1_v: sweep.an_volt1 * ANALOG_INPUT_DIVIDER,
            an_volt2_v: sweep.an_volt2 * ANALOG_INPUT_DIVIDER,
        }
    }
}
