//! Stage-1 analog sampling — the sensor half of the data logger.
//!
//! Sweeps the MRE analog channels at [`SAMPLE_HZ`], scales them with the
//! board front-end constants, publishes the latest [`SensorFrame`] for any
//! consumer (future CAN task), and streams parseable `DL,S` records over
//! defmt. Record format:
//!
//! `DL,S,<t_us>,<vbatt_v>,<clt_c>,<iat_c>,<tps_map_v>,<an1_v>,<an2_v>`

use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::peripherals::ADC1;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Instant, Ticker};
use sigma_racer_efi::board::mre_board::SensorChannels;
use sigma_racer_efi::sensor::{RawSweep, SensorFrame, counts_to_pin_volts};

/// Latest scaled frame, shared with any consumer.
pub static LATEST: Watch<CriticalSectionRawMutex, SensorFrame, 2> = Watch::new();

/// Sampling cadence for stage-1 logging.
const SAMPLE_HZ: u64 = 100;

/// Emit one `DL,S` record every N sweeps (10 Hz at 100 Hz sampling).
const LOG_EVERY: u32 = 10;

/// Long sample time: the divided inputs are high-impedance sources.
const SAMPLE_TIME: SampleTime = SampleTime::CYCLES144;

#[embassy_executor::task]
pub async fn sample(mut adc: Adc<'static, ADC1>, mut channels: SensorChannels) {
    let sender = LATEST.sender();
    let mut ticker = Ticker::every(Duration::from_hz(SAMPLE_HZ));
    let mut sweeps: u32 = 0;

    loop {
        ticker.next().await;

        let sweep = RawSweep {
            vbatt: counts_to_pin_volts(adc.blocking_read(&mut channels.vbatt, SAMPLE_TIME)),
            clt: counts_to_pin_volts(adc.blocking_read(&mut channels.clt, SAMPLE_TIME)),
            iat: counts_to_pin_volts(adc.blocking_read(&mut channels.iat, SAMPLE_TIME)),
            tps_map: counts_to_pin_volts(adc.blocking_read(&mut channels.tps_map, SAMPLE_TIME)),
            an_volt1: counts_to_pin_volts(adc.blocking_read(&mut channels.an_volt1, SAMPLE_TIME)),
            an_volt2: counts_to_pin_volts(adc.blocking_read(&mut channels.an_volt2, SAMPLE_TIME)),
        };
        let frame = SensorFrame::from_sweep(Instant::now().as_micros(), sweep);
        sender.send(frame);

        sweeps = sweeps.wrapping_add(1);
        if sweeps.is_multiple_of(LOG_EVERY) {
            defmt::info!(
                "DL,S,{=u64},{=f32},{=f32},{=f32},{=f32},{=f32},{=f32}",
                frame.t_us,
                frame.vbatt_v,
                frame.clt_c,
                frame.iat_c,
                frame.tps_map_v,
                frame.an_volt1_v,
                frame.an_volt2_v
            );
        }
    }
}
