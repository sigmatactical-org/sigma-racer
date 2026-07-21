//! Trigger domain: wheel geometry, sensor setup, the sync decoder, and the
//! Phase-1 edge statistics.
//!
//! - [`wheel`] — [`TriggerWheel`] geometry + [`rpm_from_period_us`]
//! - [`setup`] — [`TriggerSetup`], [`TriggerInputKind`], [`TriggerState`]
//! - [`decoder`] — the [`Decoder`] sync state machine (the spark gate)
//! - [`intervals`] — [`EdgeIntervals`], the tooth-pattern discovery tool

pub mod decoder;
pub mod intervals;
pub mod setup;
pub mod wheel;

pub use decoder::{DecodeEvent, Decoder, DecoderOutput, DesyncCause, SyncState};
pub use intervals::{EdgeEvent, EdgeIntervals, EdgeReport, TriggerLine};
pub use setup::{TriggerInputKind, TriggerSetup, TriggerState};
pub use wheel::{TriggerWheel, rpm_from_period_us};
