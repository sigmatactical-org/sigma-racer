//! Replay-driven anomaly detection tests: deterministic NDJSON rides in,
//! exact event sequences out. This is the offline test story — no bike, no
//! network, no clock.

use std::collections::HashMap;

use sigma_racer_telemetry::anomaly::{AnomalyEngine, Edge, Severity};
use sigma_racer_telemetry::availability::LIVENESS_PATHS;
use sigma_racer_telemetry::protocol::Message;
use sigma_racer_telemetry::{VehicleState, parse_ts_millis};

/// Encode a scenario's `signals_live` intent as the sparse availability map the
/// daemon would send: empty when live, core liveness paths marked unavailable
/// when not.
fn avail_for(state: &VehicleState) -> HashMap<String, String> {
    if state.signals_live {
        HashMap::new()
    } else {
        LIVENESS_PATHS
            .iter()
            .map(|p| (p.to_string(), "Unavailable".to_string()))
            .collect()
    }
}

/// Fixed session start (2026-07-18T10:00:00Z) so fixture timestamps are stable.
const EPOCH_MS: i64 = 1_784_455_200_000;

/// Builds a deterministic NDJSON ride: snapshots at a fixed cadence from a
/// mutating `VehicleState`, exactly like the daemon records them.
struct RideBuilder {
    state: VehicleState,
    ts_ms: i64,
    seq: u64,
    lines: Vec<String>,
}

impl RideBuilder {
    fn new() -> Self {
        let state = VehicleState {
            signals_live: true,
            side_stand: false,
            gear: 3,
            speed: 80.0,
            rpm: 6_000.0,
            battery_v: 14.0,
            coolant_c: 88,
            oil_c: 95,
            dtc: 0,
            ..VehicleState::idle()
        };
        Self {
            state,
            ts_ms: EPOCH_MS,
            seq: 0,
            lines: Vec::new(),
        }
    }

    /// Mutate the state, then emit one snapshot line `step_ms` later. The
    /// scenario's `signals_live` intent is carried as the per-signal `avail`
    /// map, exactly as the daemon now emits it.
    fn step(&mut self, step_ms: i64, mutate: impl FnOnce(&mut VehicleState)) {
        mutate(&mut self.state);
        self.ts_ms += step_ms;
        self.seq += 1;
        let mut msg = Message::snapshot(self.seq, &self.state).with_avail(avail_for(&self.state));
        msg.ts = ts_string(self.ts_ms);
        self.lines.push(msg.to_line());
    }

    /// Emit `n` unchanged snapshots at `step_ms` cadence.
    fn cruise(&mut self, n: usize, step_ms: i64) {
        for _ in 0..n {
            self.step(step_ms, |_| {});
        }
    }
}

