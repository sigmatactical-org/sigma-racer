//! Deterministic streaming anomaly detection over [`VehicleState`] samples.
//!
//! Detectors are pure state machines — no clocks, no I/O, no RNG. Timestamps
//! are inputs (from message `ts` or captured once at the producer's loop
//! boundary), so replaying a recorded session reproduces the exact same
//! events. The bike daemon and the shop tools run the same
//! [`AnomalyEngine`]; events travel as protocol `Event` messages and are
//! merged idempotently.
//!
//! [`VehicleState`]: crate::state::VehicleState

mod consistency;
mod engine;
mod event;
mod manager;
mod rate;
mod sequence;
mod staleness;
mod threshold;
mod types;

pub use consistency::{ConsistencyConfig, ConsistencyDetector};
pub use engine::AnomalyEngine;
pub use event::AnomalyEvent;
pub use manager::{AlertManager, AlertMeta, AlertSlot, SUPPRESSOR_ID};
pub use rate::{RateConfig, RateDetector};
pub use sequence::{CounterDetector, SequenceConfig, SequenceDetector};
pub use staleness::{StalenessConfig, StalenessDetector};
pub use threshold::{ThresholdConfig, ThresholdDetector};
pub use types::{Category, Edge, PredicateFn, Severity, SignalFn, TsMillis};

/// Parse a message's RFC 3339 `ts` into epoch milliseconds.
pub fn parse_ts_millis(ts: &str) -> Option<TsMillis> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.timestamp_millis())
}
