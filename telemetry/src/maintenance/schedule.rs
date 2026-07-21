//! The prescribed maintenance schedule for a Sigma Racer model.
//!
//! The schedule is *data*, not code: it is authored and distributed by the
//! updates service (like the OTA catalog) and changes over time, so nothing
//! here hard-codes intervals. The `version` string is the identity that the
//! shop tool compares against the version the bike reports.

use serde::{Deserialize, Serialize};

/// A complete prescribed maintenance schedule for one vehicle model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaintenanceSchedule {
    /// Opaque schedule identity (e.g. `"2026.1"`). This is the value the bike
    /// reports and the shop tool compares; equal strings mean equal schedules.
    pub version: String,
    /// Vehicle model the schedule applies to (e.g. `"sigma-racer"`).
    pub model: String,
    /// RFC 3339 timestamp the schedule was published by the updates service.
    #[serde(default)]
    pub published: String,
    /// The service items, each with the interval at which it comes due.
    pub items: Vec<MaintenanceItem>,
}

impl MaintenanceSchedule {
    /// Look up a service item by its stable id.
    pub fn item(&self, id: &str) -> Option<&MaintenanceItem> {
        self.items.iter().find(|i| i.id == id)
    }
}

/// One prescribed service item (e.g. engine-oil change, valve-clearance check).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaintenanceItem {
    /// Stable identifier used to correlate against maintenance-log entries.
    pub id: String,
    /// Human-readable name shown in reports (e.g. `"Engine oil & filter"`).
    pub name: String,
    /// When the item comes due. Any combination of bounds may be set; the item
    /// is due at whichever bound is reached first.
    pub interval: MaintenanceInterval,
}

/// The recurrence of a service item. Every bound is optional; an item with no
/// bounds set is informational only (never comes "due").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MaintenanceInterval {
    /// Distance between services, in kilometres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every_km: Option<f64>,
    /// Calendar time between services, in days.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every_days: Option<u32>,
    /// Engine run-time between services, in hours.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub every_engine_hours: Option<f64>,
}

impl MaintenanceInterval {
    /// True when no recurrence bound is set (informational-only item).
    pub fn is_informational(&self) -> bool {
        self.every_km.is_none() && self.every_days.is_none() && self.every_engine_hours.is_none()
    }
}
