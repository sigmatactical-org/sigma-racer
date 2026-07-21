//! Generate the baked-in candump replay sample (`testdata/sample-ride.log`).
//!
//! Scripts a short XSR900 ride, encodes each 50 ms state through the real M7
//! DBC codec (`encode_sim_frames`), and prints `candump -L` lines covering all
//! five contract messages. Regenerate with:
//!
//! ```sh
//! cargo run --example gen_sample_log > testdata/sample-ride.log
//! ```

use sigma_racer_telemetry::can::encode_sim_frames;
use sigma_racer_telemetry::state::VehicleState;

const IFACE: &str = "can1";
const DT: f64 = 0.05; // 20 Hz, matching the vehicle daemon's sample rate.
const DURATION_S: f64 = 15.0;
const BASE_EPOCH: f64 = 1_730_000_000.0; // arbitrary fixed base for deterministic output.

fn main() {
    let mut t = 0.0;
    while t < DURATION_S {
        let state = ride_state(t);
        for (id, payload) in encode_sim_frames(&state) {
            println!("({:.6}) {IFACE} {id:03X}#{}", BASE_EPOCH + t, hex(&payload));
        }
        t += DT;
    }
}

/// Scripted ride: idle → launch → accelerate through the gears → cruise → brake
/// to a stop. Values are plausible, not physically exact — the point is to move
/// every gauge signal through its range.
fn ride_state(t: f64) -> VehicleState {
    let mut s = VehicleState::idle();

    // Longitudinal speed profile (km/h) across the 15 s ride.
    let speed = if t < 2.0 {
        0.0
    } else if t < 9.0 {
        // 7 s pull to ~150 km/h.
        150.0 * ((t - 2.0) / 7.0)
    } else if t < 11.0 {
        150.0 // brief top cruise
    } else if t < 14.0 {
        // 3 s brake to a stop.
        150.0 * (1.0 - (t - 11.0) / 3.0)
    } else {
        0.0
    };
    let speed = speed.clamp(0.0, 150.0) as f32;

    let moving = speed > 0.5;
    let gear: i8 = match speed as i32 {
        0 => 0,
        1..=39 => 1,
        40..=69 => 2,
        70..=99 => 3,
        100..=129 => 4,
        _ => 5,
    };

    // RPM tracks speed within the active gear band, with an idle floor.
    let rpm = if moving {
        (2_500.0 + f32::from(gear).mul_add(-350.0, speed * 78.0)).clamp(1_800.0, 11_400.0)
    } else {
        1_200.0
    };

    // Throttle: hard on while accelerating, trailing while braking.
    let throttle = if (2.0..9.0).contains(&t) {
        85.0
    } else if (9.0..11.0).contains(&t) {
        35.0
    } else if moving {
        5.0
    } else {
        0.0
    };

    s.speed = speed;
    s.rpm = rpm;
    s.gear = gear;
    s.throttle_pct = throttle;
    s.side_stand = !moving;
    s.lean_angle = if moving {
        22.0 * (t * 0.7).sin() as f32
    } else {
        0.0
    };
    s.gforce = if (2.0..9.0).contains(&t) {
        0.55
    } else if (11.0..14.0).contains(&t) {
        -0.6
    } else {
        0.0
    };
    // Warm the engine over the ride; drop fuel and load the bus a little.
    s.coolant_c = (42.0 + t * 3.2).min(92.0) as i16;
    s.oil_c = (52.0 + t * 3.6).min(105.0) as i16;
    s.fuel_pct = (62.0 - t * 0.15) as f32;
    s.battery_v = if moving { 13.9 } else { 13.1 };
    s.can_load = if moving { 24 } else { 9 };
    s.abs_active = (11.5..12.2).contains(&t); // a brief ABS event under braking
    s.tc_active = (2.5..3.2).contains(&t); // traction control off the line
    s.odometer = 1_245.0 + (speed * 0.0008); // creeps up

    s.refresh_derived();
    s
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02X}"));
    }
    out
}
