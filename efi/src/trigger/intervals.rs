//! Trigger edge interval statistics — the Phase-1 tooth-pattern discovery
//! tool (mule runbook Phase 1).
//!
//! A missing-tooth gap shows up as a period ratio of ~(1 + missing) between
//! consecutive edges, without assuming any wheel geometry up front. Used by
//! the stage-1 data logger before the decoder is trusted.

mod edge_event;
mod edge_report;
mod trigger_line;
pub use edge_event::EdgeEvent;
pub use edge_report::EdgeReport;
pub use trigger_line::TriggerLine;

/// Interval statistics for one trigger line.
///
/// The gap ratio (this period ÷ previous period, ×100 fixed point) is the
/// Phase-1 signal: ~100 tooth-to-tooth at steady speed, ~200 entering a
/// single-missing-tooth gap (~300 for two missing), then back under ~50 on
/// the first tooth after the gap.
#[derive(Debug, Default)]
pub struct EdgeIntervals {
    count: u32,
    last_t_us: Option<u64>,
    last_period_us: Option<u32>,
}

impl EdgeIntervals {
    /// Empty interval tracker.
    pub const fn new() -> Self {
        Self {
            count: 0,
            last_t_us: None,
            last_period_us: None,
        }
    }

    /// Total edges observed.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Record an edge timestamp. Returns `None` for the very first edge on
    /// the line (no period exists yet).
    pub fn record(&mut self, t_us: u64) -> Option<EdgeReport> {
        self.count = self.count.wrapping_add(1);
        let prev_t = self.last_t_us.replace(t_us)?;
        let period_us = t_us.saturating_sub(prev_t).min(u32::MAX as u64) as u32;
        let gap_ratio_x100 = match self.last_period_us {
            Some(prev) if prev > 0 => {
                ((period_us as u64 * 100) / prev as u64).min(u32::MAX as u64) as u32
            }
            _ => 0,
        };
        self.last_period_us = Some(period_us);
        Some(EdgeReport {
            count: self.count,
            period_us,
            gap_ratio_x100,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_edge_yields_no_report() {
        let mut intervals = EdgeIntervals::new();
        assert_eq!(intervals.record(1_000), None);
        assert_eq!(intervals.count(), 1);
    }

    #[test]
    fn uniform_edges_report_unity_gap_ratio() {
        let mut intervals = EdgeIntervals::new();
        intervals.record(0);
        intervals.record(1_000);
        let report = intervals.record(2_000).unwrap();
        assert_eq!(report.period_us, 1_000);
        assert_eq!(report.gap_ratio_x100, 100);
    }

    #[test]
    fn missing_tooth_gap_shows_double_then_half_ratio() {
        let mut intervals = EdgeIntervals::new();
        // Uniform teeth at 1 ms, then a 2 ms gap (one missing tooth),
        // then back to 1 ms.
        intervals.record(0);
        intervals.record(1_000);
        intervals.record(2_000);
        let gap = intervals.record(4_000).unwrap();
        assert_eq!(gap.period_us, 2_000);
        assert_eq!(gap.gap_ratio_x100, 200);
        let after = intervals.record(5_000).unwrap();
        assert_eq!(after.period_us, 1_000);
        assert_eq!(after.gap_ratio_x100, 50);
    }
}
