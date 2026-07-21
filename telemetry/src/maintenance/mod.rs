//! Maintenance domain types shared by the bike (source of truth for the log and
//! its schedule version) and the shop tool (which fetches the prescribed
//! schedule from the updates service and audits the bike against it).
//!
//! Pure data only — the audit logic lives in the shop tool.

mod log;
mod report;
mod schedule;

pub use log::MaintenanceLogEntry;
pub use report::MaintenanceReport;
pub use schedule::{MaintenanceInterval, MaintenanceItem, MaintenanceSchedule};
