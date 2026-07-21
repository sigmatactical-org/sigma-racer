//! Ignition domain: coil dwell control (`efi.md` §6).
//!
//! - [`dwell`] — [`DwellModel`], dwell vs battery voltage with over-dwell
//!   protection

pub mod dwell;

pub use dwell::{DwellModel, PLACEHOLDER_DWELL};
