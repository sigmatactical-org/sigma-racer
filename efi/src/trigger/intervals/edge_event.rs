//! [`EdgeEvent`].

#[allow(unused_imports)]
use super::*;

/// A timestamped trigger edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeEvent {
    pub t_us: u64,
    pub line: TriggerLine,
    pub rising: bool,
}
