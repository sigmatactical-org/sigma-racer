//! Comms domain: the ECU side of the M7 safety-bus CAN contract.
//!
//! The dictionary (message IDs, DBC, codec) lives in the shared
//! `sigma-racer-sidearm` crate; this domain owns only what the ECU
//! contributes — the [`EcuSnapshot`] and its encoding.
//!
//! Bus rules (`efi.md` §11): classic CAN, ≤8-byte frames (the MRE's F7 has
//! no FDCAN; the cockpit's FD controllers speak classic natively).

pub mod m7;
pub mod snapshot;

pub use m7::{TX_RATE_HZ, m7_signals};
pub use snapshot::EcuSnapshot;

pub use sigma_racer_sidearm::{M7Signals, MESSAGE_IDS, PerformanceMode, parse};
