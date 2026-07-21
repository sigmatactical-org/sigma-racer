//! i.MX8M Plus Cortex-M7 hardware bring-up.

mod clock;
mod flexcan;
mod mpu;
mod mu;
mod reg;
mod rpmsg;
mod rsc_table;
mod uart;

pub use clock::{CAN_PE_CLK_HZ, CORE_CLK_HZ};
pub use flexcan::{FlexCan1, RxFrame};
pub use rpmsg::{ENDPOINT_NAME, RpmsgTx};
pub use uart::Uart4;

/// Early platform init: safe MMIO, peripheral clock gates.
pub fn init() {
    unsafe { mpu::init() };
    let _ = clock::init_uart4();
    clock::init_can1();
    rdc_allow_m7_peripherals();
}

/// Grant the M7 domain access to CAN1 and IOMUXC (minimal RDC bring-up).
fn rdc_allow_m7_peripherals() {
    const RDC_PDAP_CAN1: usize = reg::RDC_BASE + 0x284;
    const RDC_PDAP_GPIO1: usize = reg::RDC_BASE + 0x200;
    for pdap in [RDC_PDAP_CAN1, RDC_PDAP_GPIO1] {
        // Set domain 1 (M7) as owner with full access (bits 23:16 = 0xFF).
        reg::modify(pdap, 0x00FF_0000, 0x00FF_0000);
    }
}
