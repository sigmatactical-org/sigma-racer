//! Calibration table interpolation — the lookup core under VE, ignition
//! advance, dead-time and dwell (`efi.md` §5–6).
//!
//! Fixed-size, `no_std`, allocation-free. Axes must be strictly increasing;
//! lookups clamp at the edges (no extrapolation — an engine at 110 kPa on a
//! 100 kPa-topped table gets the edge cell, never an invented value).
//! Values in any table shipped here are ⚠ [MEASURE] shapes, not calibration
//! — the dyno owns the numbers (`efi.md` project rule).

mod curve;
pub use curve::Curve;

/// 2-D table: `rows` (e.g. RPM) × `cols` (e.g. MAP kPa), bilinear.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Table<const R: usize, const C: usize> {
    pub row_axis: [f32; R],
    pub col_axis: [f32; C],
    pub values: [[f32; C]; R],
}

impl<const R: usize, const C: usize> Table<R, C> {
    /// Bilinear interpolation with clamped edges.
    pub fn lookup(&self, row: f32, col: f32) -> f32 {
        let (r, rf) = axis_index(&self.row_axis, row);
        let (c, cf) = axis_index(&self.col_axis, col);
        let top = self.values[r][c] + (self.values[r][c + 1] - self.values[r][c]) * cf;
        let bottom =
            self.values[r + 1][c] + (self.values[r + 1][c + 1] - self.values[r + 1][c]) * cf;
        top + (bottom - top) * rf
    }
}

/// Locate `x` on a strictly-increasing axis: returns the lower index and
/// the fraction toward the next point, clamped to `[0, N-2]` / `[0, 1]`.
fn axis_index<const N: usize>(axis: &[f32; N], x: f32) -> (usize, f32) {
    if x <= axis[0] {
        return (0, 0.0);
    }
    if x >= axis[N - 1] {
        return (N - 2, 1.0);
    }
    let mut i = 0;
    while i < N - 2 && x >= axis[i + 1] {
        i += 1;
    }
    let span = axis[i + 1] - axis[i];
    let frac = if span > 0.0 {
        (x - axis[i]) / span
    } else {
        0.0
    };
    (i, frac)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> Curve<4> {
        Curve {
            x: [0.0, 10.0, 20.0, 40.0],
            y: [1.0, 2.0, 4.0, 8.0],
        }
    }

    #[test]
    fn curve_hits_knots_and_interpolates_between() {
        let c = curve();
        assert_eq!(c.lookup(10.0), 2.0);
        assert_eq!(c.lookup(15.0), 3.0);
        assert_eq!(c.lookup(30.0), 6.0);
    }

    #[test]
    fn curve_clamps_at_edges() {
        let c = curve();
        assert_eq!(c.lookup(-5.0), 1.0);
        assert_eq!(c.lookup(99.0), 8.0);
    }

    fn table() -> Table<3, 3> {
        Table {
            row_axis: [1_000.0, 4_000.0, 8_000.0],
            col_axis: [20.0, 60.0, 100.0],
            values: [[10.0, 20.0, 30.0], [40.0, 50.0, 60.0], [70.0, 80.0, 90.0]],
        }
    }

    #[test]
    fn table_hits_grid_points() {
        let t = table();
        assert_eq!(t.lookup(1_000.0, 20.0), 10.0);
        assert_eq!(t.lookup(4_000.0, 60.0), 50.0);
        assert_eq!(t.lookup(8_000.0, 100.0), 90.0);
    }

    #[test]
    fn table_bilinear_center_of_cell() {
        let t = table();
        // Center of the first cell: mean of its four corners.
        assert_eq!(t.lookup(2_500.0, 40.0), (10.0 + 20.0 + 40.0 + 50.0) / 4.0);
    }

    #[test]
    fn table_clamps_beyond_axes() {
        let t = table();
        assert_eq!(t.lookup(0.0, 0.0), 10.0);
        assert_eq!(t.lookup(20_000.0, 300.0), 90.0);
        assert_eq!(t.lookup(0.0, 300.0), 30.0);
    }
}
