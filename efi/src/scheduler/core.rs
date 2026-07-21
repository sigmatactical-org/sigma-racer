//! Angle-domain event scheduler — the core trick of `efi.md` §3.
//!
//! Fuel and spark are commanded in **crank angles**; hardware fires on
//! **timestamps**. On every tooth the decoder reports position and speed,
//! and the scheduler *predicts* the absolute time of each angle-event inside
//! the window up to the next expected tooth, emitting arm requests for
//! hardware compare channels. Acceleration is handled by construction —
//! every tooth re-predicts from the freshest velocity.
//!
//! Windows are half-open `(now, now + window]`: consecutive tooth windows
//! tile the cycle exactly, so each event arms **exactly once per cycle**.

mod table_full;
pub use table_full::TableFull;

use crate::scheduler::{AngleEvent, Armed, ArmedBuf, EventId, MAX_EVENTS};

#[derive(Debug)]
pub struct Scheduler {
    cycle_deg: f32,
    events: [Option<AngleEvent>; MAX_EVENTS],
}

impl Scheduler {
    /// Scheduler for an engine cycle of `cycle_deg` degrees.
    pub const fn new(cycle_deg: f32) -> Self {
        Self {
            cycle_deg,
            events: [None; MAX_EVENTS],
        }
    }

    /// Insert or replace (by `id`) an angle event. Angles are normalized
    /// into `0..cycle_deg`.
    pub fn set_event(&mut self, id: EventId, angle_deg: f32) -> Result<(), TableFull> {
        let mut angle = angle_deg % self.cycle_deg;
        if angle < 0.0 {
            angle += self.cycle_deg;
        }
        let event = AngleEvent {
            id,
            angle_deg: angle,
        };
        if let Some(slot) = self
            .events
            .iter_mut()
            .find(|slot| slot.is_some_and(|e| e.id == id))
        {
            *slot = Some(event);
            return Ok(());
        }
        match self.events.iter_mut().find(|slot| slot.is_none()) {
            Some(slot) => {
                *slot = Some(event);
                Ok(())
            }
            None => Err(TableFull),
        }
    }

    /// Remove a pending event by id.
    pub fn clear_event(&mut self, id: EventId) {
        for slot in &mut self.events {
            if slot.is_some_and(|e| e.id == id) {
                *slot = None;
            }
        }
    }

    /// Called on every decoded tooth.
    ///
    /// * `t_us` — timestamp of this tooth.
    /// * `angle_deg` — cycle angle of this tooth (decoder output).
    /// * `deg_per_us` — instantaneous angular velocity.
    /// * `window_deg` — degrees until the next expected tooth (one pitch,
    ///   or `(missing + 1) ×` pitch entering the gap).
    ///
    /// Arms every event with cycle angle in `(angle, angle + window]`,
    /// appending to `out` (not cleared here — the caller owns the buffer
    /// lifecycle).
    pub fn on_tooth(
        &self,
        t_us: u64,
        angle_deg: f32,
        deg_per_us: f32,
        window_deg: f32,
        out: &mut ArmedBuf,
    ) {
        if deg_per_us <= 0.0 || window_deg <= 0.0 {
            return;
        }
        for event in self.events.iter().flatten() {
            // Distance ahead of the current angle, wrapped into the cycle.
            let mut ahead = event.angle_deg - angle_deg;
            if ahead <= 0.0 {
                ahead += self.cycle_deg;
            }
            if ahead > 0.0 && ahead <= window_deg {
                let dt_us = ahead / deg_per_us;
                out.push(Armed {
                    id: event.id,
                    t_us: t_us + dt_us as u64,
                });
            }
        }
    }
}

