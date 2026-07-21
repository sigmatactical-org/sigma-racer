//! Watchdog supervision — MCU IWDG backstop + TLE8888 window watchdog feed.

use defmt::warn;
use embassy_executor::Spawner;
use embassy_stm32::Peri;
use embassy_stm32::wdg::IndependentWatchdog;
use embassy_time::{Duration, Timer};
use sigma_racer_efi::board::mre_board::SafeOutputs;
use sigma_racer_efi::board::tle8888::Tle8888Bus;
use sigma_racer_efi::board::tle8888::{IWDG_TIMEOUT_US, WWD_PERIOD_MS};

/// Kick the independent watchdog from any context.
pub fn kick_iwdg() {
    use stm32_metapac::iwdg::vals::Key;
    stm32_metapac::IWDG.kr().write(|w| w.set_key(Key::RESET));
}

/// Configure and start the STM32 IWDG. Must be called once before spawning
/// [`spawn_supervisor`].
pub fn start_iwdg(iwdg: Peri<'static, embassy_stm32::peripherals::IWDG>) {
    let mut wdg = IndependentWatchdog::new(iwdg, IWDG_TIMEOUT_US);
    wdg.unleash();
    kick_iwdg();
}

/// Combined safety supervisor: pet IWDG + service TLE8888 WWD.
#[embassy_executor::task]
async fn supervisor(mut tle: Tle8888Bus) {
    loop {
        if tle.wwd_service().is_err() {
            warn!("TLE8888 WWD service failed");
        }
        kick_iwdg();
        Timer::after(Duration::from_millis(WWD_PERIOD_MS / 2)).await;
    }
}

/// Spawn the safety supervisor task.
pub fn spawn_supervisor(
    spawner: &Spawner,
    tle: Tle8888Bus,
) -> Result<(), embassy_executor::SpawnError> {
    spawner.spawn(supervisor(tle)?);
    Ok(())
}

/// Enter safe state: all actuators off + critical LED fast-blink forever.
pub async fn safe_state_loop(
    safe: &mut SafeOutputs,
    mut critical_led: embassy_stm32::gpio::Output<'static>,
) -> ! {
    safe.drive_off();
    loop {
        critical_led.set_high();
        Timer::after_millis(120).await;
        critical_led.set_low();
        Timer::after_millis(120).await;
        kick_iwdg();
    }
}
