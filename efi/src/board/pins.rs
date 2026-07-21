//! STM32 GPIO and peripheral assignments for microRusEFI (rev 0.6.x wiring).
//!
//! Injectors and several outputs route through the onboard TLE8888 smart driver;
//! logical connector names are preserved as documentation on each field.

mod gpio_pin;
mod gpio_port;
pub use gpio_pin::GpioPin;
pub use gpio_port::GpioPort;

/// MCU pin names for the microRusEFI PCB (STM32F767).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardPins {
    // --- Status LEDs (on PCB) ---
    pub led_comms: GpioPin,
    pub led_running: GpioPin,
    pub led_warning: GpioPin,
    pub led_critical: GpioPin,

    // --- Ignition (TC4427, 5 V logic) ---
    pub ignition: [GpioPin; 4],

    // --- Trigger inputs ---
    /// Primary crank — VR conditioner via TLE8888 (pin 45).
    pub trigger_crank: GpioPin,
    /// Cam hall input (pin 25).
    pub trigger_cam: GpioPin,

    // --- TLE8888 (injectors + low-side outputs) SPI1 ---
    pub tle8888_spi_mosi: GpioPin,
    pub tle8888_spi_miso: GpioPin,
    pub tle8888_spi_sck: GpioPin,
    pub tle8888_spi_cs: GpioPin,
    /// Active-high enable for TLE8888 injector outputs (PD11 + pulldown).
    pub tle8888_inj_en: GpioPin,
    /// Active-high enable for TLE8888 ignition outputs (PD10 + pulldown).
    pub tle8888_ign_en: GpioPin,

    // --- Electronic throttle (TLE9201) ---
    pub etb_pwm: GpioPin,
    pub etb_dir_a: GpioPin,
    /// TLE9201 DIS — low disables the H-bridge (rusEFI `setupTLE9201` disable pin).
    pub etb_disable: GpioPin,

    // --- CAN ---
    pub can_tx: GpioPin,
    pub can_rx: GpioPin,

    // --- Onboard microSD (SPI2, v0.6.0+) ---
    pub sd_spi_mosi: GpioPin,
    pub sd_spi_miso: GpioPin,
    pub sd_spi_sck: GpioPin,
    pub sd_spi_cs: GpioPin,

    // --- Expansion SPI3 (header, disabled by default in rusEFI) ---
    pub exp_spi_mosi: GpioPin,
    pub exp_spi_miso: GpioPin,
    pub exp_spi_sck: GpioPin,

    // --- Console UART (USART3 on J12/J13 — conflicts with SD DMA in rusEFI) ---
    pub console_tx: GpioPin,
    pub console_rx: GpioPin,
}

impl BoardPins {
    /// Pin map matching rusEFI `board_configuration.cpp` defaults for MRE F7.
    pub const fn mre_f7() -> Self {
        Self {
            led_comms: GpioPin::new(GpioPort::E, 2),
            led_running: GpioPin::new(GpioPort::E, 4),
            led_warning: GpioPin::new(GpioPort::E, 1),
            led_critical: GpioPin::new(GpioPort::E, 3),

            ignition: [
                GpioPin::new(GpioPort::D, 4), // pin 9  — INJ/IGN 1
                GpioPin::new(GpioPort::D, 3), // pin 10
                GpioPin::new(GpioPort::D, 2), // pin 11
                GpioPin::new(GpioPort::D, 1), // pin 12
            ],

            trigger_crank: GpioPin::new(GpioPort::C, 6), // pin 45 VR/Hall
            trigger_cam: GpioPin::new(GpioPort::A, 5),   // pin 25 Hall cam

            tle8888_spi_mosi: GpioPin::new(GpioPort::B, 5),
            tle8888_spi_miso: GpioPin::new(GpioPort::B, 4),
            tle8888_spi_sck: GpioPin::new(GpioPort::B, 3),
            tle8888_spi_cs: GpioPin::new(GpioPort::D, 5),
            tle8888_inj_en: GpioPin::new(GpioPort::D, 11),
            tle8888_ign_en: GpioPin::new(GpioPort::D, 10),

            etb_pwm: GpioPin::new(GpioPort::C, 7),
            etb_dir_a: GpioPin::new(GpioPort::A, 8),
            etb_disable: GpioPin::new(GpioPort::C, 8),

            can_tx: GpioPin::new(GpioPort::B, 6),
            can_rx: GpioPin::new(GpioPort::B, 12),

            sd_spi_mosi: GpioPin::new(GpioPort::B, 15),
            sd_spi_miso: GpioPin::new(GpioPort::B, 14),
            sd_spi_sck: GpioPin::new(GpioPort::B, 13),
            sd_spi_cs: GpioPin::new(GpioPort::E, 15),

            exp_spi_mosi: GpioPin::new(GpioPort::C, 12),
            exp_spi_miso: GpioPin::new(GpioPort::C, 11),
            exp_spi_sck: GpioPin::new(GpioPort::C, 10),

            console_tx: GpioPin::new(GpioPort::B, 10),
            console_rx: GpioPin::new(GpioPort::B, 11),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignition_pins_match_rusefi_board_config() {
        let pins = BoardPins::mre_f7();
        assert_eq!(pins.ignition[0], GpioPin::new(GpioPort::D, 4));
        assert_eq!(pins.ignition[3], GpioPin::new(GpioPort::D, 1));
    }

    #[test]
    fn tle8888_enables_match_mre_wiring() {
        let pins = BoardPins::mre_f7();
        assert_eq!(pins.tle8888_inj_en, GpioPin::new(GpioPort::D, 11));
        assert_eq!(pins.tle8888_ign_en, GpioPin::new(GpioPort::D, 10));
    }

    #[test]
    fn etb_disable_is_tle9201_dis_pin() {
        let pins = BoardPins::mre_f7();
        assert_eq!(pins.etb_disable, GpioPin::new(GpioPort::C, 8));
    }
}
