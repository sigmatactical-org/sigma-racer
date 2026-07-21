//! Board domain: microRusEFI pin map, outputs, wiring validation, brown-out,
//! the TLE8888 driver, and the Embassy peripheral split.
//!
//! - [`identity`] — firmware/MCU identity strings
//! - [`pins`] — [`BoardPins`] / [`GpioPin`] / [`GpioPort`] map
//! - [`output`] — [`TleOutput`] roles
//! - [`wiring`] — cylinder→output mapping + profile validation
//! - [`bor`] — brown-out reset level + programming
//! - [`tle8888`] — SPI command encoding + bus driver
//! - [`mre_board`] — the [`MreBoard`] Embassy peripheral split (firmware)
//! - [`heap`] — boot allocator (firmware)

pub mod bor;
pub mod identity;
pub mod output;
pub mod pins;
pub mod tle8888;
pub mod wiring;

#[cfg(feature = "firmware")]
pub mod heap;
#[cfg(feature = "firmware")]
pub mod mre_board;

pub use bor::BorLevel;
pub use identity::{FIRMWARE_ID, TARGET_MCU};
pub use output::TleOutput;
pub use pins::{BoardPins, GpioPin, GpioPort};

#[cfg(feature = "firmware")]
pub use mre_board::{MreBoard, SafeOutputs, SensorChannels};
