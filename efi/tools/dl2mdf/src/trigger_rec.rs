//! [`TriggerRec`].

#[allow(unused_imports)]
use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TriggerRec {
    pub(crate) t_us: u64,
    /// 0 = crank, 1 = cam.
    pub(crate) line: u64,
    pub(crate) count: u64,
    pub(crate) period_us: u64,
    pub(crate) gap_ratio_x100: u64,
}
