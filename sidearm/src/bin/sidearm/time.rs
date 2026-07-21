//! `embassy-time` backend for the M7.
//!
//! There is no vendor Embassy HAL for the i.MX8M Plus Cortex-M7, so nothing
//! supplies the global time driver that `embassy-time` requires. We register a
//! SysTick-backed driver here: the core's SysTick counter is extended to 64-bit
//! and drives `Timer`/`Duration` for every task.
//!
//! # Calibration
//!
//! [`SYSTICK_FREQ_HZ`] must match the actual SysTick clock on the M7. It is
//! wired to [`sigma_racer_sidearm::hw::CORE_CLK_HZ`] (800 MHz nominal); verify
//! on hardware before trusting timeouts.

use cortex_m::peripheral::SYST;
use sigma_racer_sidearm::hw::CORE_CLK_HZ;
use systick_timer::SystickDriver;

/// SysTick input clock, in Hz. Matches the nominal M7 core clock from
/// [`sigma_racer_sidearm::hw::CORE_CLK_HZ`]; verify on hardware if timeouts drift.
const SYSTICK_FREQ_HZ: u64 = CORE_CLK_HZ;

/// `embassy-time` tick rate. Kept in sync with the `tick-hz-1_000` feature on
/// `embassy-time` in `Cargo.toml` (1 ms resolution).
const TICK_HZ: u64 = 1_000;

/// SysTick reload for a `TICK_HZ` interrupt. Must fit the 24-bit SysTick range.
const RELOAD: u32 = (SYSTICK_FREQ_HZ / TICK_HZ - 1) as u32;

/// Number of concurrent timer wakers the driver can track.
const WAKERS: usize = 8;

embassy_time_driver::time_driver_impl!(
    static DRIVER: SystickDriver<WAKERS> = SystickDriver::new(SYSTICK_FREQ_HZ, RELOAD)
);

/// Start the SysTick time driver. Call once, early in boot, before any
/// `embassy-time` API is used.
pub fn init(syst: SYST) {
    let mut syst = syst;
    DRIVER.start(&mut syst);
}

/// SysTick exception — advances the 64-bit software time base and fires due
/// timers.
#[cortex_m_rt::exception]
fn SysTick() {
    DRIVER.systick_interrupt();
}
