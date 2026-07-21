//! Replay plan and its step iterator — the crank/cam schedule the second
//! MRE plays as a signal generator (bench Phase 3).
//!
//! The host tests close the loop: a generated plan fed into
//! [`crate::trigger::Decoder`] must reach full sync and track RPM.

mod steps;
pub use steps::Steps;

use crate::trigger::TriggerWheel;

/// A linear-RPM replay plan over a fixed number of revolutions.
///
/// The cam pulses once per 720° (every second revolution), between two
/// crank teeth — its exact crank angle is arbitrary for bench purposes,
/// which mirrors reality: the true CP3 cam angle is ⚠ [MEASURE] data.
#[derive(Clone, Copy, Debug)]
pub struct ReplayPlan {
    pub wheel: TriggerWheel,
    pub rpm_start: f32,
    pub rpm_end: f32,
    pub revs: u32,
    /// Physical tooth index after which the cam pulse fires (even revs).
    pub cam_after_tooth: u8,
}

impl ReplayPlan {
    /// Constant-speed plan.
    pub const fn constant(wheel: TriggerWheel, rpm: f32, revs: u32) -> Self {
        Self {
            wheel,
            rpm_start: rpm,
            rpm_end: rpm,
            revs,
            cam_after_tooth: 2,
        }
    }

    /// Linear sweep plan (cranking flare, accel run).
    pub const fn sweep(wheel: TriggerWheel, rpm_start: f32, rpm_end: f32, revs: u32) -> Self {
        Self {
            wheel,
            rpm_start,
            rpm_end,
            revs,
            cam_after_tooth: 2,
        }
    }

    /// Iterate the electrical edge sequence of this plan.
    pub fn steps(&self) -> Steps {
        Steps {
            plan: *self,
            rev: 0,
            tooth: 0,
            cam_pending: false,
            carry_us: 0,
        }
    }

    fn rpm_at_rev(&self, rev: u32) -> f32 {
        if self.revs <= 1 {
            return self.rpm_start;
        }
        let frac = rev as f32 / (self.revs - 1) as f32;
        self.rpm_start + (self.rpm_end - self.rpm_start) * frac
    }

    /// One tooth pitch at this revolution's speed, µs.
    fn tooth_us(&self, rev: u32) -> u32 {
        let rpm = self.rpm_at_rev(rev);
        if rpm <= 0.0 {
            return u32::MAX;
        }
        (60_000_000.0 / (rpm * self.wheel.teeth as f32)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::Step;
    use crate::trigger::{Decoder, SyncState};

    const WHEEL: TriggerWheel = TriggerWheel {
        teeth: 12,
        missing: 1,
    };

    /// Play a plan into the decoder, as the bench will electrically.
    fn play(plan: &ReplayPlan, decoder: &mut Decoder) {
        let mut t_us = 0u64;
        for step in plan.steps() {
            t_us += step.delay_us() as u64;
            match step {
                Step::Crank { .. } => {
                    decoder.on_crank_edge(t_us);
                }
                Step::Cam { .. } => {
                    decoder.on_cam_edge(t_us);
                }
            }
        }
    }

    #[test]
    fn edge_counts_match_wheel_geometry() {
        let plan = ReplayPlan::constant(WHEEL, 1_200.0, 4);
        let crank = plan
            .steps()
            .filter(|s| matches!(s, Step::Crank { .. }))
            .count();
        let cam = plan
            .steps()
            .filter(|s| matches!(s, Step::Cam { .. }))
            .count();
        assert_eq!(crank, 4 * WHEEL.effective_edges_per_rev() as usize);
        assert_eq!(cam, 2, "one cam pulse per 720°");
    }

    #[test]
    fn generated_stream_drives_decoder_to_full_sync() {
        let plan = ReplayPlan::constant(WHEEL, 1_200.0, 4);
        let mut decoder = Decoder::new(WHEEL, true);
        play(&plan, &mut decoder);
        assert_eq!(decoder.state(), SyncState::SyncFull);
        assert!(
            (decoder.rpm() - 1_200.0).abs() < 40.0,
            "rpm {}",
            decoder.rpm()
        );
    }

    #[test]
    fn sweep_stream_holds_sync_through_acceleration() {
        let plan = ReplayPlan::sweep(WHEEL, 300.0, 3_000.0, 12);
        let mut decoder = Decoder::new(WHEEL, true);
        play(&plan, &mut decoder);
        assert_eq!(decoder.state(), SyncState::SyncFull);
        assert!(
            (decoder.rpm() - 3_000.0).abs() < 120.0,
            "rpm {}",
            decoder.rpm()
        );
    }

    #[test]
    fn cam_split_preserves_total_window_time() {
        // Sum of all delays must equal the pure-crank plan's total: the cam
        // pulse splits a window, it must not stretch it.
        let with_cam = ReplayPlan::constant(WHEEL, 1_000.0, 2);
        let mut no_cam = with_cam;
        no_cam.cam_after_tooth = u8::MAX; // never fires
        let sum = |p: &ReplayPlan| -> u64 { p.steps().map(|s| s.delay_us() as u64).sum() };
        assert_eq!(sum(&with_cam), sum(&no_cam));
    }

    #[test]
    fn works_on_sixty_minus_two() {
        let wheel = TriggerWheel::sixty_minus_two();
        let plan = ReplayPlan::constant(wheel, 900.0, 4);
        let mut decoder = Decoder::new(wheel, true);
        play(&plan, &mut decoder);
        assert_eq!(decoder.state(), SyncState::SyncFull);
    }
}
