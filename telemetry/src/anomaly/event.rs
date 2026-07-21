//! Anomaly event: one raised/cleared edge, ready for the wire.

use std::collections::HashMap;

use chrono::{SecondsFormat, TimeZone, Utc};
use serde_json::Value;

use crate::protocol::Message;

use super::types::{Category, Edge, Severity, TsMillis};

/// One alert transition, produced by the [`AlertManager`](super::AlertManager).
#[derive(Debug, Clone, PartialEq)]
pub struct AnomalyEvent {
    /// Stable id, e.g. `coolant_overheat` (matches the wire schema enum).
    pub id: String,
    pub severity: Severity,
    pub category: Category,
    pub edge: Edge,
    /// Timestamp of the sample that triggered the edge.
    pub ts_ms: TsMillis,
    /// Human-readable one-liner for logs and UIs.
    pub message: String,
    /// Primary VSS path the alert is about.
    pub vss: String,
    /// Triggering value.
    pub value: Value,
}

impl AnomalyEvent {
    /// Wire form: an `Event` envelope carrying the sample's timestamp (unlike
    /// the self-stamping snapshot/update constructors).
    pub fn to_message(&self, seq: u64) -> Message {
        let ts = Utc
            .timestamp_millis_opt(self.ts_ms)
            .single()
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let mut data = HashMap::new();
        data.insert(
            "state".into(),
            Value::from(match self.edge {
                Edge::Raised => "raised",
                Edge::Cleared => "cleared",
            }),
        );
        data.insert("severity".into(), Value::from(self.severity.label()));
        data.insert("message".into(), Value::from(self.message.clone()));
        Message::event(seq, ts, &self.id, &self.vss, self.value.clone(), data)
    }
}
