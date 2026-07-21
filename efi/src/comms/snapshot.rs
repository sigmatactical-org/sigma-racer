//! The ECU-side snapshot published onto the M7 safety bus.

/// What the ECU knows and publishes each broadcast tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EcuSnapshot {
    pub rpm: f32,
    pub coolant_c: f32,
    /// Oil temperature — 0 until the added oil-temp sensor is wired
    /// (`engine.md` §3 extreme-cooling spec).
    pub oil_c: f32,
    pub battery_v: f32,
    /// Throttle plate position from the RbW monitor's normalized TPS pair.
    pub throttle_pct: f32,
    pub side_stand: bool,
    /// Latched fault count (RbW trips, sensor faults, decoder desyncs).
    pub dtc_count: u8,
}
