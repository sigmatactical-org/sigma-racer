//! Illegal-combination and counter-increment detectors.

use crate::state::VehicleState;

use super::types::{Edge, PredicateFn, TsMillis};

/// Configuration for an illegal state combination with a short debounce.
#[derive(Debug, Clone, Copy)]
pub struct SequenceConfig {
    pub illegal: PredicateFn,
    /// Combination must persist this long before raising (decode-glitch filter).
    pub debounce_ms: TsMillis,
}

/// Pure state machine; O(1) per sample, no allocation.
#[derive(Debug)]
pub struct SequenceDetector {
    cfg: SequenceConfig,
    pending: Option<TsMillis>,
    active: bool,
}

impl SequenceDetector {
    pub fn new(cfg: SequenceConfig) -> Self {
        Self {
            cfg,
            pending: None,
            active: false,
        }
    }

    /// Feed one timestamped sample; returns a transition when one occurs.
    pub fn update(&mut self, ts: TsMillis, state: &VehicleState) -> Option<Edge> {
        if (self.cfg.illegal)(state) {
            if self.active {
                return None;
            }
            match self.pending {
                None => self.pending = Some(ts),
                Some(since) if ts < since => self.pending = Some(ts),
                Some(since) => {
                    if ts - since >= self.cfg.debounce_ms {
                        self.pending = None;
                        self.active = true;
                        return Some(Edge::Raised);
                    }
                }
            }
            None
        } else {
            self.pending = None;
            if self.active {
                self.active = false;
                Some(Edge::Cleared)
            } else {
                None
            }
        }
    }
}

/// Raises when a fault counter becomes non-zero (or increases), clears at zero.
#[derive(Debug)]
pub struct CounterDetector {
    counter: fn(&VehicleState) -> u32,
    prev: Option<u32>,
    active: bool,
}

impl CounterDetector {
    pub fn new(counter: fn(&VehicleState) -> u32) -> Self {
        Self {
            counter,
            prev: None,
            active: false,
        }
    }

    /// Feed one sample; returns a transition when one occurs.
    pub fn update(&mut self, _ts: TsMillis, state: &VehicleState) -> Option<Edge> {
        let cur = (self.counter)(state);
        let prev = self.prev.replace(cur).unwrap_or(0);
        if cur > 0 && (!self.active || cur > prev) {
            let was_active = self.active;
            self.active = true;
            // Re-raise on further increments; the alert manager dedups actives.
            if !was_active || cur > prev {
                return Some(Edge::Raised);
            }
            None
        } else if cur == 0 && self.active {
            self.active = false;
            Some(Edge::Cleared)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stand_combo(state: &VehicleState) -> bool {
        state.side_stand && state.gear != 0 && state.speed > 5.0
    }

    fn moving(side_stand: bool, gear: i8, speed: f32) -> VehicleState {
        VehicleState {
            side_stand,
            gear,
            speed,
            ..VehicleState::idle()
        }
    }

    #[test]
    fn illegal_combo_raises_after_debounce_and_clears() {
        let mut d = SequenceDetector::new(SequenceConfig {
            illegal: stand_combo,
            debounce_ms: 200,
        });
        assert_eq!(d.update(0, &moving(true, 1, 20.0)), None);
        assert_eq!(d.update(100, &moving(true, 1, 20.0)), None);
        assert_eq!(d.update(200, &moving(true, 1, 20.0)), Some(Edge::Raised));
        assert_eq!(d.update(300, &moving(false, 1, 20.0)), Some(Edge::Cleared));
    }

    #[test]
    fn glitch_shorter_than_debounce_is_ignored() {
        let mut d = SequenceDetector::new(SequenceConfig {
            illegal: stand_combo,
            debounce_ms: 200,
        });
        assert_eq!(d.update(0, &moving(true, 1, 20.0)), None);
        // Gone before 200 ms elapsed.
        assert_eq!(d.update(100, &moving(false, 1, 20.0)), None);
        assert_eq!(d.update(500, &moving(false, 1, 20.0)), None);
    }

    #[test]
    fn counter_raises_on_increment_and_clears_at_zero() {
        fn dtc(state: &VehicleState) -> u32 {
            u32::from(state.dtc)
        }
        let mut d = CounterDetector::new(dtc);
        let mut s = VehicleState::idle();
        assert_eq!(d.update(0, &s), None);
        s.dtc = 1;
        assert_eq!(d.update(1_000, &s), Some(Edge::Raised));
        assert_eq!(d.update(2_000, &s), None);
        // A further code re-raises (manager dedups the active alert).
        s.dtc = 2;
        assert_eq!(d.update(3_000, &s), Some(Edge::Raised));
        s.dtc = 0;
        assert_eq!(d.update(4_000, &s), Some(Edge::Cleared));
    }

    #[test]
    fn session_starting_with_stored_codes_raises() {
        fn dtc(state: &VehicleState) -> u32 {
            u32::from(state.dtc)
        }
        let mut d = CounterDetector::new(dtc);
        let s = VehicleState {
            dtc: 3,
            ..VehicleState::idle()
        };
        assert_eq!(d.update(0, &s), Some(Edge::Raised));
    }
}
