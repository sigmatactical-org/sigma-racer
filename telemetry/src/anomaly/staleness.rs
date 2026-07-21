//! Signal-feed health: stale source flag or outright silence.

use crate::state::VehicleState;

use super::types::{Edge, TsMillis};

/// Configuration for the staleness watchdog.
#[derive(Debug, Clone, Copy)]
pub struct StalenessConfig {
    /// Raise once `signals_live` has been false this long.
    pub stale_after_ms: TsMillis,
    /// Raise once no sample has been observed for this long (live paths only).
    pub silence_after_ms: TsMillis,
}

/// Watches the feed itself. `observe` runs per sample; `tick` runs on the
/// producer's own cadence to catch total silence — replay never calls `tick`,
/// so replayed sessions stay deterministic.
#[derive(Debug)]
pub struct StalenessDetector {
    cfg: StalenessConfig,
    stale_since: Option<TsMillis>,
    last_observed: Option<TsMillis>,
    active: bool,
}

impl StalenessDetector {
    pub fn new(cfg: StalenessConfig) -> Self {
        Self {
            cfg,
            stale_since: None,
            last_observed: None,
            active: false,
        }
    }

    /// Feed one timestamped sample; returns a transition when one occurs.
    pub fn observe(&mut self, ts: TsMillis, state: &VehicleState) -> Option<Edge> {
        self.last_observed = Some(ts);
        if state.signals_live {
            self.stale_since = None;
            if self.active {
                self.active = false;
                return Some(Edge::Cleared);
            }
            None
        } else {
            match self.stale_since {
                None => self.stale_since = Some(ts),
                Some(since) if ts < since => self.stale_since = Some(ts),
                Some(since) => {
                    if !self.active && ts - since >= self.cfg.stale_after_ms {
                        self.active = true;
                        return Some(Edge::Raised);
                    }
                }
            }
            None
        }
    }

    /// Producer-cadence check for total silence (no samples at all).
    pub fn tick(&mut self, ts: TsMillis) -> Option<Edge> {
        if let Some(last) = self.last_observed
            && !self.active
            && ts - last >= self.cfg.silence_after_ms
        {
            self.active = true;
            return Some(Edge::Raised);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> StalenessConfig {
        StalenessConfig {
            stale_after_ms: 1_000,
            silence_after_ms: 2_000,
        }
    }

    fn live(live: bool) -> VehicleState {
        VehicleState {
            signals_live: live,
            ..VehicleState::idle()
        }
    }

    #[test]
    fn sustained_stale_flag_raises_then_recovery_clears() {
        let mut d = StalenessDetector::new(cfg());
        assert_eq!(d.observe(0, &live(false)), None);
        assert_eq!(d.observe(999, &live(false)), None);
        assert_eq!(d.observe(1_000, &live(false)), Some(Edge::Raised));
        assert_eq!(d.observe(2_000, &live(true)), Some(Edge::Cleared));
    }

    #[test]
    fn silence_raises_via_tick() {
        let mut d = StalenessDetector::new(cfg());
        assert_eq!(d.observe(0, &live(true)), None);
        assert_eq!(d.tick(1_999), None);
        assert_eq!(d.tick(2_000), Some(Edge::Raised));
        // Samples resume: clears.
        assert_eq!(d.observe(3_000, &live(true)), Some(Edge::Cleared));
    }

    #[test]
    fn tick_before_any_sample_is_quiet() {
        let mut d = StalenessDetector::new(cfg());
        assert_eq!(d.tick(60_000), None);
    }
}
