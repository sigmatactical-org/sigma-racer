//! [`SyncState`].

#[allow(unused_imports)]
use super::*;

/// Sync confidence, in increasing order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncState {
    /// No usable edge stream.
    Lost,
    /// Edges arriving; hunting for a confirmed gap.
    Syncing,
    /// Crank position known within 360°.
    SyncCrank,
    /// Cycle position known within 720° (cam resolved, or cam not required).
    SyncFull,
}
