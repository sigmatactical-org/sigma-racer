//! Crank/cam trigger decoder — the `LOST → SYNCING → SYNC_CRANK → SYNC_FULL`
//! state machine from `efi.md` §4.
//!
//! Pure `no_std` logic: edges go in as microsecond timestamps, engine
//! position and sync confidence come out. The firmware feeds it from
//! input-capture; host tests feed it synthesized golden captures.
//!
//! Rules encoded here:
//! - **No sync = no spark.** [`Decoder::spark_allowed`] is the gate.
//! - Missing-tooth detection by inter-tooth period ratio, thresholds derived
//!   from the wheel geometry (not tuned magic numbers).
//! - A gap arriving anywhere but the expected wheel position, or a missed
//!   gap, degrades sync immediately — never silently.
//! - Noise edges (implausibly short periods) are rejected without losing sync.
//! - The wheel pattern itself is engine data (⚠ [MEASURE] — mule Phase 1);
//!   the decoder takes whatever [`TriggerWheel`] the profile carries.

mod decode_event;
mod decoder_output;
mod desync_cause;
mod sync_state;
mod thresholds;
pub use decode_event::DecodeEvent;
pub use decoder_output::DecoderOutput;
pub use desync_cause::DesyncCause;
pub use sync_state::SyncState;
pub(crate) use thresholds::Thresholds;

use crate::trigger::{TriggerWheel, rpm_from_period_us};

/// The decoder. One instance per engine.
#[derive(Debug)]
pub struct Decoder {
    wheel: TriggerWheel,
    cam_required: bool,
    thresholds: Thresholds,

    state: SyncState,
    last_t_us: Option<u64>,
    last_period_us: Option<u32>,
    /// Wheel position of the last edge: 0 = first tooth after the gap.
    position: u8,
    /// 0 or 1: which 360° of the 720° cycle we are in (cam rev = 0).
    rev_parity: u8,
    rpm: f32,
}

impl Decoder {
    /// Decoder for `wheel`, optionally requiring cam sync for full sync.
    pub fn new(wheel: TriggerWheel, cam_required: bool) -> Self {
        Self {
            wheel,
            cam_required,
            thresholds: Thresholds::for_wheel(wheel),
            state: SyncState::Lost,
            last_t_us: None,
            last_period_us: None,
            position: 0,
            rev_parity: 0,
            rpm: 0.0,
        }
    }

    /// Current sync state.
    pub fn state(&self) -> SyncState {
        self.state
    }

    /// The spark gate: nothing fires below full sync.
    pub fn spark_allowed(&self) -> bool {
        self.state == SyncState::SyncFull
    }

    /// Latest rpm estimate (0 when unsynced).
    pub fn rpm(&self) -> f32 {
        self.rpm
    }

    /// Degrees per wheel position.
    fn pitch_deg(&self) -> f32 {
        360.0 / self.wheel.teeth as f32
    }

    /// Physical teeth on the wheel (positions that produce an edge).
    fn physical_teeth(&self) -> u8 {
        self.wheel.effective_edges_per_rev()
    }

    fn cycle_angle_deg(&self) -> f32 {
        self.position as f32 * self.pitch_deg() + 360.0 * self.rev_parity as f32
    }

    fn output(&self, event: Option<DecodeEvent>) -> DecoderOutput {
        DecoderOutput {
            state: self.state,
            rpm: self.rpm,
            cycle_angle_deg: self.cycle_angle_deg(),
            event,
        }
    }

    fn desync(&mut self, cause: DesyncCause) -> DecoderOutput {
        self.state = SyncState::Lost;
        self.position = 0;
        self.last_period_us = None;
        self.rpm = 0.0;
        self.output(Some(DecodeEvent::Desync(cause)))
    }

