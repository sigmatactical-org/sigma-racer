//! [`TriggerLine`].

#[allow(unused_imports)]
use super::*;

/// Which trigger line an edge came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerLine {
    Crank,
    Cam,
}
