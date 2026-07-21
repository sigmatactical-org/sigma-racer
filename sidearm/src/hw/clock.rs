//! Minimal CCM clock gating for M7 bring-up.

use super::reg::{self, CCM_BASE};

const CCM_CCGR5: usize = CCM_BASE + 0x78;
const CCM_CCGR5_CG14_MASK: u32 = 0x0C00_0000;
const CCM_CCGR5_CG10_MASK: u32 = 0x0030_0000;
const CCM_CCGR_RUN_HW: u32 = 0x0C00_0000;
const CCM_CCGR_CAN1: u32 = 0x0030_0000;

/// Nominal M7 core clock after the SoC is in RUN mode (800 MHz max per TRM).
pub const CORE_CLK_HZ: u64 = 800_000_000;

/// FLEXCAN1 protocol engine clock (matches Linux `assigned-clock-rates = <40000000>`).
pub const CAN_PE_CLK_HZ: u32 = 40_000_000;

/// Ungate LPUART4.
pub fn init_uart4() -> u64 {
    reg::modify(CCM_CCGR5, CCM_CCGR5_CG14_MASK, CCM_CCGR_RUN_HW);
    CORE_CLK_HZ
}

/// Ungate FLEXCAN1.
pub fn init_can1() {
    reg::modify(CCM_CCGR5, CCM_CCGR5_CG10_MASK, CCM_CCGR_CAN1);
}