    /// Feed one crank edge (already conditioned: one edge per tooth).
    pub fn on_crank_edge(&mut self, t_us: u64) -> DecoderOutput {
        let Some(prev_t) = self.last_t_us else {
            // First edge ever: an anchor, nothing more.
            self.last_t_us = Some(t_us);
            self.state = SyncState::Syncing;
            return self.output(None);
        };

        let period = t_us.saturating_sub(prev_t).min(u32::MAX as u64) as u32;

        let Some(prev_period) = self.last_period_us else {
            // Second edge: first period. Still hunting.
            self.last_t_us = Some(t_us);
            self.last_period_us = Some(period);
            return self.output(None);
        };

        let ratio_x100 = ((period as u64 * 100) / prev_period.max(1) as u64) as u32;

        // Noise: implausibly short. Reject without touching decoder state —
        // the next real tooth measures slightly long and passes the gap
        // thresholds' generous margins.
        if ratio_x100 < self.thresholds.noise_x100 {
            return self.output(Some(DecodeEvent::NoiseRejected));
        }

        // Stall: implausibly long even for the gap.
        if ratio_x100 > self.thresholds.gap_high_x100 {
            self.last_t_us = Some(t_us);
            return self.desync(DesyncCause::Stall);
        }

        self.last_t_us = Some(t_us);
        let is_gap = ratio_x100 >= self.thresholds.gap_low_x100;

        // Normalize the stored period to one tooth pitch so the next ratio
        // is meaningful whether or not this edge closed a gap.
        self.last_period_us = Some(if is_gap {
            period / (self.wheel.missing as u32 + 1)
        } else {
            period
        });

        let tooth_period = self.last_period_us.unwrap_or(period);
        self.rpm = rpm_from_period_us(tooth_period, self.wheel.teeth);

        match (self.state, is_gap) {
            (SyncState::Lost | SyncState::Syncing, true) => {
                let confirming =
                    self.state == SyncState::Syncing && self.position == self.physical_teeth() - 1;
                self.position = 0;
                if confirming {
                    // Second gap with exactly the right tooth count between:
                    // crank sync. Cam (or its absence) decides full sync.
                    if self.cam_required {
                        self.state = SyncState::SyncCrank;
                        self.output(Some(DecodeEvent::SyncAchieved))
                    } else {
                        self.state = SyncState::SyncFull;
                        self.output(Some(DecodeEvent::FullSyncAchieved))
                    }
                } else {
                    // First gap candidate: position now known, count teeth.
                    self.state = SyncState::Syncing;
                    self.output(Some(DecodeEvent::GapDetected))
                }
            }
            (SyncState::Lost | SyncState::Syncing, false) => {
                // Counting teeth between gap candidates. Position saturates
                // rather than wraps; an overlong run resolves at the next
                // gap (count mismatch → stays Syncing).
                self.position = self.position.saturating_add(1).min(self.wheel.teeth);
                self.output(None)
            }
            (SyncState::SyncCrank | SyncState::SyncFull, true) => {
                if self.position == self.physical_teeth() - 1 {
                    self.position = 0;
                    self.rev_parity ^= 1;
                    self.output(Some(DecodeEvent::GapDetected))
                } else {
                    self.desync(DesyncCause::GapAtWrongPosition)
                }
            }
            (SyncState::SyncCrank | SyncState::SyncFull, false) => {
                if self.position >= self.physical_teeth() - 1 {
                    // Ran past the last physical tooth without a gap.
                    self.desync(DesyncCause::MissedGap)
                } else {
                    self.position += 1;
                    self.output(None)
                }
            }
        }
    }

