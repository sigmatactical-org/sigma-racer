//! [`DesyncCause`].

#[allow(unused_imports)]
use super::*;

/// Why sync degraded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesyncCause {
    /// A gap-sized period arrived at the wrong wheel position.
    GapAtWrongPosition,
    /// Ran past the last physical tooth without seeing the gap.
    MissedGap,
    /// Period implausibly long — engine stopped or signal lost.
    Stall,
}
