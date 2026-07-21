//! Tier-1 vehicle state keyed by VSS paths.

mod vehicle_state;
mod vss;

pub use vehicle_state::{REDLINE_RPM, VehicleState};
pub(crate) use vss::binding as vss_binding;
