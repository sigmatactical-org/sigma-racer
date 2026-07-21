//! Shared types for anomaly detectors.

use crate::state::VehicleState;

/// Milliseconds since the Unix epoch. Always derived from message timestamps
/// (or captured once at the producer's loop boundary) — detectors never read a
/// clock themselves, so replaying a session reproduces events exactly.
pub type TsMillis = i64;

/// How urgent an anomaly is for the rider / shop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Advisory,
    Warning,
    Critical,
}

impl Severity {
    /// Uppercase label used on the wire and in UIs.
    pub fn label(self) -> &'static str {
        match self {
            Self::Advisory => "ADVISORY",
            Self::Warning => "WARNING",
            Self::Critical => "CRITICAL",
        }
    }

    /// Parse a wire label back into a severity.
    pub fn from_label(s: &str) -> Option<Self> {
        match s {
            "ADVISORY" => Some(Self::Advisory),
            "WARNING" => Some(Self::Warning),
            "CRITICAL" => Some(Self::Critical),
            _ => None,
        }
    }
}

/// What kind of condition a detector watches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    /// Derived from decoded signal values; unreliable while signals are stale.
    StateBased,
    /// The signal feed itself is broken (stale, silent).
    SensorFault,
}

/// A detector state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Raised,
    Cleared,
}

/// Numeric signal accessor (plain fn pointer: `Copy`, no allocation).
pub type SignalFn = fn(&VehicleState) -> f64;

/// Boolean condition over the whole state.
pub type PredicateFn = fn(&VehicleState) -> bool;
