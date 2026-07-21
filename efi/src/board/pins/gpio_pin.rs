//! [`GpioPin`].

#[allow(unused_imports)]
use super::*;

/// STM32 port/pin identifier (e.g. `PD4` → port D, pin 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GpioPin {
    pub port: GpioPort,
    pub pin: u8,
}
impl GpioPin {
    /// Pin handle from a port + pin index.
    pub const fn new(port: GpioPort, pin: u8) -> Self {
        Self { port, pin }
    }
}
