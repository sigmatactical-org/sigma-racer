//! [`IgnitionMode`].

#[allow(unused_imports)]
use super::*;

/// Coil wiring strategy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum IgnitionMode {
    /// One coil per cylinder, fired on compression stroke only.
    #[default]
    IndividualCoils,
    /// Pairs of cylinders share a coil (360° wasted spark).
    WastedSpark,
}
