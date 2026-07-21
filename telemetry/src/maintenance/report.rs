//! The maintenance payload the bike returns in answer to a query.
//!
//! It is deliberately self-contained: it carries the bike's schedule version,
//! its live odometer / engine-hours, and the full maintenance log, so the shop
//! tool can run a complete audit without a separate live telemetry subscription.

use serde::{Deserialize, Serialize};

use super::log::MaintenanceLogEntry;

/// The bike's answer to a `MaintenanceQuery`: schedule version, current usage
/// counters, and the maintenance log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaintenanceReport {
    /// Version of the maintenance schedule the bike is currently provisioned
    /// with. The shop tool compares this against the updates-service latest.
    pub schedule_version: String,
    /// Current odometer reading, in kilometres.
    pub odometer_km: f64,
    /// Current engine hours, when the bike tracks them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_hours: Option<f64>,
    /// Every service the bike has recorded.
    pub logs: Vec<MaintenanceLogEntry>,
}