fn ts_string(ts_ms: i64) -> String {
    use chrono::{SecondsFormat, TimeZone, Utc};
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .expect("valid ts")
        .to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Replay NDJSON lines through a fresh engine exactly as a shop consumer
/// would: rebuild state per line, then observe with the line's own timestamp.
fn replay(lines: &[String]) -> Vec<(String, Edge, i64)> {
    let mut engine = AnomalyEngine::sigma_defaults();
    let mut state = VehicleState::idle();
    let mut out = Vec::new();
    for line in lines {
        let msg = Message::parse_validated(line).expect("fixture line parses");
        if let Some(data) = msg.vss_data() {
            state.apply_vss_map(data);
        }
        state.apply_availability(msg.avail_data());
        let ts = parse_ts_millis(&msg.ts).expect("fixture ts parses");
        for ev in engine.observe(ts, &state) {
            out.push((ev.id.clone(), ev.edge, ev.ts_ms));
        }
    }
    out
}

#[test]
fn nominal_ride_is_quiet() {
    let mut ride = RideBuilder::new();
    // 2 minutes of ordinary riding: gentle warm-up, healthy charging.
    for i in 0..1_200i64 {
        ride.step(100, |s| {
            s.coolant_c = 88 + i16::try_from(i / 300).unwrap(); // +1 °C per 30 s
            s.battery_v = 14.0 + ((i % 20) as f32) * 0.01; // small ripple
            s.rpm = 5_500.0 + ((i % 40) as f32) * 25.0;
        });
    }
    let events = replay(&ride.lines);
    assert!(
        events.is_empty(),
        "false positives on a nominal ride: {events:?}"
    );
}

#[test]
fn coolant_ramp_raises_rising_then_overheat() {
    let mut ride = RideBuilder::new();
    ride.cruise(50, 100); // 5 s settled
    // Coolant climbs 1 °C per second: trend fires first, then the limit.
    for i in 0..40i64 {
        ride.step(1_000, |s| s.coolant_c = 90 + i16::try_from(i).unwrap());
    }
    let events = replay(&ride.lines);
    let ids: Vec<&str> = events.iter().map(|(id, _, _)| id.as_str()).collect();
    let rising = ids.iter().position(|id| *id == "coolant_rising");
    let overheat = ids.iter().position(|id| *id == "coolant_overheat");
    assert!(rising.is_some(), "trend alert missing: {events:?}");
    assert!(overheat.is_some(), "overheat alert missing: {events:?}");
    assert!(
        rising.unwrap() < overheat.unwrap(),
        "trend must fire before the absolute limit: {events:?}"
    );
    // The overheat raise lands once 115 °C has been sustained 3 s.
    let (_, edge, ts) = &events[overheat.unwrap()];
    assert_eq!(*edge, Edge::Raised);
    let cross_ms = EPOCH_MS + 5_000 + 26_000; // coolant hits 115 °C at +26 s of ramp
    assert_eq!(*ts, cross_ms + 3_000, "sustain window must be exact");
}

#[test]
fn battery_sag_raises_not_charging_then_recovery_clears() {
    let mut ride = RideBuilder::new();
    ride.cruise(20, 100);
    // Voltage sags while revving: charging fault.
    for _ in 0..70 {
        ride.step(100, |s| s.battery_v = 12.1);
    }
    // Regulator comes back: recovery holds, alert clears.
    for _ in 0..70 {
        ride.step(100, |s| s.battery_v = 14.1);
    }
    let events = replay(&ride.lines);
    let raised = events
        .iter()
        .any(|(id, e, _)| id == "not_charging" && *e == Edge::Raised);
    let cleared = events
        .iter()
        .any(|(id, e, _)| id == "not_charging" && *e == Edge::Cleared);
    assert!(raised, "sag must raise: {events:?}");
    assert!(cleared, "recovery must clear: {events:?}");
}

#[test]
fn gap_raises_signal_stale_and_suppresses_state_alerts() {
    let mut ride = RideBuilder::new();
    ride.cruise(20, 100);
    // Feed goes stale AND the (unreliable) coolant reads implausibly hot:
    // only the stale alert may fire.
    for _ in 0..100 {
        ride.step(100, |s| {
            s.signals_live = false;
            s.coolant_c = 130;
        });
    }
    let events = replay(&ride.lines);
    let ids: Vec<&str> = events.iter().map(|(id, _, _)| id.as_str()).collect();
    assert_eq!(ids, vec!["signal_stale"], "suppression failed: {events:?}");
}

#[test]
fn dtc_bump_and_side_stand_combo_raise() {
    let mut ride = RideBuilder::new();
    ride.cruise(20, 100);
    ride.step(100, |s| s.dtc = 1);
    ride.cruise(5, 100);
    // Side stand drops while moving in gear.
    for _ in 0..5 {
        ride.step(100, |s| s.side_stand = true);
    }
    let events = replay(&ride.lines);
    let ids: Vec<&str> = events.iter().map(|(id, _, _)| id.as_str()).collect();
    assert!(ids.contains(&"dtc_appeared"), "{events:?}");
    assert!(ids.contains(&"side_stand_interlock"), "{events:?}");
    // The interlock is Critical: it must be the worst active at the end.
    let mut engine = AnomalyEngine::sigma_defaults();
    let mut state = VehicleState::idle();
    for line in &ride.lines {
        let msg = Message::parse_validated(line).unwrap();
        if let Some(data) = msg.vss_data() {
            state.apply_vss_map(data);
        }
        state.apply_availability(msg.avail_data());
        engine.observe(parse_ts_millis(&msg.ts).unwrap(), &state);
    }
    assert_eq!(
        engine.worst_active(),
        Some(("side_stand_interlock", Severity::Critical))
    );
}

#[test]
fn replay_is_deterministic() {
    let mut ride = RideBuilder::new();
    ride.cruise(30, 100);
    for i in 0..30i64 {
        ride.step(1_000, |s| {
            s.coolant_c = 95 + i16::try_from(i).unwrap();
            s.battery_v = 12.0;
        });
    }
    let first = replay(&ride.lines);
    let second = replay(&ride.lines);
    assert!(!first.is_empty(), "scenario should produce events");
    assert_eq!(first, second, "same session must produce identical events");
}

/// Regenerates the checked-in fixtures when they drift (run with
/// `UPDATE_FIXTURES=1 cargo test --test anomaly_replay`); otherwise verifies
/// the committed files still produce the expected alerts.
#[test]
fn checked_in_fixtures_match() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let nominal_path = dir.join("nominal-ride.jsonl");
    let faulty_path = dir.join("faulty-ride.jsonl");

    let nominal = build_nominal_fixture();
    let faulty = build_faulty_fixture();

    if std::env::var("UPDATE_FIXTURES").is_ok() {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&nominal_path, nominal.join("\n") + "\n").unwrap();
        std::fs::write(&faulty_path, faulty.join("\n") + "\n").unwrap();
    }

    let read = |p: &std::path::Path| -> Vec<String> {
        std::fs::read_to_string(p)
            .unwrap_or_else(|e| {
                panic!(
                    "missing fixture {} ({e}) — run UPDATE_FIXTURES=1",
                    p.display()
                )
            })
            .lines()
            .map(str::to_string)
            .collect()
    };

    assert!(
        replay(&read(&nominal_path)).is_empty(),
        "nominal fixture must stay quiet"
    );
    let ids: Vec<String> = replay(&read(&faulty_path))
        .into_iter()
        .filter(|(_, e, _)| *e == Edge::Raised)
        .map(|(id, _, _)| id)
        .collect();
    assert_eq!(
        ids,
        vec!["coolant_rising", "coolant_overheat", "not_charging"],
        "faulty fixture alert sequence changed"
    );
}

fn build_nominal_fixture() -> Vec<String> {
    let mut ride = RideBuilder::new();
    for i in 0..900i64 {
        ride.step(100, |s| {
            s.coolant_c = 86 + i16::try_from(i / 300).unwrap();
            s.battery_v = 14.0 + ((i % 20) as f32) * 0.01;
            s.rpm = 5_000.0 + ((i % 50) as f32) * 30.0;
            s.speed = 70.0 + ((i % 40) as f32) * 0.5;
        });
    }
    ride.lines
}

fn build_faulty_fixture() -> Vec<String> {
    let mut ride = RideBuilder::new();
    ride.cruise(50, 100);
    // Coolant ramp into overheat…
    for i in 0..35i64 {
        ride.step(1_000, |s| s.coolant_c = 90 + i16::try_from(i).unwrap());
    }
    // …then a charging fault on top.
    for _ in 0..80 {
        ride.step(100, |s| s.battery_v = 12.0);
    }
    ride.lines
}
