//! VSS diff helpers for incremental updates.

use crate::state::VehicleState;
use serde_json::Value;
use std::collections::HashMap;

/// VSS entries whose value changed between two states (delta update payload).
pub fn diff_vss(prev: &VehicleState, next: &VehicleState) -> HashMap<String, Value> {
    crate::state::vss_binding().diff_json_map(prev, next)
}
