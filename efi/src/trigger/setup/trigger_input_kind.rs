//! [`TriggerInputKind`].

#[allow(unused_imports)]
use super::*;

/// Sensor type wired to a trigger input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerInputKind {
    /// Variable reluctance crank/cam pickup.
    Vr,
    /// Hall-effect digital sensor.
    Hall,
}
