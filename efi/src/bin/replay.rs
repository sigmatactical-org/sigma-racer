#![no_std]
#![no_main]

//! Trigger replay firmware — flash this onto the **second** MRE to turn it
//! into a crank/cam signal generator for bench Phase 3 (`efi.md` §12
//! stage 3).
//!
//! Outputs (TC4427-buffered 5 V push-pull, ideal for the DUT's trigger
//! inputs):
//! - **Crank**: ignition output 1 — MCU PD4, connector pin 9
//! - **Cam**:   ignition output 2 — MCU PD3, connector pin 10
//!
//! Wire replay pin 9 → DUT pin 45 (crank, Hall-variant input) and replay
//! pin 10 → DUT pin 25 (cam), grounds common. The DUT counts falling
//! edges; each pulse here is high for half the window, low to the edge.
//!
//! The plan loops forever: a cranking flare (300 → 1200 rpm) then a steady
//! idle segment — enough to exercise cold sync, acceleration, and steady
//! running on every pass. Timing jitter is executor-grade (~µs at the
//! 1 MHz tick), which Phase 3 tolerates; scope the outputs to quantify it.

#![forbid(unsafe_code)]

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use sigma_racer_efi::replay::{ReplayPlan, Step};
use sigma_racer_efi::trigger::TriggerWheel;
use {defmt_rtt as _, panic_probe as _};

/// ⚠ [MEASURE] — placeholder wheel until mule Phase 1 reads the real CP3
/// pattern; keep in lockstep with the profile in `engines/yamaha_cp3.rs`.
const WHEEL: TriggerWheel = TriggerWheel {
    teeth: 12,
    missing: 1,
};

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

async fn pulse(pin: &mut Output<'static>, window_us: u32) {
    // High for half the window, low into the (falling) edge the DUT counts.
    let high = window_us / 2;
    pin.set_high();
    Timer::after_micros(high as u64).await;
    pin.set_low();
    Timer::after_micros((window_us - high) as u64).await;
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    sigma_racer_efi::board::heap::init();
    let p = embassy_stm32::init(rcc_config());

    let mut crank = Output::new(p.PD4, Level::Low, Speed::High);
    let mut cam = Output::new(p.PD3, Level::Low, Speed::High);
    let mut led = Output::new(p.PE4, Level::Low, Speed::Low);

    let flare = ReplayPlan::sweep(WHEEL, 300.0, 1_200.0, 8);
    let idle = ReplayPlan::constant(WHEEL, 1_200.0, 32);

    info!(
        "replay: {}-{} wheel — flare 300→1200 rpm ×8 revs, idle 1200 rpm ×32 revs, looping",
        WHEEL.teeth, WHEEL.missing
    );

    loop {
        led.set_high();
        for step in flare.steps() {
            drive(&mut crank, &mut cam, step).await;
        }
        led.set_low();
        for step in idle.steps() {
            drive(&mut crank, &mut cam, step).await;
        }
    }
}

async fn drive(crank: &mut Output<'static>, cam: &mut Output<'static>, step: Step) {
    match step {
        Step::Crank { delay_us } => pulse(crank, delay_us).await,
        Step::Cam { delay_us } => pulse(cam, delay_us).await,
    }
}
