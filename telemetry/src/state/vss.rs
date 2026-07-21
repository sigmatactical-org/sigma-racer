//! VSS path mapping for [`VehicleState`](super::vehicle_state::VehicleState).
//!
//! One [`VssBinding`] is the single source of truth for which state field maps
//! to which VSS path and how its value converts in both directions. The
//! `to_vss_map` / `apply_vss` / `apply_vss_map` methods delegate to it, so the
//! encode and decode sides can no longer drift apart (the previous hand-listed
//! tables had to be kept in sync by hand, and JSON coercion lived in a separate
//! `coerce` module — both now subsumed by vss-map).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value;
use vss_map::{VssBinding, VssValue};

use super::vehicle_state::VehicleState;

static BINDING: OnceLock<VssBinding<VehicleState>> = OnceLock::new();

/// The shared state↔VSS binding, built once.
pub(crate) fn binding() -> &'static VssBinding<VehicleState> {
    BINDING.get_or_init(build_binding)
}

fn build_binding() -> VssBinding<VehicleState> {
    let mut b = VssBinding::new();
    b.bind(
        "Vehicle.Speed",
        |s: &VehicleState| VssValue::Int(s.speed.round() as i64),
        |s, v| s.speed = v.as_f32(),
    )
    .bind(
        "Vehicle.Powertrain.CombustionEngine.Speed",
        |s: &VehicleState| VssValue::Int(s.rpm.round() as i64),
        |s, v| s.rpm = v.as_f32(),
    )
    .bind(
        "Vehicle.Powertrain.Transmission.CurrentGear",
        |s: &VehicleState| VssValue::Int(s.gear as i64),
        |s, v| s.gear = v.as_i8(),
    )
    .bind(
        "Vehicle.Powertrain.CombustionEngine.IsRedline",
        |s: &VehicleState| VssValue::Bool(s.at_redline),
        // Derived from rpm + redline_can in refresh_derived; not restorable.
        |_s, _v| {},
    )
    .bind(
        "Vehicle.Powertrain.CombustionEngine.ThrottlePosition",
        |s: &VehicleState| VssValue::Float(s.throttle_pct as f64),
        |s, v| s.throttle_pct = v.as_f32(),
    )
    .bind(
        "Vehicle.Body.IsSideStandEngaged",
        |s: &VehicleState| VssValue::Bool(s.side_stand),
        |s, v| s.side_stand = v.as_bool(),
    )
    .bind(
        "Vehicle.Powertrain.Transmission.PerformanceMode",
        |s: &VehicleState| VssValue::Text(s.riding_mode.clone()),
        |s, v| {
            if let Some(label) = v.as_str() {
                s.riding_mode = label.to_owned();
            }
        },
    )
    .bind(
        "Vehicle.FuelSystem.Level",
        |s: &VehicleState| VssValue::Float(s.fuel_pct as f64),
        |s, v| s.fuel_pct = v.as_f32(),
    )
    .bind(
        "Vehicle.OBD.CoolantTemperature",
        |s: &VehicleState| VssValue::Int(s.coolant_c as i64),
        |s, v| s.coolant_c = v.as_i16(),
    )
    .bind(
        "Vehicle.OBD.OilTemperature",
        |s: &VehicleState| VssValue::Int(s.oil_c as i64),
        |s, v| s.oil_c = v.as_i16(),
    )
    .bind(
        "Vehicle.TraveledDistance",
        |s: &VehicleState| VssValue::Float(s.odometer as f64),
        |s, v| s.odometer = v.as_f32(),
    )
    .bind(
        "Vehicle.TripMeter1",
        |s: &VehicleState| VssValue::Float(s.trip1 as f64),
        |s, v| s.trip1 = v.as_f32(),
    )
    .bind(
        "Vehicle.TripMeter2",
        |s: &VehicleState| VssValue::Float(s.trip2 as f64),
        |s, v| s.trip2 = v.as_f32(),
    )
    .bind(
        "Vehicle.Acceleration.Lateral",
        |s: &VehicleState| VssValue::Float(s.lean_angle as f64),
        |s, v| s.lean_angle = v.as_f32(),
    )
    .bind(
        "Vehicle.Acceleration.Longitudinal",
        |s: &VehicleState| VssValue::Float(s.gforce as f64),
        |s, v| s.gforce = v.as_f32(),
    )
    .bind(
        "Vehicle.ElectricalSystem.Battery.Voltage",
        |s: &VehicleState| VssValue::Float(s.battery_v as f64),
        |s, v| s.battery_v = v.as_f32(),
    )
    .bind(
        "Vehicle.Cabin.Infotainment.CanBusLoad",
        |s: &VehicleState| VssValue::Uint(s.can_load as u64),
        |s, v| s.can_load = v.as_u8(),
    )
    .bind(
        "Vehicle.OBD.DTCCount",
        |s: &VehicleState| VssValue::Uint(s.dtc as u64),
        |s, v| s.dtc = v.as_u8(),
    )
    .bind(
        "Vehicle.ADAS.ABS.IsActive",
        |s: &VehicleState| VssValue::Bool(s.abs_active),
        |s, v| s.abs_active = v.as_bool(),
    )
    .bind(
        "Vehicle.ADAS.TCS.IsActive",
        |s: &VehicleState| VssValue::Bool(s.tc_active),
        |s, v| s.tc_active = v.as_bool(),
    )
    .bind(
        "Vehicle.CurrentLocation.Heading",
        |s: &VehicleState| VssValue::Int(s.heading.round() as i64),
        |s, v| s.heading = v.as_f32(),
    )
    .bind(
        "Vehicle.CurrentLocation.Altitude",
        |s: &VehicleState| VssValue::Int(s.elevation as i64),
        |s, v| s.elevation = v.as_i32(),
    );
    // Vehicle.Service.SignalsLive is intentionally NOT bound: per-signal
    // freshness now travels in the message `avail` map (see
    // crate::availability), not as a global VSS boolean. `signals_live`
    // survives only as an internal input to the staleness detector.
    b
}

