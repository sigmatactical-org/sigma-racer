//! Brown-out reset: the option-byte level (host-testable) and the firmware
//! routine that programs it.
//!
//! Register map and `OPTCR` layout: ST RM0410 (STM32F76xxx/F77xxx) §2.2.2 /
//! §3.9.8; FLASH controller @ `0x40023C00`.

/// Brown-out reset level programmed into STM32F7 option bytes.
///
/// `Level3` ≈ 2.1 V — appropriate for a 3.3 V ECU rail during cranking sags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorLevel {
    Off = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
}

impl BorLevel {
    /// Production BOR target (2.7 V) so DDR survives brown-outs.
    pub const TARGET: Self = Self::Level3;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_bor_is_not_off() {
        assert_ne!(BorLevel::TARGET, BorLevel::Off);
    }
}

/// Program `BorLevel::TARGET` if the device is still below target.
#[cfg(feature = "firmware")]
pub use firmware::ensure;

#[cfg(feature = "firmware")]
mod firmware {
    use super::BorLevel;
    use defmt::{info, warn};
    use stm32_metapac::FLASH;

    const OPTKEY1: u32 = 0x0819_2A3B;
    const OPTKEY2: u32 = 0x4C5D_6E7F;

    /// `OPTCR.BOR_LEV` field mask (bits 2:3).
    const OPTCR_BOR_LEV_MASK: u32 = 0x03 << 2;
    /// `OPTCR.OPTSTRT` (bit 1).
    const OPTCR_OPTSTRT: u32 = 1 << 1;

    fn current_bor_level() -> BorLevel {
        match FLASH.optcr().read().bor_lev() {
            0 => BorLevel::Off,
            1 => BorLevel::Level1,
            2 => BorLevel::Level2,
            _ => BorLevel::Level3,
        }
    }

    fn wait_not_busy() {
        while FLASH.sr().read().bsy() {}
    }

    fn unlock_option_bytes() {
        if FLASH.optcr().read().optlock() {
            FLASH.optkeyr().write_value(OPTKEY1);
            FLASH.optkeyr().write_value(OPTKEY2);
        }
    }

    fn lock_option_bytes() {
        FLASH.optcr().modify(|w| w.set_optlock(true));
    }

    /// Masked `OPTCR` write — same contract as Zephyr `flash_stm32_option_bytes_write()`.
    fn option_bytes_write(mask: u32, value: u32) {
        if FLASH.optcr().read().optlock() {
            warn!("OPTCR locked — cannot program option bytes");
            return;
        }

        let current = FLASH.optcr().read().0;
        if (current & mask) == value {
            return;
        }

        wait_not_busy();

        let merged = (current & !mask) | value;
        FLASH.optcr().modify(|w| {
            w.0 = merged;
        });
        FLASH.optcr().modify(|w| {
            w.0 = merged | OPTCR_OPTSTRT;
        });

        wait_not_busy();
    }

    /// Program `BorLevel::TARGET` if the device is still below target.
    pub fn ensure() {
        let current = current_bor_level();
        if current as u8 >= BorLevel::TARGET as u8 {
            info!(
                "BOR already at level {=u8} (target {=u8})",
                current as u8,
                BorLevel::TARGET as u8
            );
            return;
        }

        info!(
            "programming BOR level {=u8} (was {=u8}) — expect option-byte reload reset",
            BorLevel::TARGET as u8,
            current as u8
        );

        wait_not_busy();
        unlock_option_bytes();

        let bor_value = (BorLevel::TARGET as u32) << 2;
        option_bytes_write(OPTCR_BOR_LEV_MASK, bor_value);

        lock_option_bytes();

        if current_bor_level() as u8 >= BorLevel::TARGET as u8 {
            info!("BOR programmed successfully");
        } else {
            warn!("BOR program finished but level unchanged — check option bytes manually");
        }
    }
}
