#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use defmt::{error, info, warn};
use embassy_executor::Spawner;
use embassy_time::Timer;
use sigma_racer_efi::board::mre_board::MreBoard;
use sigma_racer_efi::board::tle8888::init_or_log;
use sigma_racer_efi::board::{BoardPins, bor, wiring};
use sigma_racer_efi::{FIRMWARE_ID, TARGET_MCU, active_profile};
use {defmt_rtt as _, panic_probe as _};

mod tasks;

fn rcc_config() -> embassy_stm32::Config {
    use embassy_stm32::rcc::*;

    let mut config = embassy_stm32::Config::default();
    config.rcc.hsi = true;
    config.rcc.pll_src = PllSource::HSI;
    config.rcc.pll = Some(Pll {
        prediv: PllPreDiv::DIV8,
        mul: PllMul::MUL216,
        divp: Some(PllPDiv::DIV2),
        divq: Some(PllQDiv::DIV9),
        divr: None,
    });
    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.ahb_pre = AHBPrescaler::DIV1;
    config.rcc.apb1_pre = APBPrescaler::DIV4;
    config.rcc.apb2_pre = APBPrescaler::DIV2;
    config
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let pins = BoardPins::mre_f7();
    let profile = active_profile();

    sigma_racer_efi::board::heap::init();
    let p = embassy_stm32::init(rcc_config());

    bor::ensure();

    let (mut board, mut tle, iwdg) = MreBoard::init(pins, p);

    tasks::safety::start_iwdg(iwdg);

    if let Err(_err) = wiring::validate_profile(&profile) {
        error!("invalid engine profile — entering safe state, engine control disabled");
        tasks::safety::safe_state_loop(&mut board.safe, board.led_critical).await;
    }

    if !init_or_log(&mut tle) {
        error!("TLE8888 init failed — entering safe state");
        tasks::safety::safe_state_loop(&mut board.safe, board.led_critical).await;
    }

    info!("{} on {} — engine: {}", FIRMWARE_ID, TARGET_MCU, profile.id);
    info!(
        "Engine: {} cylinders, {} cc, {}° cycle",
        profile.engine.cylinders, profile.engine.displacement_cc, profile.cycle_degrees
    );
    info!("stage 1: characterization data logger (DL,S sensor / DL,T trigger records)");
    warn!("profile trigger geometry + rev limits are UNVERIFIED placeholders — logger stage only");

    let logger_tokens = (
        tasks::sensors::sample(board.adc, board.sensors),
        tasks::trigger::crank_capture(board.crank),
        tasks::trigger::cam_capture(board.cam),
        tasks::trigger::edge_logger(),
    );
    match logger_tokens {
        (Ok(sensors), Ok(crank), Ok(cam), Ok(edges)) => {
            spawner.spawn(sensors);
            spawner.spawn(crank);
            spawner.spawn(cam);
            spawner.spawn(edges);
        }
        _ => {
            error!("failed to spawn data-logger tasks — entering safe state");
            tasks::safety::safe_state_loop(&mut board.safe, board.led_critical).await;
        }
    }

    if tasks::safety::spawn_supervisor(&spawner, tle).is_err() {
        error!("failed to spawn safety supervisor — entering safe state");
        tasks::safety::safe_state_loop(&mut board.safe, board.led_critical).await;
    }

    match heartbeat(board.led_comms) {
        Ok(token) => spawner.spawn(token),
        Err(_) => error!("failed to spawn heartbeat task"),
    }

    loop {
        board.led_running.set_high();
        Timer::after_millis(500).await;
        board.led_running.set_low();
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
async fn heartbeat(mut comms_led: embassy_stm32::gpio::Output<'static>) {
    loop {
        comms_led.set_high();
        Timer::after_millis(50).await;
        comms_led.set_low();
        Timer::after_secs(2).await;
    }
}
