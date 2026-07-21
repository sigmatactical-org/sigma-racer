//! [`DecoderOutput`].

#[allow(unused_imports)]
use super::*;

/// Snapshot after an edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecoderOutput {
    pub state: SyncState,
    pub rpm: f32,
    /// Crank angle within the cycle, degrees from the tooth after the gap.
    /// 0..360 at `SyncCrank`; 0..720 at `SyncFull` (four-stroke).
    pub cycle_angle_deg: f32,
    pub event: Option<DecodeEvent>,
}
