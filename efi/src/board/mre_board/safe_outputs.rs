//! [`SafeOutputs`].

#[allow(unused_imports)]
use super::*;
use embassy_stm32::gpio::Output;

/// All actuator pins driven to a known-safe state before any go/no-go decision.
pub struct SafeOutputs {
    pub ignition: [Output<'static>; 4],
    pub inj_en: Output<'static>,
    pub ign_en: Output<'static>,
    pub etb_pwm: Output<'static>,
    pub etb_dir: Output<'static>,
    pub etb_disable: Output<'static>,
}
impl SafeOutputs {
    /// Injectors/coils off, TLE8888 enables low, ETB H-bridge disabled.
    pub fn drive_off(&mut self) {
        for coil in &mut self.ignition {
            coil.set_low();
        }
        self.inj_en.set_low();
        self.ign_en.set_low();
        self.etb_pwm.set_low();
        self.etb_dir.set_low();
        // TLE9201 DIS low = motor disabled.
        self.etb_disable.set_low();
    }
}
