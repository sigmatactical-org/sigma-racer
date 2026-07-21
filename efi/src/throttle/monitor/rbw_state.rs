//! [`RbwState`].

#[allow(unused_imports)]
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RbwState {
    Armed,
    Tripped(TripCause),
}
