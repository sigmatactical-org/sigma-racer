//! TLE8888 SPI command encoding — ported from rusEFI `tle8888.cpp`.
//!
//! MCU SPI peripheral: SPI1 @ `0x40013000` (Zephyr `dts/arm/st/f7/stm32f7.dtsi`,
//! `spi1@40013000`; `stm32_metapac::SPI1`). Board wiring: SCK `PB3`, MOSI `PB5`,
//! MISO `PB4`, CS `PD5` — see [`crate::board::MreBoard::init`].
//!
//! IWDG backstop @ `0x40003000` (Zephyr `iwdg@40003000`) is serviced in
//! `src/bin/tasks/safety.rs`; the TLE8888 window watchdog uses [`CMD_WWDSERVICECMD`].
//!
//! Pure `no_std` logic; the Embassy driver in [`crate::board::tle8888::bus`]
//! performs the actual transfers.

/// SPI write bit.
pub const CMD_WRITE: u16 = 1 << 0;
/// SPI read bit (write bit clear).
pub const CMD_READ: u16 = 0;

/// Build a 16-bit write command: register address + 8-bit payload.
pub const fn cmd_w(reg: u8, data: u8) -> u16 {
    CMD_WRITE | ((reg as u16 & 0x7f) << 1) | ((data as u16) << 8)
}

/// Build a 16-bit read command.
pub const fn cmd_r(reg: u8) -> u16 {
    CMD_READ | ((reg as u16 & 0x7f) << 1)
}

/// Read the status register.
pub const CMD_SR: u16 = cmd_w(0x1a, 0x03);
/// Unlock configuration writes.
pub const CMD_CHIP_UNLOCK: u16 = cmd_w(0x1e, 0x01);
/// Assert the output-enable flag.
pub const CMD_OE_SET: u16 = cmd_w(0x1c, 0x02);
/// Clear the output-enable flag.
pub const CMD_OE_CLR: u16 = cmd_w(0x1c, 0x01);
/// Service the window watchdog.
pub const CMD_WWDSERVICECMD: u16 = cmd_w(0x15, 0x03);

/// Configure input mapping register `n`.
pub const fn cmd_inconfig(n: u8, data: u8) -> u16 {
    cmd_w(0x53 + (n & 0x03), data)
}

/// Configure output mode register `n`.
pub const fn cmd_outconfig(n: u8, data: u8) -> u16 {
    cmd_w(0x40 + n, data)
}

/// Configure output-enable register `n`.
pub const fn cmd_oeconfig(n: u8, data: u8) -> u16 {
    cmd_w(0x58 + n, data)
}

/// Configure direct-drive register `n`.
pub const fn cmd_ddconfig(n: u8, data: u8) -> u16 {
    cmd_w(0x5c + n, data)
}

/// Configure VR-sensor register `n`.
pub const fn cmd_vrsconfig(n: u8, data: u8) -> u16 {
    cmd_w(0x49 + (n & 0x03), data)
}

/// Default INCONFIG value: maps aux inputs to a non-existent output so SPI
/// retains sole control (rusEFI `InConfig[i] = 25 - 1 - 4`).
pub const DEFAULT_INCONFIG: u8 = 20;

/// Window-watchdog service period (rusEFI `WWD_PERIOD_MS`).
pub const WWD_PERIOD_MS: u64 = 101;

/// IWDG timeout target for the MCU backstop watchdog.
pub const IWDG_TIMEOUT_US: u32 = 2_000_000;

/// Safe bring-up sequence: configure the chip but leave outputs disabled
/// (`CMD_OE_CLR`) and do **not** assert INJ_EN / IGN_EN (caller responsibility).
pub const INIT_SAFE: &[u16] = &[
    CMD_CHIP_UNLOCK,
    cmd_inconfig(0, DEFAULT_INCONFIG),
    cmd_inconfig(1, DEFAULT_INCONFIG),
    cmd_inconfig(2, DEFAULT_INCONFIG),
    cmd_inconfig(3, DEFAULT_INCONFIG),
    // Diagnostic settings — match rusEFI `chip_init()` OUTCONFIG values.
    cmd_outconfig(0, (1 << 7) | (1 << 5) | (1 << 3) | (1 << 1)),
    cmd_outconfig(1, (1 << 5) | (1 << 3) | (1 << 1)),
    cmd_outconfig(2, 0),
    cmd_outconfig(3, 1 << 5),
    cmd_outconfig(4, (1 << 5) | (1 << 3) | (1 << 1)),
    cmd_outconfig(5, (1 << 5) | (1 << 3) | (1 << 1)),
    cmd_oeconfig(0, 0),
    cmd_ddconfig(0, 0),
    cmd_oeconfig(1, 0),
    cmd_ddconfig(1, 0),
    cmd_oeconfig(2, 0),
    cmd_ddconfig(2, 0),
    cmd_oeconfig(3, 0),
    cmd_ddconfig(3, 0),
    cmd_vrsconfig(1, 0),
    CMD_OE_CLR,
];

/// Extract the register address from a command halfword.
pub const fn register_from_cmd(tx: u16) -> u8 {
    ((tx >> 1) & 0x7f) as u8
}

/// Extract data byte from a write command.
pub const fn data_from_cmd(tx: u16) -> u8 {
    (tx >> 8) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_command_encoding_matches_rusefi_macros() {
        assert_eq!(CMD_WWDSERVICECMD, cmd_w(0x15, 0x03));
        assert_eq!(CMD_CHIP_UNLOCK, cmd_w(0x1e, 0x01));
        assert_eq!(CMD_OE_CLR, cmd_w(0x1c, 0x01));
    }

    #[test]
    fn read_command_has_write_bit_clear() {
        let read_status = cmd_r(0x1a);
        assert_eq!(read_status & CMD_WRITE, 0);
        assert_eq!(register_from_cmd(read_status), 0x1a);
    }

    #[test]
    fn init_safe_ends_with_output_enable_clear() {
        assert_eq!(INIT_SAFE.last(), Some(&CMD_OE_CLR));
    }
}
