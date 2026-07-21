//! [`Curve`].

#[allow(unused_imports)]
use super::*;

/// 1-D curve: `x` axis (strictly increasing) → `y`, linear between points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curve<const N: usize> {
    pub x: [f32; N],
    pub y: [f32; N],
}
impl<const N: usize> Curve<N> {
    /// Linear interpolation with clamped ends.
    pub fn lookup(&self, x: f32) -> f32 {
        let (i, frac) = axis_index(&self.x, x);
        self.y[i] + (self.y[i + 1] - self.y[i]) * frac
    }
}
