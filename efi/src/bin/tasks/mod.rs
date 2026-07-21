//! Background tasks (subsystem boundaries mirror rusEFI's).
//!
//! Implemented (stage 1 — characterization data logger):
//! - `sensors` — ADC sweep of the analog channels, `DL,S` records
//! - `trigger` — crank/cam edge capture + interval stats, `DL,T` records
//! - `safety` — IWDG + TLE8888 window watchdog supervision
//!
//! Planned:
//! - trigger decoding proper (tooth scheduler, sync state machine)
//! - `fuel` — injection scheduling via TLE8888
//! - `ignition` — coil charging and fire timing
//! - `can` — rusEFI-compatible protocol (future)
//! - `etb` — electronic throttle via TLE9201

pub mod safety;
pub mod sensors;
pub mod trigger;
