//! TLE8888 smart low-side driver: SPI command encoding and the Embassy bus.
//!
//! - [`command`] — `no_std` SPI command words and the safe-init sequence
//! - [`bus`] — the SPI1 [`Tle8888Bus`] driver (firmware)

pub mod command;

#[cfg(feature = "firmware")]
pub mod bus;

pub use command::{CMD_SR, INIT_SAFE, IWDG_TIMEOUT_US, WWD_PERIOD_MS};

#[cfg(feature = "firmware")]
pub use bus::{Tle8888Bus, init_or_log, spi_config};
