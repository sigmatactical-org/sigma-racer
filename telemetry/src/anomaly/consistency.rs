//! Cross-signal consistency detector: a predicate over the whole state,
//! sustained to raise and held false to clear.

use crate::state::VehicleState;

use super::types::{Edge, PredicateFn, TsMillis};

/// Configuration for a sustained cross-signal check.
#[derive(Debug, Clone, Copy)]
pub struct ConsistencyConfig {
    pub predicate: PredicateFn,
    /// Predicate must hold this long before raising.
    pub sustain_ms: TsMillis,
    /// Predicate must stay false this long before clearing (0 = immediate).
    pub clear_hold_ms: TsMillis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Pending(TsMillis),
    Active,
    Recovering(TsMillis),
}

/// Pure state machine; O(1) per sample, no allocation.
#[derive(Debug)]
pub struct ConsistencyDetector {
    cfg: ConsistencyConfig,
    phase: Phase,
}

impl ConsistencyDetector {
    pub fn new(cfg: ConsistencyConfig) -> Self {
        Self {
            cfg,
            phase: Phase::Idle,
        }
    }

    /// Feed one timestamped sample; returns a transition when one occurs.
    pub fn update(&mut self, ts: TsMillis, state: &VehicleState) -> Option<Edge> {
        let hit = (self.cfg.predicate)(state);
        match self.phase {
            Phase::Idle => {
                if hit {
                    self.phase = Phase::Pending(ts);
                }
                None
            }
            Phase::Pending(since) => {
                if !hit {
                    self.phase = Phase::Idle;
                    None
                } else if ts < since {
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
                if !hit {
                    if self.cfg.clear_hold_ms == 0 {
                        self.phase = Phase::Idle;
                        return Some(Edge::Cleared);
                    }
                    self.phase = Phase::Recovering(ts);
                }
                None
            }
            Phase::Recovering(since) => {
                if hit {
                    self.phase = Phase::Active;
                    None
                } else if ts < since {
                    self.phase = Phase::Recovering(ts);
                    None
                } else if ts - since >= self.cfg.clear_hold_ms {
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

    fn not_charging(state: &VehicleState) -> bool {
        state.rpm > 3_000.0 && f64::from(state.battery_v) < 12.6
    }

    fn cfg() -> ConsistencyConfig {
        ConsistencyConfig {
            predicate: not_charging,
            sustain_ms: 5_000,
            clear_hold_ms: 3_000,
        }
    }

    fn state_at(rpm: f32, battery_v: f32) -> VehicleState {
        VehicleState {
            rpm,
            battery_v,
            ..VehicleState::idle()
        }
    }

    #[test]
    fn sustained_predicate_raises_then_held_recovery_clears() {
        let mut d = ConsistencyDetector::new(cfg());
        assert_eq!(d.update(0, &state_at(4_000.0, 12.1)), None);
        assert_eq!(d.update(4_999, &state_at(4_000.0, 12.1)), None);
        assert_eq!(
            d.update(5_000, &state_at(4_000.0, 12.1)),
            Some(Edge::Raised)
        );
        // Recovery must hold 3 s before clearing.
        assert_eq!(d.update(6_000, &state_at(4_000.0, 14.1)), None);
        assert_eq!(d.update(7_000, &state_at(4_000.0, 14.1)), None);
        assert_eq!(
            d.update(9_000, &state_at(4_000.0, 14.1)),
            Some(Edge::Cleared)
        );
    }

    #[test]
    fn brief_recovery_does_not_clear() {
        let mut d = ConsistencyDetector::new(cfg());
        let _ = d.update(0, &state_at(4_000.0, 12.1));
        assert_eq!(
            d.update(5_000, &state_at(4_000.0, 12.1)),
            Some(Edge::Raised)
        );
        // Blip of good voltage, then bad again: still active.
        assert_eq!(d.update(6_000, &state_at(4_000.0, 14.1)), None);
        assert_eq!(d.update(7_000, &state_at(4_000.0, 12.0)), None);
        assert_eq!(d.update(15_000, &state_at(4_000.0, 12.0)), None);
    }

    #[test]
    fn idle_engine_never_raises() {
        let mut d = ConsistencyDetector::new(cfg());
        // Low voltage at low rpm is normal (engine off / idling).
        for i in 0..100i64 {
            assert_eq!(d.update(i * 1_000, &state_at(1_200.0, 12.1)), None);
        }
    }
}