    /// Feed one cam edge. The cam rev is defined as rev 0 of the cycle;
    /// the crank angle at which the cam edge occurs is engine data
    /// (⚠ [MEASURE] — mule Phase 1), not assumed here.
    pub fn on_cam_edge(&mut self, _t_us: u64) -> Option<DecodeEvent> {
        match self.state {
            SyncState::SyncCrank => {
                self.rev_parity = 0;
                self.state = SyncState::SyncFull;
                Some(DecodeEvent::FullSyncAchieved)
            }
            SyncState::SyncFull => {
                // Re-pin parity every cycle; drift here would mean a missed
                // gap that position tracking should already have caught.
                self.rev_parity = 0;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHEEL: TriggerWheel = TriggerWheel {
        teeth: 12,
        missing: 1,
    };

    /// Golden-capture generator: edge timestamps for `revs` revolutions at
    /// a per-rev RPM given by `rpm_at(rev)`. Returns (crank, cam) streams;
    /// cam fires once per 720° early in even revs.
    fn capture(
        wheel: TriggerWheel,
        revs: u32,
        rpm_at: impl Fn(u32) -> f32,
    ) -> (Vec<u64>, Vec<u64>) {
        let mut crank = Vec::new();
        let mut cam = Vec::new();
        let mut t_us = 0u64;
        let physical = wheel.effective_edges_per_rev() as u32;
        for rev in 0..revs {
            let tooth_us = (60_000_000.0 / (rpm_at(rev) * wheel.teeth as f32)) as u64;
            for tooth in 0..physical {
                // Gap precedes tooth 0: it spans (missing + 1) pitches.
                t_us += if tooth == 0 {
                    tooth_us * (wheel.missing as u64 + 1)
                } else {
                    tooth_us
                };
                crank.push(t_us);
                if tooth == 2 && rev.is_multiple_of(2) {
                    cam.push(t_us + tooth_us / 2);
                }
            }
        }
        (crank, cam)
    }

    /// Feed both streams in timestamp order.
    fn run(decoder: &mut Decoder, crank: &[u64], cam: &[u64]) -> Vec<DecoderOutput> {
        let mut outputs = Vec::new();
        let mut cam_iter = cam.iter().copied().peekable();
        for &t in crank {
            while cam_iter.peek().is_some_and(|&c| c < t) {
                decoder.on_cam_edge(cam_iter.next().unwrap());
            }
            outputs.push(decoder.on_crank_edge(t));
        }
        outputs
    }

    #[test]
    fn cold_start_reaches_full_sync_within_three_revs() {
        let (crank, cam) = capture(WHEEL, 3, |_| 300.0);
        let mut decoder = Decoder::new(WHEEL, true);
        let outputs = run(&mut decoder, &crank, &cam);

        assert_eq!(decoder.state(), SyncState::SyncFull);
        assert!(decoder.spark_allowed());
        // No spark permission before full sync was reached.
        let full_at = outputs
            .iter()
            .position(|o| o.state == SyncState::SyncFull)
            .unwrap();
        assert!(
            outputs[..full_at]
                .iter()
                .all(|o| o.state < SyncState::SyncFull)
        );
    }

    #[test]
    fn angle_tracks_wheel_position_after_sync() {
        let (crank, cam) = capture(WHEEL, 4, |_| 1_200.0);
        let mut decoder = Decoder::new(WHEEL, true);
        run(&mut decoder, &crank, &cam);

        // Feed one more exact rev and check angles tooth by tooth.
        let last = *crank.last().unwrap();
        let tooth_us = 60_000_000 / (1_200 * 12) as u64;
        let mut t = last;
        let mut angles = Vec::new();
        for tooth in 0..WHEEL.effective_edges_per_rev() {
            t += if tooth == 0 { tooth_us * 2 } else { tooth_us };
            let out = decoder.on_crank_edge(t);
            assert_eq!(out.state, SyncState::SyncFull);
            angles.push(out.cycle_angle_deg % 360.0);
        }
        let pitch = 360.0 / 12.0;
        for (i, angle) in angles.iter().enumerate() {
            assert!(
                (angle - i as f32 * pitch).abs() < 0.01,
                "tooth {i}: {angle}"
            );
        }
    }

    #[test]
    fn rpm_estimate_tracks_capture_speed() {
        let (crank, cam) = capture(WHEEL, 4, |_| 1_200.0);
        let mut decoder = Decoder::new(WHEEL, true);
        run(&mut decoder, &crank, &cam);
        assert!(
            (decoder.rpm() - 1_200.0).abs() < 40.0,
            "rpm {}",
            decoder.rpm()
        );
    }

    #[test]
    fn survives_hard_acceleration() {
        // 300 → 3000 rpm over 12 revs — cranking into a first-start flare.
        let (crank, cam) = capture(WHEEL, 12, |rev| 300.0 + rev as f32 * 245.0);
        let mut decoder = Decoder::new(WHEEL, true);
        let outputs = run(&mut decoder, &crank, &cam);

        assert_eq!(decoder.state(), SyncState::SyncFull);
        let full_at = outputs
            .iter()
            .position(|o| o.state == SyncState::SyncFull)
            .unwrap();
        assert!(
            outputs[full_at..]
                .iter()
                .all(|o| !matches!(o.event, Some(DecodeEvent::Desync(_)))),
            "desync during acceleration"
        );
    }

    #[test]
    fn dropped_tooth_degrades_then_recovers() {
        let (crank, cam) = capture(WHEEL, 8, |_| 1_000.0);
        let mut decoder = Decoder::new(WHEEL, true);

        // Drop one mid-revolution edge from rev 5 (index: 4 revs × 11 + 5).
        let dropped_idx = 4 * 11 + 5;
        let mut faulty = crank.clone();
        faulty.remove(dropped_idx);

        let outputs = run(&mut decoder, &faulty, &cam);

        // The dropped tooth reads as a gap at the wrong position → desync.
        assert!(
            outputs
                .iter()
                .any(|o| o.event == Some(DecodeEvent::Desync(DesyncCause::GapAtWrongPosition))),
            "expected a desync event"
        );
        // …and spark was gated the moment sync degraded.
        let desync_at = outputs
            .iter()
            .position(|o| matches!(o.event, Some(DecodeEvent::Desync(_))))
            .unwrap();
        assert!(outputs[desync_at].state < SyncState::SyncFull);
        // Recovery: full sync again by the end of the capture.
        assert_eq!(decoder.state(), SyncState::SyncFull);
    }

    #[test]
    fn noise_edge_is_rejected_without_losing_sync() {
        let (crank, cam) = capture(WHEEL, 6, |_| 1_000.0);
        let mut decoder = Decoder::new(WHEEL, true);

        // Inject a spurious edge 200 µs after a mid-rev tooth in rev 4.
        let idx = 3 * 11 + 6;
        let mut noisy = crank.clone();
        noisy.insert(idx + 1, crank[idx] + 200);

        let outputs = run(&mut decoder, &noisy, &cam);
        assert!(
            outputs
                .iter()
                .any(|o| o.event == Some(DecodeEvent::NoiseRejected))
        );
        assert!(
            !outputs
                .iter()
                .any(|o| matches!(o.event, Some(DecodeEvent::Desync(_)))),
            "noise must not desync"
        );
        assert_eq!(decoder.state(), SyncState::SyncFull);
    }

    #[test]
    fn stall_loses_sync() {
        let (crank, cam) = capture(WHEEL, 4, |_| 1_000.0);
        let mut decoder = Decoder::new(WHEEL, true);
        run(&mut decoder, &crank, &cam);
        assert_eq!(decoder.state(), SyncState::SyncFull);

        // Next edge arrives a second later.
        let out = decoder.on_crank_edge(crank.last().unwrap() + 1_000_000);
        assert_eq!(out.event, Some(DecodeEvent::Desync(DesyncCause::Stall)));
        assert!(!decoder.spark_allowed());
    }

    #[test]
    fn cam_not_required_reaches_full_sync_without_cam() {
        let (crank, _) = capture(WHEEL, 3, |_| 600.0);
        let mut decoder = Decoder::new(WHEEL, false);
        run(&mut decoder, &crank, &[]);
        assert_eq!(decoder.state(), SyncState::SyncFull);
    }

    #[test]
    fn cam_required_stays_at_crank_sync_without_cam() {
        let (crank, _) = capture(WHEEL, 5, |_| 600.0);
        let mut decoder = Decoder::new(WHEEL, true);
        run(&mut decoder, &crank, &[]);
        assert_eq!(decoder.state(), SyncState::SyncCrank);
        assert!(!decoder.spark_allowed());
    }

    #[test]
    fn works_on_sixty_minus_two() {
        let wheel = TriggerWheel::sixty_minus_two();
        let (crank, cam) = capture(wheel, 4, |_| 900.0);
        let mut decoder = Decoder::new(wheel, true);
        run(&mut decoder, &crank, &cam);
        assert_eq!(decoder.state(), SyncState::SyncFull);
        assert!((decoder.rpm() - 900.0).abs() < 30.0);
    }
}
