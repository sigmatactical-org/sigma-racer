//! Trigger sensor configuration and decoded engine phase.

mod trigger_input_kind;
mod trigger_state;
pub use trigger_input_kind::TriggerInputKind;
pub use trigger_state::TriggerState;

use crate::trigger::TriggerWheel;

/// Crank/cam decoder configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerSetup {
    pub crank_wheel: TriggerWheel,
    pub crank_input: TriggerInputKind,
    pub cam_input: TriggerInputKind,
    /// When true, fuel/ignition scheduling waits for cam sync before running sequential.
    pub cam_required: bool,
}
