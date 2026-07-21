//! [`SensorRec`].

#[allow(unused_imports)]
use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SensorRec {
    pub(crate) t_us: u64,
    /// vbatt_v, clt_c, iat_c, tps_map_v, an_volt1_v, an_volt2_v
    pub(crate) values: [f64; 6],
}
