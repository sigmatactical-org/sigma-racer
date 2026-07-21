//! Alert lifecycle: latching, suppression, worst-active selection.

use serde_json::Value;

use crate::protocol::Message;
use crate::state::VehicleState;

use super::event::AnomalyEvent;
use super::types::{Category, Edge, Severity, TsMillis};

/// Id of the alert whose activity suppresses state-based detections.
pub const SUPPRESSOR_ID: &str = "signal_stale";

/// Static description of one configured alert (shared by manager and engine).
#[derive(Debug, Clone, Copy)]
pub struct AlertMeta {
    pub id: &'static str,
    pub severity: Severity,
    pub category: Category,
    pub vss: &'static str,
    /// One-liner built only when an edge fires.
    pub describe: fn(&VehicleState) -> String,
    /// Triggering value extracted from the state.
    pub value: fn(&VehicleState) -> Value,
}

/// Tracked lifecycle of one alert.
#[derive(Debug, Clone)]
pub struct AlertSlot {
    pub id: &'static str,
    pub severity: Severity,
    pub category: Category,
    pub active: bool,
    /// Critical alerts latch: the slot stays active after the condition
    /// clears, until [`AlertManager::ack`] or [`AlertManager::reset`].
    pub latched: bool,
    pub raised_at: TsMillis,
}

/// Owns alert slots; turns detector edges into deduplicated, suppressed,
/// latched [`AnomalyEvent`]s.
#[derive(Debug, Default)]
pub struct AlertManager {
    slots: Vec<AlertSlot>,
}

impl AlertManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-register a slot for a configured alert.
    pub fn register(&mut self, meta: &AlertMeta) {
        self.slots.push(AlertSlot {
            id: meta.id,
            severity: meta.severity,
            category: meta.category,
            active: false,
            latched: false,
            raised_at: 0,
        });
    }

    fn slot_mut(&mut self, id: &str) -> Option<&mut AlertSlot> {
        self.slots.iter_mut().find(|s| s.id == id)
    }

    fn suppressor_active(&self) -> bool {
        self.slots.iter().any(|s| s.active && s.id == SUPPRESSOR_ID)
    }

    /// Process a detector edge; returns the event to publish, if any.
    pub fn on_edge(
        &mut self,
        meta: &AlertMeta,
        edge: Edge,
        ts: TsMillis,
        state: &VehicleState,
    ) -> Option<AnomalyEvent> {
        // While the signal feed is unreliable, derived alerts are too.
        if meta.category == Category::StateBased
            && meta.id != SUPPRESSOR_ID
            && self.suppressor_active()
            && edge == Edge::Raised
        {
            return None;
        }
        let slot = self.slot_mut(meta.id)?;
        match edge {
            Edge::Raised => {
                if slot.active {
                    // Deduplicate re-raises (e.g. a further DTC increment).
                    return None;
                }
                slot.active = true;
                slot.latched = meta.severity == Severity::Critical;
                slot.raised_at = ts;
            }
            Edge::Cleared => {
                if !slot.active {
                    return None;
                }
                if slot.latched {
                    // Condition cleared but the alert stays until ack/reset.
                    return None;
                }
                slot.active = false;
            }
        }
        Some(AnomalyEvent {
            id: meta.id.into(),
            severity: meta.severity,
            category: meta.category,
            edge,
            ts_ms: ts,
            message: (meta.describe)(state),
            vss: meta.vss.into(),
            value: (meta.value)(state),
        })
    }

    /// Merge an `Event` message produced elsewhere (e.g. by the bike while the
    /// shop runs its own engine). Idempotent per alert id: if the local slot
    /// already matches the reported state, the event is deduplicated.
    pub fn apply_external(&mut self, msg: &Message) -> Option<AnomalyEvent> {
        if msg.msg != "Event" {
            return None;
        }
        let id = msg.event.as_deref()?;
        let data = msg.data.as_ref();
        let edge = match data.and_then(|d| d.get("state")).and_then(Value::as_str) {
            Some("cleared") => Edge::Cleared,
            _ => Edge::Raised,
        };
        let severity = data
            .and_then(|d| d.get("severity"))
            .and_then(Value::as_str)
            .and_then(Severity::from_label)
            .unwrap_or(Severity::Warning);
        let message = data
            .and_then(|d| d.get("message"))
            .and_then(Value::as_str)
            .unwrap_or(id)
            .to_string();
        let ts_ms = super::parse_ts_millis(&msg.ts).unwrap_or_default();

        if let Some(slot) = self.slot_mut(id) {
            match edge {
                Edge::Raised if slot.active => return None,
                Edge::Cleared if !slot.active => return None,
                Edge::Raised => {
                    slot.active = true;
                    slot.latched = slot.severity == Severity::Critical;
                    slot.raised_at = ts_ms;
                }
                Edge::Cleared => {
                    if slot.latched {
                        return None;
                    }
                    slot.active = false;
                }
            }
        }
        // Unknown ids (newer bike firmware) are surfaced but not tracked.
        Some(AnomalyEvent {
            id: id.into(),
            severity,
            category: Category::StateBased,
            edge,
            ts_ms,
            message,
            vss: msg.vss.clone().unwrap_or_default(),
            value: msg.value.clone().unwrap_or(Value::Null),
        })
    }

    /// Highest-severity active alert (the single rider-facing surface).
    pub fn worst_active(&self) -> Option<(&'static str, Severity)> {
        self.slots
            .iter()
            .filter(|s| s.active)
            .max_by_key(|s| s.severity)
            .map(|s| (s.id, s.severity))
    }

    /// All currently active alerts.
    pub fn active(&self) -> impl Iterator<Item = &AlertSlot> {
        self.slots.iter().filter(|s| s.active)
    }

    /// Acknowledge a latched alert, deactivating it.
    pub fn ack(&mut self, id: &str) {
        if let Some(slot) = self.slot_mut(id) {
            slot.active = false;
            slot.latched = false;
        }
    }

    /// Deactivate everything (new session).
    pub fn reset(&mut self) {
        for slot in &mut self.slots {
            slot.active = false;
            slot.latched = false;
        }
    }
}
