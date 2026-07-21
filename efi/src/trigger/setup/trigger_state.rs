//! [`TriggerState`].

#[allow(unused_imports)]
use super::*;

/// Engine phase derived from primary trigger edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriggerState {
    pub rpm: f32,
    pub tooth_count: u32,
    /// 0.0–1.0 position within the current engine cycle (720° four-stroke).
    pub cycle_phase: f32,
    pub synced: bool,
}
impl Default for TriggerState {
    fn default() -> Self {
        Self {
            rpm: 0.0,
            tooth_count: 0,
            cycle_phase: 0.0,
            synced: false,
        }
    }
}
