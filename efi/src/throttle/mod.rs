//! Throttle (ride-by-wire) domain — the safety-critical subsystem
//! (`efi.md` §7).
//!
//! - [`config`] — [`RbwConfig`], [`RbwInputs`], [`SensorCal`] calibration
//! - [`monitor`] — the independent [`RbwMonitor`] and its fault matrix

pub mod config;
pub mod monitor;

pub use config::{PLACEHOLDER_CAL, RbwConfig, RbwInputs, SensorCal};
pub use monitor::{RbwCommand, RbwMonitor, RbwState, Side, TripCause};
