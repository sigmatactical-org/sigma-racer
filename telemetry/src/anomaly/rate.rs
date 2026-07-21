//! Rate-of-change detector: signal rising faster than a limit over a window.

use crate::state::VehicleState;

use super::types::{Edge, SignalFn, TsMillis};

/// Ring capacity: at the daemon's 50 ms sample cadence this spans > 3 s; the
/// window anchor is the oldest retained sample not older than `window_ms`.
const RING: usize = 64;

/// Configuration for a rising-trend check.
#[derive(Debug, Clone, Copy)]
pub struct RateConfig {
    pub signal: SignalFn,
    /// Raise when the signal climbs by at least this much within `window_ms`.
    pub rise: f64,
    pub window_ms: TsMillis,
}

/// Fixed-size ring of samples; O(RING) bounded scan per update, no allocation.
#[derive(Debug)]
pub struct RateDetector {
    cfg: RateConfig,
    ring: [(TsMillis, f64); RING],
    len: usize,
    head: usize,
    active: bool,
}

impl RateDetector {
    pub fn new(cfg: RateConfig) -> Self {
        Self {
            cfg,
            ring: [(0, 0.0); RING],
            len: 0,
            head: 0,
            active: false,
        }
    }

    fn reset(&mut self) {
        self.len = 0;
        self.head = 0;
    }

    fn last_ts(&self) -> Option<TsMillis> {
        if self.len == 0 {
            return None;
        }
        let idx = (self.head + RING - 1) % RING;
        Some(self.ring[idx].0)
    }

    /// Feed one timestamped sample; returns a transition when one occurs.
    pub fn update(&mut self, ts: TsMillis, state: &VehicleState) -> Option<Edge> {
        let v = (self.cfg.signal)(state);
        if let Some(last) = self.last_ts()
            && ts < last
        {
            // Backwards timestamp: history is no longer comparable.
            self.reset();
        }
        self.ring[self.head] = (ts, v);
        self.head = (self.head + 1) % RING;
        self.len = (self.len + 1).min(RING);

        // Oldest retained sample still inside the window.
        let mut anchor: Option<(TsMillis, f64)> = None;
        for i in 0..self.len {
            let idx = (self.head + RING - self.len + i) % RING;
            let (t0, v0) = self.ring[idx];
            if ts - t0 <= self.cfg.window_ms {
                anchor = Some((t0, v0));
                break;
            }
        }
        let (t0, v0) = anchor?;
        // Need most of a window of history before the slope is meaningful.
        if ts - t0 < self.cfg.window_ms * 8 / 10 {
            return None;
        }
        let rise = v - v0;
        if !self.active && rise >= self.cfg.rise {
            self.active = true;
            Some(Edge::Raised)
        } else if self.active && rise <= self.cfg.rise / 2.0 {
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

    fn coolant(state: &VehicleState) -> f64 {
        f64::from(state.coolant_c)
    }

    fn cfg() -> RateConfig {
        RateConfig {
            signal: coolant,
            rise: 3.0,
            window_ms: 10_000,
        }
    }

    fn state_at(coolant_c: i16) -> VehicleState {
        VehicleState {
            coolant_c,
            ..VehicleState::idle()
        }
    }

    #[test]
    fn slow_drift_stays_quiet() {
        let mut d = RateDetector::new(cfg());
        // +1 °C per 10 s: well under the +3 °C limit.
        for i in 0..120i64 {
            let ts = i * 500;
            let c = 80 + i16::try_from(i / 20).unwrap();
            assert_eq!(d.update(ts, &state_at(c)), None, "at {ts}");
        }
    }

    #[test]
    fn fast_ramp_raises_then_plateau_clears() {
        let mut d = RateDetector::new(cfg());
        let mut raised_at = None;
        // +1 °C per second: crosses +3 °C/10 s once a window of history exists.
        for i in 0..30i64 {
            let ts = i * 1_000;
            let c = 80 + i16::try_from(i).unwrap();
            if let Some(Edge::Raised) = d.update(ts, &state_at(c)) {
                raised_at = Some(ts);
                break;
            }
        }
        let raised_at = raised_at.expect("ramp should raise");
        // Plateau: rise over the window decays below half the limit → clears.
        let mut cleared = false;
        for i in 0..30i64 {
            let ts = raised_at + (i + 1) * 1_000;
            if let Some(Edge::Cleared) = d.update(ts, &state_at(110)) {
                cleared = true;
                break;
            }
        }
        assert!(cleared, "plateau should clear the trend alert");
    }

    #[test]
    fn backwards_ts_resets_history() {
        let mut d = RateDetector::new(cfg());
        for i in 0..12i64 {
            let _ = d.update(i * 1_000, &state_at(80 + i16::try_from(i).unwrap()));
        }
        // Jump back in time: no stale anchor may survive.
        assert_eq!(d.update(500, &state_at(120)), None);
    }
}
