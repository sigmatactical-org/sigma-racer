//! Maintenance-log entries: the record of services actually performed on a bike.
//!
//! These live on the vehicle (the bike is the source of truth for what work has
//! been done to it) and are reported to the shop tool over the query protocol.

use serde::{Deserialize, Serialize};

/// A single completed service, correlated to a schedule item by `item_id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaintenanceLogEntry {
    /// The [`MaintenanceItem`](super::MaintenanceItem) id this service satisfies.
    pub item_id: String,
    /// RFC 3339 timestamp the service was performed.
    pub performed_at: String,
    /// Odometer reading (km) at the time of service.
    pub odometer_km: f64,
    /// Engine hours at the time of service, when the bike tracks them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_hours: Option<f64>,
    /// Free-text note (who/where/parts), optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
