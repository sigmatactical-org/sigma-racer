//! [`EventId`].

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventId {
    /// Injector opens.
    InjOpen(u8),
    /// Injector closes.
    InjClose(u8),
    /// Coil dwell begins (charge).
    DwellStart(u8),
    /// Spark: coil fires.
    Fire(u8),
}
