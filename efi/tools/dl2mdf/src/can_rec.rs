//! [`CanRec`].

#[allow(unused_imports)]
use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CanRec {
    pub(crate) t_s: f64,
    pub(crate) id: u32,
    pub(crate) dlc: u8,
    /// First 8 data bytes, big-endian as printed.
    pub(crate) data: u64,
}
