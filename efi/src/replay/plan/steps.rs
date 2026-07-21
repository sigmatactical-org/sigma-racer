//! [`Steps`].

#[allow(unused_imports)]
use super::*;
use crate::replay::Step;

/// Iterator over the replay schedule.
#[derive(Clone, Debug)]
pub struct Steps {
    pub(crate) plan: ReplayPlan,
    pub(crate) rev: u32,
    pub(crate) tooth: u8,
    pub(crate) cam_pending: bool,
    /// Remainder of the current inter-tooth delay after a cam pulse split it.
    pub(crate) carry_us: u32,
}
impl Iterator for Steps {
    type Item = Step;

    fn next(&mut self) -> Option<Step> {
        if self.cam_pending {
            // Cam fires halfway through the current inter-tooth window;
            // the crank tooth then completes the remainder.
            self.cam_pending = false;
            let half = self.carry_us / 2;
            self.carry_us -= half;
            return Some(Step::Cam { delay_us: half });
        }
        if let Some(step) = self.take_carry() {
            return Some(step);
        }
        if self.rev >= self.plan.revs {
            return None;
        }

        let physical = self.plan.wheel.effective_edges_per_rev();
        let tooth_us = self.plan.tooth_us(self.rev);
        // The gap precedes tooth 0: (missing + 1) pitches.
        let delay = if self.tooth == 0 {
            tooth_us.saturating_mul(self.plan.wheel.missing as u32 + 1)
        } else {
            tooth_us
        };

        let fire_cam = self.rev.is_multiple_of(2) && self.tooth == self.plan.cam_after_tooth;

        // Advance to the next tooth/rev.
        self.tooth += 1;
        if self.tooth >= physical {
            self.tooth = 0;
            self.rev += 1;
        }

        if fire_cam {
            // Split this window: cam first (on the following call), then
            // the crank tooth with the remainder.
            self.carry_us = delay;
            self.cam_pending = true;
            return self.next();
        }
        Some(Step::Crank { delay_us: delay })
    }
}
impl Steps {
    fn take_carry(&mut self) -> Option<Step> {
        if self.carry_us > 0 {
            let delay = self.carry_us;
            self.carry_us = 0;
            return Some(Step::Crank { delay_us: delay });
        }
        None
    }
}
