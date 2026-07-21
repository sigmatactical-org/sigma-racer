//! Volatile MMIO helpers for the i.MX8M Plus Cortex-M7.

use core::ptr::{read_volatile, write_volatile};

/// Physical base of the IOMUXC block.
pub const IOMUXC_BASE: usize = 0x3033_0000;
/// Physical base of the CCM (clock controller).
pub const CCM_BASE: usize = 0x3038_0000;
/// Physical base of LPUART4.
pub const LPUART4_BASE: usize = 0x30A6_0000;
/// Physical base of FLEXCAN1 (M7 safety bus).
pub const FLEXCAN1_BASE: usize = 0x308C_0000;
/// Physical base of MU1 (A53 ↔ M7 mailbox).
pub const MU1_BASE: usize = 0x30AA_0000;
/// Resource Domain Controller.
pub const RDC_BASE: usize = 0x303A_0000;

#[inline(always)]
pub(crate) fn read(addr: usize) -> u32 {
    unsafe { read_volatile(addr as *const u32) }
}

#[inline(always)]
pub(crate) fn write(addr: usize, value: u32) {
    unsafe { write_volatile(addr as *mut u32, value) }
}

#[inline(always)]
pub(crate) fn modify(addr: usize, mask: u32, value: u32) {
    let current = read(addr);
    write(addr, (current & !mask) | (value & mask));
}
