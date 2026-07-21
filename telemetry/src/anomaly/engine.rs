//! The stock Sigma Racer detector set, bundled for producers and consumers.

use serde_json::json;

use crate::protocol::Message;
use crate::state::VehicleState;

use super::consistency::{ConsistencyConfig, ConsistencyDetector};
use super::event::AnomalyEvent;
use super::manager::{AlertManager, AlertMeta, AlertSlot, SUPPRESSOR_ID};
use super::rate::{RateConfig, RateDetector};
use super::sequence::{CounterDetector, SequenceConfig, SequenceDetector};
use super::staleness::{StalenessConfig, StalenessDetector};
use super::threshold::{ThresholdConfig, ThresholdDetector};
use super::types::{Category, Severity, TsMillis};

// —— Tunables (single source of truth for the stock detector set) ——
/// `signals_live` false this long → stale.
pub const STALE_AFTER_MS: TsMillis = 1_000;
/// No samples at all this long → stale (live producers only).
pub const SILENCE_AFTER_MS: TsMillis = 2_000;
/// Charging check: rpm above this with battery below the floor.
pub const CHARGING_RPM: f32 = 3_000.0;
pub const CHARGING_MIN_V: f32 = 12.6;
pub const CHARGING_SUSTAIN_MS: TsMillis = 5_000;
pub const CHARGING_CLEAR_HOLD_MS: TsMillis = 3_000;
/// Coolant absolute limit and hysteresis clear point (°C).
pub const COOLANT_LIMIT_C: f64 = 115.0;
pub const COOLANT_CLEAR_C: f64 = 108.0;
pub const COOLANT_SUSTAIN_MS: TsMillis = 3_000;
/// Coolant trend: this many °C within the window is abnormal.
pub const COOLANT_RISE_C: f64 = 3.0;
pub const COOLANT_RISE_WINDOW_MS: TsMillis = 10_000;
/// Side-stand interlock: moving faster than this in gear with the stand down.
pub const STAND_SPEED_KMH: f32 = 5.0;
pub const STAND_DEBOUNCE_MS: TsMillis = 200;

fn coolant(state: &VehicleState) -> f64 {
    f64::from(state.coolant_c)
}

fn not_charging(state: &VehicleState) -> bool {
    state.rpm > CHARGING_RPM && state.battery_v < CHARGING_MIN_V
}

fn stand_combo(state: &VehicleState) -> bool {
    state.side_stand && state.gear != 0 && state.speed > STAND_SPEED_KMH
}

fn dtc_count(state: &VehicleState) -> u32 {
    u32::from(state.dtc)
}

const META_STALE: AlertMeta = AlertMeta {
    id: SUPPRESSOR_ID,
    severity: Severity::Warning,
    category: Category::SensorFault,
    vss: "Vehicle.Service.SignalsLive",
    describe: |_| "Signal feed stale — decoded values are unreliable".to_string(),
    value: |state| json!(state.signals_live),
};

const META_NOT_CHARGING: AlertMeta = AlertMeta {
    id: "not_charging",
    severity: Severity::Warning,
    category: Category::StateBased,
    vss: "Vehicle.ElectricalSystem.Battery.Voltage",
    describe: |state| {
        format!(
            "Battery {:.1} V at {:.0} rpm — charging system not keeping up",
            state.battery_v, state.rpm
        )
    },
    value: |state| json!(state.battery_v),
};

const META_OVERHEAT: AlertMeta = AlertMeta {
    id: "coolant_overheat",
    severity: Severity::Critical,
    category: Category::StateBased,
    vss: "Vehicle.OBD.CoolantTemperature",
    describe: |state| {
        format!(
            "Coolant {} °C — overheating (limit {COOLANT_LIMIT_C} °C)",
            state.coolant_c
        )
    },
    value: |state| json!(state.coolant_c),
};

const META_RISING: AlertMeta = AlertMeta {
    id: "coolant_rising",
    severity: Severity::Warning,
    category: Category::StateBased,
    vss: "Vehicle.OBD.CoolantTemperature",
    describe: |state| {
        format!(
            "Coolant {} °C and climbing abnormally fast",
            state.coolant_c
        )
    },
    value: |state| json!(state.coolant_c),
};

