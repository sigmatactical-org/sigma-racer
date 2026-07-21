//! MPU and cache setup for the M7.
//!
//! Peripheral space must not be accessed through the D-cache. Until a full NXP
//! `BOARD_InitMemory()` port lands, I/D-cache are disabled so MMIO to UART/CCM
//! is safe. Re-enable with proper MPU regions once the layout is validated on
//! silicon.

/// Disable caches so MMIO is strongly ordered (safe bring-up default).
pub unsafe fn init() {
    let mut cp = unsafe { cortex_m::Peripherals::steal() };
    cp.SCB.disable_icache();
    cp.SCB.disable_dcache(&mut cp.CPUID);
    cortex_m::asm::dmb();
}
