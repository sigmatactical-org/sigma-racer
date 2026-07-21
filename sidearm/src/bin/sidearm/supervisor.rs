//! Fail-operational **supervisor**: watchdog and load-shed anchor.

use core::sync::atomic::{AtomicU32, Ordering};

use embassy_time::Instant;

/// Control-loop iterations since boot. Atomics rather than `static mut`: sound
/// even if a diagnostic reader is ever called from another context (an ISR or a
/// second task), and the standard idiom under edition 2024.
static LOOP_COUNT: AtomicU32 = AtomicU32::new(0);
/// `embassy-time` millis at the last supervisor kick.
static LAST_KICK_MS: AtomicU32 = AtomicU32::new(0);

/// Periodic supervisor service, called once per control-loop iteration (5 ms).
pub fn service() {
    LOOP_COUNT.fetch_add(1, Ordering::Relaxed);
    LAST_KICK_MS.store(Instant::now().as_millis() as u32, Ordering::Relaxed);
    // TODO: feed SoC WDOG3 once the kick register sequence is validated on silicon.
}

/// Milliseconds since the last successful supervisor kick.
pub fn millis_since_kick() -> u32 {
    let now = Instant::now().as_millis() as u32;
    now.saturating_sub(LAST_KICK_MS.load(Ordering::Relaxed))
}

/// Control-loop iterations since boot (diagnostic).
pub fn loop_count() -> u32 {
    LOOP_COUNT.load(Ordering::Relaxed)
}

/// Enter a safe, non-returning fault state.
///
/// Used only for build-time contract failures (e.g. the embedded `.dbc` fails
/// to parse), where a reset would just boot-loop. Interrupts are masked and the
/// core parks in `wfi`; once the SoC watchdog (WDOG3) is fed in [`service`], a
/// runtime fault here will additionally trip a hardware reset.
pub fn fault_loop() -> ! {
    cortex_m::interrupt::disable();
    loop {
        cortex_m::asm::wfi();
    }
}