/// Angular velocity from RPM, degrees per microsecond.
pub fn deg_per_us_from_rpm(rpm: f32) -> f32 {
    rpm * 360.0 / 60_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const CYCLE: f32 = 720.0;
    /// 12-position wheel: 30° pitch.
    const PITCH: f32 = 30.0;

    fn armed_for(buf: &ArmedBuf, id: EventId) -> Option<u64> {
        buf.iter().find(|a| a.id == id).map(|a| a.t_us)
    }

    /// Simulate one full cycle of teeth at constant rpm, collecting arms.
    fn run_cycle(sched: &Scheduler, rpm: f32, t0: u64) -> Vec<Armed> {
        let dpu = deg_per_us_from_rpm(rpm);
        let tooth_dt = (PITCH / dpu) as u64;
        let mut armed = Vec::new();
        let mut t = t0;
        let steps = (CYCLE / PITCH) as usize;
        for step in 0..steps {
            let angle = step as f32 * PITCH;
            let mut buf = ArmedBuf::new();
            sched.on_tooth(t, angle, dpu, PITCH, &mut buf);
            armed.extend(buf.iter().copied());
            t += tooth_dt;
        }
        armed
    }

    #[test]
    fn constant_speed_event_lands_at_exact_angle_time() {
        let mut sched = Scheduler::new(CYCLE);
        sched.set_event(EventId::Fire(0), 95.0).unwrap();

        // 3000 rpm: 18 deg/ms.
        let rpm = 3_000.0;
        let dpu = deg_per_us_from_rpm(rpm);
        let armed = run_cycle(&sched, rpm, 1_000_000);

        assert_eq!(armed.len(), 1, "exactly once per cycle");
        // Fired from the tooth at 90°: 5° ahead.
        let t_tooth_90 = 1_000_000 + ((90.0 / PITCH) as u64) * ((PITCH / dpu) as u64);
        let expected = t_tooth_90 + (5.0 / dpu) as u64;
        assert_eq!(armed[0].t_us, expected);
    }

    #[test]
    fn every_event_arms_exactly_once_per_cycle() {
        let mut sched = Scheduler::new(CYCLE);
        // Events all over the cycle, including window-boundary angles.
        let angles = [0.0, 5.0, 30.0, 199.9, 360.0, 540.0, 719.9];
        for (i, &angle) in angles.iter().enumerate() {
            sched.set_event(EventId::InjOpen(i as u8), angle).unwrap();
        }
        let armed = run_cycle(&sched, 1_200.0, 500_000);
        assert_eq!(armed.len(), angles.len());
        for (i, _) in angles.iter().enumerate() {
            assert_eq!(
                armed
                    .iter()
                    .filter(|a| a.id == EventId::InjOpen(i as u8))
                    .count(),
                1,
                "event {i} must arm exactly once"
            );
        }
    }

    #[test]
    fn window_is_half_open_start_excluded_end_included() {
        let mut sched = Scheduler::new(CYCLE);
        sched.set_event(EventId::Fire(0), 30.0).unwrap(); // window end
        sched.set_event(EventId::Fire(1), 0.0).unwrap(); // window start

        let dpu = deg_per_us_from_rpm(1_000.0);
        let mut buf = ArmedBuf::new();
        sched.on_tooth(0, 0.0, dpu, PITCH, &mut buf);

        // 30° (end) arms; 0° (start, = current tooth) does not — it belongs
        // to the previous window that ended here.
        assert!(armed_for(&buf, EventId::Fire(0)).is_some());
        assert!(armed_for(&buf, EventId::Fire(1)).is_none());
    }

    #[test]
    fn wraps_across_cycle_end() {
        let mut sched = Scheduler::new(CYCLE);
        sched.set_event(EventId::InjClose(2), 10.0).unwrap();

        let dpu = deg_per_us_from_rpm(2_000.0);
        let mut buf = ArmedBuf::new();
        // Tooth at 690° with a 30° window: (690, 720] — 10° is *not* in it.
        sched.on_tooth(9_000_000, 690.0, dpu, PITCH, &mut buf);
        assert!(buf.is_empty());
        // Tooth at 0° (wrapped): (0, 30] contains 10°.
        sched.on_tooth(9_100_000, 0.0, dpu, PITCH, &mut buf);
        let t = armed_for(&buf, EventId::InjClose(2)).unwrap();
        assert_eq!(t, 9_100_000 + (10.0 / dpu) as u64);
    }

    #[test]
    fn gap_window_covers_missing_tooth_span() {
        let mut sched = Scheduler::new(CYCLE);
        // Event inside where the missing tooth would be.
        sched.set_event(EventId::DwellStart(1), 345.0).unwrap();

        let dpu = deg_per_us_from_rpm(1_500.0);
        let mut buf = ArmedBuf::new();
        // Last physical tooth of the rev at 330°; gap window = 2 × pitch.
        sched.on_tooth(2_000_000, 330.0, dpu, 2.0 * PITCH, &mut buf);
        let t = armed_for(&buf, EventId::DwellStart(1)).unwrap();
        assert_eq!(t, 2_000_000 + (15.0 / dpu) as u64);
    }

    #[test]
    fn acceleration_error_stays_below_one_tooth() {
        // Ground truth: engine accelerating hard; integrate angle exactly.
        // 600 → 6000 rpm over 2 cycles (brutal — a first-start flare).
        let mut sched = Scheduler::new(CYCLE);
        sched.set_event(EventId::Fire(0), 275.0).unwrap();

        // Simulate: angular velocity w(t) = w0 + a·t (deg/µs).
        let w0 = deg_per_us_from_rpm(600.0);
        let w_end = deg_per_us_from_rpm(6_000.0);
        let total_deg = 2.0 * CYCLE;
        // From v² = v0² + 2aΔθ.
        let accel = (w_end * w_end - w0 * w0) / (2.0 * total_deg);

        let angle_at = |t: f64| w0 as f64 * t + 0.5 * accel as f64 * t * t;
        // Invert θ(t) by bisection for ground-truth event times.
        let time_of_angle = |theta: f64| {
            let (mut lo, mut hi) = (0.0f64, 10_000_000.0);
            for _ in 0..64 {
                let mid = (lo + hi) / 2.0;
                if angle_at(mid) < theta {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            (lo + hi) / 2.0
        };

        // Walk the teeth; predict from instantaneous velocity like the ISR.
        let mut worst_err_us = 0.0f64;
        let mut checked = 0;
        let steps = (total_deg / PITCH) as usize;
        for step in 0..steps {
            let theta = step as f64 * PITCH as f64;
            let t_tooth = time_of_angle(theta);
            let w_now = (w0 as f64 + accel as f64 * t_tooth) as f32;
            let mut buf = ArmedBuf::new();
            sched.on_tooth(
                t_tooth as u64,
                (theta % CYCLE as f64) as f32,
                w_now,
                PITCH,
                &mut buf,
            );
            if let Some(t_armed) = armed_for(&buf, EventId::Fire(0)) {
                let cycle_base = (theta / CYCLE as f64).floor() * CYCLE as f64;
                let t_true = time_of_angle(cycle_base + 275.0);
                worst_err_us = worst_err_us.max((t_armed as f64 - t_true).abs());
                checked += 1;
            }
        }
        assert_eq!(checked, 2, "event must arm once per cycle");

        // Error bound: strictly inside one tooth period at the fastest speed
        // reached — the re-predict-per-tooth guarantee.
        let min_tooth_us = (PITCH / w_end) as f64;
        assert!(
            worst_err_us < min_tooth_us,
            "worst error {worst_err_us:.1} µs vs tooth {min_tooth_us:.1} µs"
        );
    }

    #[test]
    fn set_event_replaces_by_id_and_reports_full() {
        let mut sched = Scheduler::new(CYCLE);
        sched.set_event(EventId::Fire(0), 100.0).unwrap();
        sched.set_event(EventId::Fire(0), 200.0).unwrap(); // replace

        let dpu = deg_per_us_from_rpm(1_000.0);
        let mut buf = ArmedBuf::new();
        sched.on_tooth(0, 90.0, dpu, PITCH, &mut buf);
        assert!(buf.is_empty(), "old angle must be gone");
        sched.on_tooth(0, 180.0, dpu, PITCH, &mut buf);
        assert_eq!(buf.len(), 1);

        // Fill the table to capacity with distinct ids.
        let mut sched = Scheduler::new(CYCLE);
        for i in 0..MAX_EVENTS {
            sched
                .set_event(EventId::InjOpen(i as u8), i as f32)
                .unwrap();
        }
        assert_eq!(
            sched.set_event(EventId::Fire(9), 1.0),
            Err(TableFull),
            "17th distinct event must not fit"
        );
    }

    #[test]
    fn zero_velocity_arms_nothing() {
        let mut sched = Scheduler::new(CYCLE);
        sched.set_event(EventId::Fire(0), 10.0).unwrap();
        let mut buf = ArmedBuf::new();
        sched.on_tooth(0, 0.0, 0.0, PITCH, &mut buf);
        assert!(buf.is_empty());
    }
}
