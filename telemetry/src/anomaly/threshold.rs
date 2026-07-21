//! High-side threshold detector: signal at or above a limit, sustained.

use crate::state::VehicleState;

use super::types::{Edge, SignalFn, TsMillis};

/// Configuration for a sustained high-side threshold with a hysteresis band.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdConfig {
    pub signal: SignalFn,
    /// Raise once the signal has been at or above this for `sustain_ms`.
    pub raise_at: f64,
    /// Clear only once the signal drops below this (must be < `raise_at`).
    pub clear_below: f64,
    pub sustain_ms: TsMillis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Pending(TsMillis),
    Active,
}

/// Pure state machine; O(1) per sample, no allocation.
#[derive(Debug)]
pub struct ThresholdDetector {
    cfg: ThresholdConfig,
    phase: Phase,
}

impl ThresholdDetector {
    pub fn new(cfg: ThresholdConfig) -> Self {
        Self {
            cfg,
            phase: Phase::Idle,
        }
    }

    /// Feed one timestamped sample; returns a transition when one occurs.
    pub fn update(&mut self, ts: TsMillis, state: &VehicleState) -> Option<Edge> {
        let v = (self.cfg.signal)(state);
        match self.phase {
            Phase::Idle => {
                if v >= self.cfg.raise_at {
                    self.phase = Phase::Pending(ts);
                }
                None
            }
            Phase::Pending(since) => {
                if v < self.cfg.raise_at {
                    self.phase = Phase::Idle;
                    None
                } else if ts < since {
                    // Backwards timestamp (clock adjust / replay reset): restart the window.
                    self.phase = Phase::Pending(ts);
                    None
                } else if ts - since >= self.cfg.sustain_ms {
                    self.phase = Phase::Active;
                    Some(Edge::Raised)
                } else {
                    None
                }
            }
            Phase::Active => {
                if v < self.cfg.clear_below {
                    self.phase = Phase::Idle;
                    Some(Edge::Cleared)
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coolant(state: &VehicleState) -> f64 {
        f64::from(state.coolant_c)
    }

    fn cfg() -> ThresholdConfig {
        ThresholdConfig {
            signal: coolant,
            raise_at: 115.0,
            clear_below: 108.0,
            sustain_ms: 3_000,
        }
    }

    fn state_at(coolant_c: i16) -> VehicleState {
        VehicleState {
            coolant_c,
            ..VehicleState::idle()
        }
    }

    #[test]
    fn blip_below_sustain_does_not_raise() {
        let mut d = ThresholdDetector::new(cfg());
        assert_eq!(d.update(0, &state_at(116)), None);
        assert_eq!(d.update(1_000, &state_at(116)), None);
        // Dips back under before the 3 s sustain elapses.
        assert_eq!(d.update(2_000, &state_at(100)), None);
        assert_eq!(d.update(5_000, &state_at(100)), None);
    }

    #[test]
    fn sustained_raise_at_expected_ts_then_hysteresis_clear() {
        let mut d = ThresholdDetector::new(cfg());
        assert_eq!(d.update(0, &state_at(116)), None);
        assert_eq!(d.update(2_999, &state_at(118)), None);
        assert_eq!(d.update(3_000, &state_at(118)), Some(Edge::Raised));
        // Inside the hysteresis band: still active.
        assert_eq!(d.update(4_000, &state_at(110)), None);
        assert_eq!(d.update(5_000, &state_at(107)), Some(Edge::Cleared));
        // Idle again; needs a fresh sustain to re-raise.
        assert_eq!(d.update(6_000, &state_at(116)), None);
    }

    #[test]
    fn backwards_ts_restarts_the_window() {
        let mut d = ThresholdDetector::new(cfg());
        assert_eq!(d.update(10_000, &state_at(116)), None);
        // Clock jumps back: pending window restarts at the new ts.
        assert_eq!(d.update(1_000, &state_at(116)), None);
        assert_eq!(d.update(3_999, &state_at(116)), None);
        assert_eq!(d.update(4_000, &state_at(116)), Some(Edge::Raised));
    }
}
