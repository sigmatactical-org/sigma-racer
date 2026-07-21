//! [`ConfigError`].

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigError {
    InvalidCylinderCount,
    FiringSequenceLength,
    InvalidFiringIndex,
    DuplicateFiringIndex,
    IncompleteFiringSequence,
}
