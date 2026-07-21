//! [`GpioPort`].

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GpioPort {
    A,
    B,
    C,
    D,
    E,
}
