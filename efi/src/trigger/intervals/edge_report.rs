//! [`EdgeReport`].

#[allow(unused_imports)]
use super::*;

/// Derived numbers for one recorded edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeReport {
    /// Total edges seen on this line, including this one.
    pub count: u32,
    /// Microseconds since the previous edge.
    pub period_us: u32,
    /// This period ÷ previous period, ×100. Zero when no previous period.
    pub gap_ratio_x100: u32,
}
