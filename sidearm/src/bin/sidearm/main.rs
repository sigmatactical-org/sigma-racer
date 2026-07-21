//! # sigma-racer-sidearm
//!
//! All-Rust **M7 safety-core** firmware for the Sigma Racer Wingman instrument
//! cluster, targeting the i.MX8M Plus **Cortex-M7** real-time core.
//!
//! Per the M7 architecture decisions:
//! 1. **The M7 owns the safety CAN bus** to the ECU (`sigma-racer-efi`); Linux
//!    owns a second bus independently.
//! 2. **No telltale output** — the M7 is a real-time CAN gateway + supervisor,
//!    not a lamp driver.
//! 3. **All Rust** — reusing the shared [`sigma_racer_sidearm`] contract
//!    so this core and the Linux `sigma-racer-vehicle` can never disagree on the
//!    message IDs or signal scaling.
//!
//! Built on **Embassy**: the chip-agnostic thread-mode executor plus a
//! SysTick-backed `embassy-time` driver (see [`time`]), since the i.MX8MP M7
//! has no vendor Embassy HAL.
//!
//! This is scaffolding: the control flow and module seams are in place, but the
//! hardware drivers (FlexCAN-FD, RPMsg/OpenAMP, watchdog) are stubs marked with
//! `TODO`. It builds for `thumbv7em-none-eabihf` and does not touch hardware.

#![no_std]
#![no_main]
#![deny(unsafe_code)]

use panic_halt as _;

// cortex-m-rt requires `pre_init` to be an unsafe fn; hw::init is the
// board bring-up documented in `hw`.
#[allow(unsafe_code)]
#[cortex_m_rt::pre_init]
unsafe fn pre_init() {
    sigma_racer_sidearm::hw::init();
}

mod bus;
mod rpmsg;
mod supervisor;
mod time;

use bus::{Heartbeat, SafetyBus};
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use rpmsg::RpmsgLink;
use sigma_racer_sidearm as m7;

/// Safety-core service cadence (200 Hz). The poll/heartbeat/publish/watchdog
/// pass runs once per period; the executor sleeps in between.
const SERVICE_PERIOD: Duration = Duration::from_millis(5);

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Bring up the SysTick-backed time driver before awaiting anything, or the
    // first `Timer` would never fire.
    let core = cortex_m::Peripherals::take().unwrap();
    time::init(core.SYST);

    // Parse the shared M7 dictionary once at boot. With the `heapless` backend
    // this builds bounded tables with no heap; the reactive loop below never
    // allocates. A parse failure means the embedded `.dbc` is broken (a build
    // bug), so hold in a safe state rather than run with no contract.
    let dbc = match m7::parse() {
        Ok(dbc) => dbc,
        Err(_) => supervisor::fault_loop(),
    };

    let mut state = m7::M7Signals::default();
    let mut bus = SafetyBus::new();
    let mut heartbeat = Heartbeat::new();
    let mut link = RpmsgLink::new();

    loop {
        // 1. Drain the safety bus and decode ECU frames into the shared state.
        while let Some(frame) = bus.poll() {
            m7::decode_into(&dbc, frame.id, frame.payload(), &mut state);
        }

        // 2. Keep the fail-operational heartbeat to the ECU alive.
        heartbeat.tick(&mut bus);

        // 3. Hand the digested state up to Linux over RPMsg.
        link.publish(&state);

        // 4. Feed the watchdog / evaluate load-shedding.
        supervisor::service();

        // Yield to the executor until the next service tick.
        Timer::after(SERVICE_PERIOD).await;
    }
}
