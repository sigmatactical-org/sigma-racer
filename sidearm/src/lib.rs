//! Sigma Racer **M7 safety-bus** CAN dictionary and codec.
//!
//! This crate is the single source of truth for the CAN contract on the M7
//! safety bus: the message IDs, the `.dbc` database, and the frame⇄signal
//! codec. Both the Linux `sigma-racer-vehicle` (`std`) and the M7 firmware
//! (`no_std`) depend on it so the two compute domains can never disagree on
//! message IDs or signal scaling.
//!
//! Decoding and encoding are allocation-free (bounded `dbc-rs` buffers); only
//! [`parse`] builds the database and must be called once — lazily via [`dbc`]
//! under `std`, or once at boot on the M7.
//!
//! - [`message`] — the message IDs and [`MESSAGE_IDS`] transmit order
//! - [`performance_mode`] — the [`PerformanceMode`] ride-mode enum
//! - [`signals`] — the shared [`M7Signals`] contract type
//! - [`database`] — the embedded DBC source and [`parse`]
//! - [`decode`] / [`encode`] — the allocation-free frame codec
//! - `cached` — `std`-only lazy [`dbc`] / [`decode`] / [`encode_all`] helpers

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    not(any(feature = "bringup", feature = "firmware")),
    forbid(unsafe_code)
)]

#[cfg(any(feature = "bringup", feature = "firmware"))]
pub mod hw;

pub use dbc_rs::{Dbc, DecodedSignal, Error};

pub mod database;
pub mod decode;
pub mod encode;
pub mod message;
pub mod performance_mode;
pub mod signals;

pub use database::{M7_DBC, parse};
pub use decode::decode_into;
pub use encode::encode_frames;
pub use message::{
    CHASSIS_ELECTRICAL, ENGINE_STATUS, M7_HEARTBEAT, MESSAGE_IDS, THROTTLE_GEAR, TRIP_ODOMETER,
    WHEEL_SPEED,
};
pub use performance_mode::PerformanceMode;
pub use signals::M7Signals;
pub mod wire;
pub use wire::{
    MAGIC as WIRE_MAGIC, VERSION as WIRE_VERSION, decode as decode_wire, encode as encode_wire,
};

#[cfg(feature = "std")]
mod cached;
#[cfg(feature = "std")]
pub use cached::{dbc, decode, encode_all};

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn signals_round_trip_through_can() {
        let sample = M7Signals {
            engine_rpm: 6_000.0,
            coolant_c: 90,
            oil_c: 100,
            redline: false,
            throttle_pct: 42.0,
            gear: 3,
            performance_mode: PerformanceMode::Track,
            side_stand: false,
            ground_speed: 128.0,
            lean_angle: 25.0,
            long_accel: 0.5,
            fuel_pct: 60.0,
            battery_v: 13.5,
            can_load: 12,
            abs_active: true,
            tc_active: false,
            dtc_count: 2,
            odometer: 1_234.5,
            trip1: 100.0,
            trip2: 42.0,
        };

        let frames = encode_all(&sample).expect("encode");
        let mut out = M7Signals::default();
        for (id, payload) in frames {
            assert!(decode(id, &payload, &mut out), "decode 0x{id:03X}");
        }

        assert!((out.engine_rpm - sample.engine_rpm).abs() < 1.0);
        assert_eq!(out.coolant_c, sample.coolant_c);
        assert_eq!(out.oil_c, sample.oil_c);
        assert_eq!(out.gear, sample.gear);
        assert_eq!(out.performance_mode, sample.performance_mode);
        assert_eq!(out.side_stand, sample.side_stand);
        assert_eq!(out.abs_active, sample.abs_active);
        assert_eq!(out.tc_active, sample.tc_active);
        assert_eq!(out.dtc_count, sample.dtc_count);
        assert!((out.ground_speed - sample.ground_speed).abs() < 0.1);
        assert!((out.throttle_pct - sample.throttle_pct).abs() < 0.1);
        assert!((out.odometer - sample.odometer).abs() < 0.1);
    }
}
