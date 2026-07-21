//! [`RbwInputs`].

#[allow(unused_imports)]
use super::*;

/// One monitor tick's inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RbwInputs {
    pub t_us: u64,
    pub app_a_v: f32,
    pub app_b_v: f32,
    pub tps_a_v: f32,
    pub tps_b_v: f32,
    /// The controller's current plate target, percent.
    pub commanded_pct: f32,
}