const META_STAND: AlertMeta = AlertMeta {
    id: "side_stand_interlock",
    severity: Severity::Critical,
    category: Category::StateBased,
    vss: "Vehicle.Body.IsSideStandEngaged",
    describe: |state| {
        format!(
            "Side stand down at {:.0} km/h in gear {} — interlock did not cut",
            state.speed, state.gear
        )
    },
    value: |state| json!(state.side_stand),
};

const META_DTC: AlertMeta = AlertMeta {
    id: "dtc_appeared",
    severity: Severity::Warning,
    category: Category::StateBased,
    vss: "Vehicle.OBD.DTCCount",
    describe: |state| format!("{} diagnostic trouble code(s) stored", state.dtc),
    value: |state| json!(state.dtc),
};

/// The bundled detector set + alert manager both the bike daemon and the shop
/// tools run. Feed timestamped state samples in; drain [`AnomalyEvent`]s out.
#[derive(Debug)]
pub struct AnomalyEngine {
    staleness: StalenessDetector,
    charging: ConsistencyDetector,
    overheat: ThresholdDetector,
    rising: RateDetector,
    stand: SequenceDetector,
    dtc: CounterDetector,
    manager: AlertManager,
    out: Vec<AnomalyEvent>,
}

impl AnomalyEngine {
    /// The stock Sigma Racer configuration (tunables above).
    pub fn sigma_defaults() -> Self {
        let mut manager = AlertManager::new();
        for meta in [
            &META_STALE,
            &META_NOT_CHARGING,
            &META_OVERHEAT,
            &META_RISING,
            &META_STAND,
            &META_DTC,
        ] {
            manager.register(meta);
        }
        Self {
            staleness: StalenessDetector::new(StalenessConfig {
                stale_after_ms: STALE_AFTER_MS,
                silence_after_ms: SILENCE_AFTER_MS,
            }),
            charging: ConsistencyDetector::new(ConsistencyConfig {
                predicate: not_charging,
                sustain_ms: CHARGING_SUSTAIN_MS,
                clear_hold_ms: CHARGING_CLEAR_HOLD_MS,
            }),
            overheat: ThresholdDetector::new(ThresholdConfig {
                signal: coolant,
                raise_at: COOLANT_LIMIT_C,
                clear_below: COOLANT_CLEAR_C,
                sustain_ms: COOLANT_SUSTAIN_MS,
            }),
            rising: RateDetector::new(RateConfig {
                signal: coolant,
                rise: COOLANT_RISE_C,
                window_ms: COOLANT_RISE_WINDOW_MS,
            }),
            stand: SequenceDetector::new(SequenceConfig {
                illegal: stand_combo,
                debounce_ms: STAND_DEBOUNCE_MS,
            }),
            dtc: CounterDetector::new(dtc_count),
            manager,
            out: Vec::new(),
        }
    }

    /// Feed one timestamped state sample; returns the events that fired.
    pub fn observe(&mut self, ts: TsMillis, state: &VehicleState) -> &[AnomalyEvent] {
        self.out.clear();
        // Staleness first: it gates the state-based detectors below.
        if let Some(edge) = self.staleness.observe(ts, state)
            && let Some(ev) = self.manager.on_edge(&META_STALE, edge, ts, state)
        {
            self.out.push(ev);
        }
        if let Some(edge) = self.charging.update(ts, state)
            && let Some(ev) = self.manager.on_edge(&META_NOT_CHARGING, edge, ts, state)
        {
            self.out.push(ev);
        }
        if let Some(edge) = self.overheat.update(ts, state)
            && let Some(ev) = self.manager.on_edge(&META_OVERHEAT, edge, ts, state)
        {
            self.out.push(ev);
        }
        if let Some(edge) = self.rising.update(ts, state)
            && let Some(ev) = self.manager.on_edge(&META_RISING, edge, ts, state)
        {
            self.out.push(ev);
        }
        if let Some(edge) = self.stand.update(ts, state)
            && let Some(ev) = self.manager.on_edge(&META_STAND, edge, ts, state)
        {
            self.out.push(ev);
        }
        if let Some(edge) = self.dtc.update(ts, state)
            && let Some(ev) = self.manager.on_edge(&META_DTC, edge, ts, state)
        {
            self.out.push(ev);
        }
        &self.out
    }

