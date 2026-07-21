//! Blocking LPUART4 console (Verdin iMX8M Plus M7 debug UART).
//!
//! Pin mux matches Zephyr `uart4_default` on SODIMM 151 (RX) / 153 (TX). When
//! a Wi-Fi/BT module is fitted, Linux must not claim UART4 — apply the Toradex
//! HMP device-tree overlay and `clk-imx8mp.mcore_booted=1` before expecting
//! output after Linux boots.

use super::reg::{self, IOMUXC_BASE, LPUART4_BASE};

/// IOMUXC pad configuration value (slow slew, medium drive — NXP 0x1C4).
const PAD_CONF_UART: u32 = 0x0000_01C4;

/// LPUART4 root clock assumed after CCM gating (80 MHz — verify on scope).
const UART_CLK_HZ: u32 = 80_000_000;
const BAUD_115200: u32 = 115_200;

/// LPUART register offsets.
mod lpuart {
    /// Baud rate.
    pub const BAUD: usize = 0x10;
    /// Status.
    pub const STAT: usize = 0x14;
    /// Control.
    pub const CTRL: usize = 0x18;
    /// Data.
    pub const DATA: usize = 0x1C;

    /// TX data register empty.
    pub const STAT_TDRE: u32 = 1 << 23;
    /// Transmission complete.
    pub const STAT_TC: u32 = 1 << 22;

    /// Transmitter enable.
    pub const CTRL_TE: u32 = 1 << 19;
    /// Receiver enable.
    pub const CTRL_RE: u32 = 1 << 18;
    /// 8-bit, no parity, one stop bit (M=0, PE=0 — the reset default).
    pub const CTRL_M_8N1: u32 = 0;
}

/// Verdin UART4 on SODIMM 151/153 (UART4_DCE — MCU is DTE).
pub struct Uart4;

impl Uart4 {
    /// Configure pin mux and LPUART4 for 115200 8N1.
    pub fn new() -> Self {
        pinmux_uart4();
        let base = LPUART4_BASE;

        reg::write(base + lpuart::CTRL, 0);

        let sbr = UART_CLK_HZ / (16 * BAUD_115200);
        reg::write(base + lpuart::BAUD, ((16 - 1) << 24) | (sbr & 0x1FFF));

        reg::write(
            base + lpuart::CTRL,
            lpuart::CTRL_TE | lpuart::CTRL_RE | lpuart::CTRL_M_8N1,
        );

        Self
    }

    /// Block until the transmit holding register is empty, then send one byte.
    pub fn write_byte(&self, byte: u8) {
        let base = LPUART4_BASE;
        while reg::read(base + lpuart::STAT) & lpuart::STAT_TDRE == 0 {}
        reg::write(base + lpuart::DATA, byte as u32);
        while reg::read(base + lpuart::STAT) & lpuart::STAT_TC == 0 {}
    }

    /// Transmit a byte slice.
    pub fn write(&self, bytes: &[u8]) {
        for &b in bytes {
            self.write_byte(b);
        }
    }

    /// Transmit a UTF-8 string.
    pub fn write_str(&self, s: &str) {
        self.write(s.as_bytes());
    }
}

fn pinmux_uart4() {
    mux_pad(0x238, 0x498, 0x600, 0, 0x8, PAD_CONF_UART | (1 << 16));
    mux_pad(0x23C, 0x49C, 0, 0, 0, PAD_CONF_UART);
}

fn mux_pad(
    mux_off: u32,
    conf_off: u32,
    daisy_off: u32,
    mux_mode: u32,
    daisy_val: u32,
    pad_conf: u32,
) {
    reg::write(IOMUXC_BASE + mux_off as usize, mux_mode);
    reg::write(IOMUXC_BASE + conf_off as usize, pad_conf);
    if daisy_off != 0 {
        reg::write(IOMUXC_BASE + daisy_off as usize, daisy_val);
    }
}
