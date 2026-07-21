//! [`ProfileError`].

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileError {
    EngineConfig(crate::engine::ConfigError),
    InvalidCycleDegrees,
    FireIntervalCount,
    FireIntervalSum,
    InvalidSparkPlugCount,
    /// A rev limit was zero.
    InvalidRevLimit,
    /// The soft rev limit exceeds the hard rev limit.
    RevLimitOrder,
}
impl From<crate::engine::ConfigError> for ProfileError {
    fn from(err: crate::engine::ConfigError) -> Self {
        Self::EngineConfig(err)
    }
}