impl VehicleState {
    /// Render the full state as VSS path → JSON value entries.
    pub fn to_vss_map(&self) -> HashMap<String, Value> {
        binding().to_json_map(self)
    }

    /// Apply one VSS entry; unknown paths are ignored.
    pub fn apply_vss(&mut self, path: &str, value: &Value) {
        binding().apply_json(self, path, value);
    }

    /// Apply a batch of VSS entries and refresh derived fields.
    pub fn apply_vss_map(&mut self, data: &HashMap<String, Value>) {
        binding().apply_json_map(self, data);
        self.refresh_derived();
    }

    /// Update the internal `signals_live` rollup from a message's sparse
    /// availability map (the per-signal replacement for the retired
    /// `Vehicle.Service.SignalsLive` wire value). Consumers call this alongside
    /// [`apply_vss_map`](Self::apply_vss_map).
    pub fn apply_availability(&mut self, avail: Option<&HashMap<String, String>>) {
        self.signals_live = crate::availability::signals_live_from_avail(avail);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vss_map::VssCatalog;

    #[test]
    fn every_bound_path_exists_in_the_catalog() {
        let catalog =
            VssCatalog::from_vspec_str(include_str!("../../tests/data/sigma-cluster.vspec"))
                .expect("vspec must parse");
        let missing = binding().validate_against(&catalog);
        assert!(
            missing.is_empty(),
            "state binds VSS paths absent from the catalog: {missing:?}"
        );
    }

    #[test]
    fn state_round_trips_through_the_binding() {
        let mut state = VehicleState::idle();
        state.speed = 88.0;
        state.gear = 4;
        state.riding_mode = "TRACK".into();
        state.coolant_c = 91;

        let map = state.to_vss_map();
        let mut restored = VehicleState::idle();
        restored.apply_vss_map(&map);

        assert_eq!(restored.speed, 88.0);
        assert_eq!(restored.gear, 4);
        assert_eq!(restored.riding_mode, "TRACK");
        assert_eq!(restored.coolant_c, 91);
    }
}
