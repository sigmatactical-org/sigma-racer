//! Per-signal freshness tracking.
//!
//! Replaces the single global `Vehicle.Service.SignalsLive` boolean: each CAN
//! frame gets a freshness budget derived from its `rate_hz` (so a 1 Hz odometer
//! and a 50 Hz wheel-speed are judged on their own schedules, which one global
//! timeout could never do), and a producer marks frames as they arrive. The
//! result is a sparse map of the VSS paths that are currently stale or
//! unavailable — everything else is fresh.

use std::collections::HashMap;

use vss_map::{Availability, VssMap};

/// Frames must arrive within this many transmit periods to count as fresh.
const TTL_PERIODS: f64 = 3.0;
/// Freshness budgets never drop below this, so fast frames tolerate scheduling
/// jitter (the daemon samples on a 50 ms tick).
const TTL_FLOOR_MS: i64 = 250;

/// Embedded Sigma Racer frame map (keep in sync with wingman
/// `schemas/can/sigma-racer.yaml`).
const SIGMA_RACER_FRAME_MAP: &str = include_str!("data/sigma-racer.yaml");

struct FrameBudget {
    id: u32,
    /// Freshness window; `None` for event-driven frames (never age out).
    ttl_ms: Option<i64>,
    paths: Vec<String>,
}

/// Tracks when each frame last arrived and reports per-signal availability.
pub struct AvailabilityTracker {
    frames: Vec<FrameBudget>,
    last_seen: HashMap<u32, i64>,
}

impl AvailabilityTracker {
    /// Build budgets from a frame map's timings.
    pub fn from_vss_map(map: &VssMap) -> Self {
        let frames = map
            .frame_timings()
            .into_iter()
            .map(|t| FrameBudget {
                id: t.id,
                ttl_ms: (t.rate_hz > 0.0)
                    .then(|| ((TTL_PERIODS * 1000.0 / t.rate_hz) as i64).max(TTL_FLOOR_MS)),
                paths: t.paths.into_iter().map(|p| p.as_str().to_owned()).collect(),
            })
            .collect();
        Self {
            frames,
            last_seen: HashMap::new(),
        }
    }

    /// The tracker for the embedded Sigma Racer frame map.
    pub fn sigma_default() -> Self {
        let map = VssMap::from_frame_map_str(SIGMA_RACER_FRAME_MAP)
            .expect("embedded sigma-racer frame map must parse");
        Self::from_vss_map(&map)
    }

    /// Record that frame `id` arrived at `now_ms` (epoch milliseconds).
    pub fn mark(&mut self, id: u32, now_ms: i64) {
        self.last_seen.insert(id, now_ms);
    }

    /// Record that every known frame arrived at `now_ms` (the simulator and the
    /// rpmsg bridge deliver the whole signal set at once).
    pub fn mark_all(&mut self, now_ms: i64) {
        let ids: Vec<u32> = self.frames.iter().map(|f| f.id).collect();
        for id in ids {
            self.last_seen.insert(id, now_ms);
        }
    }

    fn frame_availability(&self, budget: &FrameBudget, now_ms: i64) -> Availability {
        match self.last_seen.get(&budget.id) {
            None => Availability::Unavailable,
            Some(&seen) => match budget.ttl_ms {
                Some(ttl) => Availability::from_age(now_ms.saturating_sub(seen), ttl),
                None => Availability::Available,
            },
        }
    }

    /// Sparse map of the VSS paths that are NOT available right now, keyed by
    /// path with a wire label (`"Stale"` / `"Unavailable"`). An empty map means
    /// everything is fresh.
    pub fn stale_paths(&self, now_ms: i64) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for budget in &self.frames {
            let availability = self.frame_availability(budget, now_ms);
            if let Some(label) = wire_label(availability) {
                for path in &budget.paths {
                    out.insert(path.clone(), label.to_owned());
                }
            }
        }
        out
    }
}

/// The signals whose freshness stands in for "the source is broadly alive" —
/// the derived `signals_live` rollup that feeds the staleness detector. Engine
/// speed and ground speed are the fastest always-on frames; if they are fresh
/// the bike is talking.
pub const LIVENESS_PATHS: [&str; 2] =
    ["Vehicle.Powertrain.CombustionEngine.Speed", "Vehicle.Speed"];

/// Derive the global `signals_live` rollup from a message's sparse availability
/// map: live unless a core liveness signal is missing or stale. A message with
/// no `avail` map is treated as fully live (nothing reported stale).
pub fn signals_live_from_avail(avail: Option<&HashMap<String, String>>) -> bool {
    match avail {
        None => true,
        Some(map) => !LIVENESS_PATHS.iter().any(|p| map.contains_key(*p)),
    }
}

/// Wire label for a non-available state; `None` when the value is fresh.
fn wire_label(availability: Availability) -> Option<&'static str> {
    match availability {
        Availability::Available => None,
        Availability::Stale => Some("Stale"),
        Availability::Unavailable => Some("Unavailable"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracker() -> AvailabilityTracker {
        AvailabilityTracker::sigma_default()
    }

    #[test]
    fn unseen_frames_are_unavailable() {
        let t = tracker();
        let stale = t.stale_paths(10_000);
        // Nothing marked yet → engine speed (0x0A0) is unavailable.
        assert_eq!(
            stale.get("Vehicle.Powertrain.CombustionEngine.Speed"),
            Some(&"Unavailable".to_string())
        );
    }

    #[test]
    fn fresh_frames_drop_out_of_the_sparse_map() {
        let mut t = tracker();
        t.mark_all(10_000);
        // Immediately after marking everything, nothing is stale.
        assert!(t.stale_paths(10_000).is_empty());
    }

    #[test]
    fn per_frame_budgets_reflect_rate_hz() {
        let mut t = tracker();
        t.mark_all(0);

        // ENGINE_STATUS is 50 Hz → ~250 ms floor. TRIP_ODOMETER is 1 Hz →
        // 3000 ms budget. At t=1000 ms the odometer paths are still fresh but
        // the engine paths have gone stale.
        let stale = t.stale_paths(1_000);
        assert_eq!(
            stale.get("Vehicle.Powertrain.CombustionEngine.Speed"),
            Some(&"Stale".to_string())
        );
        assert!(!stale.contains_key("Vehicle.TraveledDistance"));

        // By t=4000 ms even the 1 Hz odometer is stale.
        assert!(
            t.stale_paths(4_000)
                .contains_key("Vehicle.TraveledDistance")
        );
    }
}
