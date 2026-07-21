//! [`TripCause`].

#[allow(unused_imports)]
use super::*;

/// Why the monitor tripped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripCause {
    AppOutOfRange(Side),
    TpsOutOfRange(Side),
    AppDisagreement,
    TpsDisagreement,
    /// Plate not following the command: stuck, runaway, or motor fault.
    Tracking,
}
