//! Replay domain: turn the second MRE into a crank/cam signal generator
//! (bench Phase 3 of the mule runbook).
//!
//! - [`step`] — one [`Step`] (crank or cam pulse)
//! - [`plan`] — the [`ReplayPlan`] and its [`Steps`] iterator

pub mod plan;
pub mod step;

pub use plan::{ReplayPlan, Steps};
pub use step::Step;
