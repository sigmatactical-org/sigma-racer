//! [`DecodeEvent`].

#[allow(unused_imports)]
use super::*;

/// Notable outcome of feeding one edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeEvent {
    GapDetected,
    /// Crank sync achieved (second consistent gap).
    SyncAchieved,
    /// Full cycle sync achieved.
    FullSyncAchieved,
    /// Edge rejected as noise; sync retained.
    NoiseRejected,
    Desync(DesyncCause),
}
