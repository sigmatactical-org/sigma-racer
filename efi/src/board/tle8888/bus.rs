//! TLE8888 SPI driver for microRusEFI.
//!
//! Uses SPI1 (`0x40013000`, Zephyr `stm32f7.dtsi` / RM0410 §2.2.2) on `PB3`/`PB5`/`PB4`
//! with active-low CS on `PD5`.

use super::command::{self as tle8888, CMD_SR, INIT_SAFE};
use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Blocking;
use embassy_stm32::spi::mode::Master;
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::time::Hertz;

/// SPI1 bus to the onboard TLE8888.
pub struct Tle8888Bus {
    spi: Spi<'static, Blocking, Master>,
    cs: Output<'static>,
}

impl Tle8888Bus {
    /// Wrap the SPI bus + chip-select and run the unlock sequence.
    pub fn new(spi: Spi<'static, Blocking, Master>, mut cs: Output<'static>) -> Self {
        cs.set_high();
        Self { spi, cs }
    }

    fn transfer(&mut self, tx: u16) -> Result<u16, embassy_stm32::spi::Error> {
        self.cs.set_low();
        let mut buf = [tx];
        let res = self.spi.blocking_transfer_in_place(&mut buf);
        self.cs.set_high();
        res?;
        Ok(buf[0])
    }

    fn transfer_sequence(&mut self, cmds: &[u16]) -> Result<(), embassy_stm32::spi::Error> {
        for &cmd in cmds {
            self.transfer(cmd)?;
        }
        Ok(())
    }

    /// Soft-reset the TLE8888 and load the safe output-disabled configuration.
    pub fn init_safe(&mut self) -> Result<(), embassy_stm32::spi::Error> {
        self.transfer(CMD_SR)?;
        // Table 8: reset times ≤ 20 µs; rusEFI sleeps 3 ms for margin.
        embassy_time::block_for(embassy_time::Duration::from_millis(3));

        self.transfer_sequence(INIT_SAFE)?;
        defmt::info!("TLE8888 initialized (outputs disabled, INJ/IGN EN held low)");
        Ok(())
    }

    /// Service the TLE8888 window watchdog (required on variants with WWD enabled).
    pub fn wwd_service(&mut self) -> Result<(), embassy_stm32::spi::Error> {
        self.transfer(tle8888::CMD_WWDSERVICECMD)?;
        Ok(())
    }
}

/// Conservative SPI1 settings for TLE8888 bring-up.
pub fn spi_config() -> SpiConfig {
    let mut cfg = SpiConfig::default();
    cfg.frequency = Hertz(1_000_000);
    cfg
}

/// Log and return whether TLE8888 init succeeded.
pub fn init_or_log(bus: &mut Tle8888Bus) -> bool {
    match bus.init_safe() {
        Ok(()) => true,
        Err(_) => {
            defmt::error!("TLE8888 init failed");
            false
        }
    }
}
