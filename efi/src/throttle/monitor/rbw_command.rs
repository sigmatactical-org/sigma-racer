//! [`RbwCommand`].

#[allow(unused_imports)]
use super::*;

/// The monitor's verdict for this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RbwCommand {
    /// All checks healthy: rider demand, normalized percent.
    Normal { demand_pct: f32 },
    /// Cut the H-bridge → spring closes the plate → cut fuel. Latched.
    FailSafe,
}
