//! Coil dwell model — dwell vs battery voltage with an over-dwell ceiling
//! (`efi.md` §6).
//!
//! ⚠ [MEASURE] against the CP3 coils (smart-vs-dumb determination comes
//! first — runbook Phase 1, item 9).

use crate::fuel::Curve;

/// Coil dwell vs battery voltage with an over-dwell ceiling.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DwellModel {
    pub dwell_ms: Curve<4>,
    /// Hard ceiling, ms — over-dwell protection regardless of the curve.
    pub max_dwell_ms: f32,
}

/// Bring-up dwell numbers — replace with measured coil data.
pub const PLACEHOLDER_DWELL: DwellModel = DwellModel {
    dwell_ms: Curve {
        x: [8.0, 11.0, 13.5, 16.0],
        y: [6.0, 4.2, 3.2, 2.6],
    },
    max_dwell_ms: 8.0,
};

impl DwellModel {
    /// Dwell time for the current battery voltage (linear vbatt comp).
    pub fn dwell_ms(&self, vbatt: f32) -> f32 {
        let d = self.dwell_ms.lookup(vbatt);
        if d > self.max_dwell_ms {
            self.max_dwell_ms
        } else {
            d
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dwell_clamps_at_overdwell_ceiling() {
        // At 8 V the curve asks 6 ms — under the 8 ms ceiling.
        assert!(PLACEHOLDER_DWELL.dwell_ms(8.0) <= PLACEHOLDER_DWELL.max_dwell_ms);
        // A pathological curve request is clamped.
        let hot = DwellModel {
            max_dwell_ms: 3.0,
            ..PLACEHOLDER_DWELL
        };
        assert_eq!(hot.dwell_ms(8.0), 3.0);
    }
}
