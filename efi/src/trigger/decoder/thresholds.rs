//! [`Thresholds`].

#[allow(unused_imports)]
use super::*;
use crate::trigger::TriggerWheel;

/// Ratio thresholds (×100) derived from the wheel geometry.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Thresholds {
    /// Below this ×prev the edge is noise.
    pub(crate) noise_x100: u32,
    /// At or above this ×prev the period is a gap candidate.
    pub(crate) gap_low_x100: u32,
    /// Above this ×prev the engine has stalled / signal lost.
    pub(crate) gap_high_x100: u32,
}
impl Thresholds {
    /// Interval thresholds tuned for the given wheel geometry.
    pub(crate) fn for_wheel(wheel: TriggerWheel) -> Self {
        // A gap spans (missing + 1) tooth pitches.
        let gap = (wheel.missing as u32 + 1) * 100;
        Self {
            noise_x100: 25,
            // Halfway between a normal tooth and the gap.
            gap_low_x100: (100 + gap) / 2,
            // Generous accel/decel margin above the gap.
            gap_high_x100: gap + gap / 2,
        }
    }
}
