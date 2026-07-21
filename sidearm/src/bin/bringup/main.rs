//! Minimal M7 boot proof — prints over LPUART4 on the Verdin iMX8M Plus.
//!
//! Build:
//! ```text
//! cargo build --no-default-features --features bringup
//! ```
//!
//! Load from U-Boot (DDR layout, default):
//! ```text
//! ext4load mmc 2:2 ${loadaddr} /sigma-racer-sidearm-bringup.bin
//! cp.b ${loadaddr} 0x80000000 ${filesize}
//! bootaux 0x80000000
//! ```
//!
//! See `docs/M7_BRINGUP.md` for remoteproc, ITCM, and HMP notes.

#![no_std]
#![no_main]
#![deny(unsafe_code)]

use panic_halt as _;
use sigma_racer_sidearm::hw::Uart4;

// cortex-m-rt requires `pre_init` to be an unsafe fn; hw::init is the
// board bring-up documented in `hw`.
#[allow(unsafe_code)]
#[cortex_m_rt::pre_init]
unsafe fn pre_init() {
    sigma_racer_sidearm::hw::init();
}

#[cortex_m_rt::entry]
fn main() -> ! {
    let uart = Uart4::new();
    uart.write_str("sigma-racer-sidearm bringup\r\n");
    uart.write_str("M7 alive on LPUART4 (115200 8N1)\r\n");

    let mut n: u32 = 0;
    loop {
        uart.write_str("tick ");
        write_u32(&uart, n);
        uart.write_str("\r\n");
        n = n.wrapping_add(1);
        delay();
    }
}

fn write_u32(uart: &Uart4, mut value: u32) {
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    if value == 0 {
        uart.write_byte(b'0');
        return;
    }
    while value > 0 {
        i -= 1;
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
    }
    uart.write(&buf[i..]);
}

fn delay() {
    for _ in 0..8_000_000 {
        cortex_m::asm::nop();
    }
}
