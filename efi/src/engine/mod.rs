//! Engine domain: configuration, profiles, and live runtime state.
//!
//! - [`config`] — the tunable [`EngineConfig`] and its validation vocabulary
//! - [`firing`] — firing-order presets
//! - [`profile`] — a complete [`EngineProfile`] (factory specs + trigger)
//! - [`state`] — live [`EngineState`] updated by the running tasks
//! - [`yamaha_cp3`] — the Yamaha CP3 profile (this build's engine)

pub mod config;
pub mod firing;
pub mod profile;
pub mod state;
pub mod yamaha_cp3;

pub use config::{ConfigError, EngineConfig, IgnitionMode, InjectionMode, MAX_CYLINDERS};
pub use profile::{CYCLE_DEGREES_FOUR_STROKE, EngineProfile, ProfileError};
pub use state::EngineState;

/// Engine profile selected at compile time via Cargo features.
#[cfg(feature = "engine-yamaha-cp3")]
pub fn active_profile() -> EngineProfile {
    yamaha_cp3::profile()
}

#[cfg(all(feature = "firmware", not(feature = "engine-yamaha-cp3")))]
compile_error!("firmware builds require an engine feature: engine-yamaha-cp3");