    /// Producer-cadence silence check (live paths only; never during replay).
    pub fn tick(&mut self, ts: TsMillis) -> &[AnomalyEvent] {
        self.out.clear();
        if let Some(edge) = self.staleness.tick(ts) {
            // No fresh state to describe: synthesize from an idle placeholder.
            let placeholder = VehicleState::idle();
            if let Some(ev) = self.manager.on_edge(&META_STALE, edge, ts, &placeholder) {
                self.out.push(ev);
            }
        }
        &self.out
    }

    /// Merge an `Event` message from another engine (see
    /// [`AlertManager::apply_external`]).
    pub fn ingest_event(&mut self, msg: &Message) -> Option<AnomalyEvent> {
        self.manager.apply_external(msg)
    }

    /// Highest-severity active alert.
    pub fn worst_active(&self) -> Option<(&'static str, Severity)> {
        self.manager.worst_active()
    }

    /// All currently active alerts.
    pub fn active(&self) -> impl Iterator<Item = &AlertSlot> {
        self.manager.active()
    }

    /// Acknowledge a latched alert.
    pub fn ack(&mut self, id: &str) {
        self.manager.ack(id);
    }

    /// Deactivate everything (new session).
    pub fn reset(&mut self) {
        self.manager.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::anomaly::types::Edge;

    fn riding() -> VehicleState {
        VehicleState {
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
        }
    }

    #[test]
    fn nominal_riding_is_quiet() {
        let mut engine = AnomalyEngine::sigma_defaults();
        for i in 0..600i64 {
            assert!(
                engine.observe(i * 100, &riding()).is_empty(),
                "false positive at {i}"
            );
        }
        assert!(engine.worst_active().is_none());
    }

    #[test]
    fn overheat_latches_until_ack() {
        let mut engine = AnomalyEngine::sigma_defaults();
        let mut hot = riding();
        hot.coolant_c = 118;
        let mut raised = false;
        for i in 0..100i64 {
            for ev in engine.observe(i * 100, &hot) {
                if ev.id == "coolant_overheat" && ev.edge == Edge::Raised {
                    raised = true;
                }
            }
        }
        assert!(raised);
        // Condition clears, but the Critical alert stays latched.
        let cool = riding();
        for i in 100..200i64 {
            for ev in engine.observe(i * 100, &cool) {
                assert_ne!(ev.id, "coolant_overheat", "latched alert must not clear");
            }
        }
        assert_eq!(
            engine.worst_active().map(|(id, _)| id),
            Some("coolant_overheat")
        );
        engine.ack("coolant_overheat");
        assert_ne!(
            engine.worst_active().map(|(id, _)| id),
            Some("coolant_overheat")
        );
    }

    #[test]
    fn stale_feed_suppresses_state_based_raises() {
        let mut engine = AnomalyEngine::sigma_defaults();
        let mut stale_hot = riding();
        stale_hot.signals_live = false;
        stale_hot.coolant_c = 130; // implausible while stale
        let mut ids = Vec::new();
        for i in 0..200i64 {
            for ev in engine.observe(i * 100, &stale_hot) {
                ids.push(ev.id.clone());
            }
        }
        assert_eq!(ids, vec!["signal_stale"], "only the stale alert may fire");
    }

    #[test]
    fn external_event_is_deduplicated_against_local_state() {
        let mut engine = AnomalyEngine::sigma_defaults();
        let mut low = riding();
        low.battery_v = 12.0;
        // Raise locally.
        let mut raised_ev = None;
        for i in 0..100i64 {
            for ev in engine.observe(i * 100, &low) {
                if ev.id == "not_charging" {
                    raised_ev = Some(ev.clone());
                }
            }
        }
        let raised_ev = raised_ev.expect("local raise");
        // The bike reports the same alert: deduplicated.
        let msg = raised_ev.to_message(1);
        assert!(engine.ingest_event(&msg).is_none());
    }
}
