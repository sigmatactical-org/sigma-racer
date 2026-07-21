//! Sigma Racer EFI — engine-control firmware for the microRusEFI board.
//!
//! The crate is organized by **domain**; each domain is a folder, and each
//! primary object lives in a file named after it. `no_std` and
//! engine-agnostic at the core; select an engine at compile time with a
//! Cargo feature (`engine-yamaha-cp3`).
//!
//! | Domain | What it owns |
//! |---|---|
//! | [`engine`] | configuration, profiles, live runtime state |
//! | [`trigger`] | wheel geometry, sync decoder, edge statistics |
//! | [`fuel`] | interpolation tables, injector model, speed-density |
//! | [`ignition`] | coil dwell control |
//! | [`throttle`] | ride-by-wire independent safety monitor |
//! | [`scheduler`] | angle-domain predict-and-arm |
//! | [`sensor`] | ADC channels, scaling, MRE front-end, sweep frame |
//! | [`board`] | pin map, outputs, wiring, TLE8888, Embassy split |
//! | [`comms`] | ECU side of the M7 CAN contract |
//! | [`replay`] | crank/cam signal generator (bench Phase 3) |

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]

pub mod board;
pub mod comms;
pub mod engine;
pub mod fuel;
pub mod ignition;
pub mod replay;
pub mod scheduler;
pub mod sensor;
pub mod throttle;
pub mod trigger;

// Curated top-level re-exports for the firmware entry point and consumers.
pub use board::{BoardPins, FIRMWARE_ID, TARGET_MCU, TleOutput};
pub use engine::{CYCLE_DEGREES_FOUR_STROKE, EngineConfig, EngineProfile, EngineState};

#[cfg(feature = "engine-yamaha-cp3")]
pub use engine::active_profile;
