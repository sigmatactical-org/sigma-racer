//! microRusEFI board identity strings.

/// Firmware identity string (rusEFI uses `microRusEFI`).
pub const FIRMWARE_ID: &str = "sigma-racer-efi-mre";

/// Target MCU — verify against your PCB silkscreen / BOM.
pub const TARGET_MCU: &str = "STM32F767VI";
