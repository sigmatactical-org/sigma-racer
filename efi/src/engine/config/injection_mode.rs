//! [`InjectionMode`].

#[allow(unused_imports)]
use super::*;

/// How injectors are fired relative to crank events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum InjectionMode {
    /// All injectors open together on each fuel event.
    #[default]
    Simultaneous,
    /// One injector per fuel event, following firing order.
    Sequential,
    /// Pairs or batches of injectors (e.g. batch fire per bank).
    Batch,
}
