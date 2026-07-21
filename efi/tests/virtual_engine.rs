//! The virtual engine — full-stack host integration.
//!
//! Runs the entire control chain against a synthetic engine, end to end:
//!
//! ```text
//! replay plan ─▶ decoder ─▶ scheduler ─▶ fueling/dwell
//!                   │            ▲
//!                   └── RbW monitor gates arming ──┘
//! ```
//!
//! This is the closest thing to a first start that software alone can
//! produce: the replay generator (the bench rig's signal source) drives the
//! decoder to sync, the scheduler arms cylinder events, fueling computes
//! pulse widths, and the RbW monitor gates everything — with armed times
//! checked against interpolated ground truth from the edge stream itself,
//! through a hard cranking flare.
//!
//! When the boards arrive, the only untested layer left is electronics.

use sigma_racer_efi::fuel::{
    PLACEHOLDER_INJECTOR, PLACEHOLDER_VE, SpeedDensityInputs, base_pulse_ms,
};
use sigma_racer_efi::replay::{ReplayPlan, Step};
use sigma_racer_efi::scheduler::{ArmedBuf, EventId, Scheduler, deg_per_us_from_rpm};
use sigma_racer_efi::throttle::{RbwCommand, RbwConfig, RbwInputs, RbwMonitor, RbwState};
use sigma_racer_efi::trigger::TriggerWheel;
use sigma_racer_efi::trigger::{Decoder, SyncState};

const WHEEL: TriggerWheel = TriggerWheel {
    teeth: 12,
    missing: 1,
};
const PITCH: f32 = 30.0;
const CYCLE: f32 = 720.0;
const CYL_CC: f32 = 890.0 / 3.0;

/// Fire events strictly inside tooth windows (not on boundaries).
fn fire_angle(cyl: u8) -> f32 {
    cyl as f32 * 240.0 + 15.0
}
fn inj_close_angle(cyl: u8) -> f32 {
    (cyl as f32 * 240.0 + 315.0) % CYCLE
}

/// Percent → volts through the placeholder RbW cal.
fn v(pct: f32) -> f32 {
    0.5 + pct / 100.0 * 4.0
}

fn healthy_rbw(t_us: u64) -> RbwInputs {
    RbwInputs {
        t_us,
        app_a_v: v(20.0),
        app_b_v: v(20.0),
        tps_a_v: v(20.0),
        tps_b_v: v(20.0),
        commanded_pct: 20.0,
    }
}

/// One armed event with its ground-truth target in unwrapped angle.
struct ArmRecord {
    id: EventId,
    t_us: u64,
    target_unwrapped_deg: f64,
}

struct VirtualRun {
    /// (t_us, unwrapped_deg) per synced tooth — the ground-truth curve.
    edges: Vec<(u64, f64)>,
    arms: Vec<ArmRecord>,
    monitor_state: RbwState,
    decoder_state: SyncState,
    final_rpm: f32,
    /// Pulse widths computed at each injection arm, ms.
    pulses_ms: Vec<f32>,
}

/// Drive the full stack over a plan. `fault_after_us` flips APP B to a
/// disagreeing value from that timestamp on (None = healthy throughout).
fn run(plans: &[ReplayPlan], fault_after_us: Option<u64>) -> VirtualRun {
    let mut decoder = Decoder::new(WHEEL, true);
    let mut scheduler = Scheduler::new(CYCLE);
    for cyl in 0..3u8 {
        scheduler
            .set_event(EventId::Fire(cyl), fire_angle(cyl))
            .unwrap();
        scheduler
            .set_event(EventId::InjClose(cyl), inj_close_angle(cyl))
            .unwrap();
    }
    let mut monitor = RbwMonitor::new(RbwConfig::default());

    let mut t_us = 0u64;
    let mut edges: Vec<(u64, f64)> = Vec::new();
    let mut arms: Vec<ArmRecord> = Vec::new();
    let mut pulses = Vec::new();
    let mut unwrapped = 0.0f64;
    let mut last_cycle_angle: Option<f32> = None;

    for plan in plans {
        for step in plan.steps() {
            t_us += step.delay_us() as u64;
            match step {
                Step::Cam { .. } => {
                    decoder.on_cam_edge(t_us);
                }
                Step::Crank { .. } => {
                    let out = decoder.on_crank_edge(t_us);

                    // Safety tick rides the tooth cadence in this virtual
                    // build (the real firmware runs it on its own timer).
                    let mut rbw_in = healthy_rbw(t_us);
                    if fault_after_us.is_some_and(|after| t_us >= after) {
                        rbw_in.app_b_v = v(60.0);
                    }
                    let rbw_ok = matches!(monitor.evaluate(&rbw_in), RbwCommand::Normal { .. });

                    if out.state != SyncState::SyncFull {
                        last_cycle_angle = None;
                        continue;
                    }

                    // Unwrap the decoder's cycle angle into a monotonic axis.
                    if let Some(prev) = last_cycle_angle {
                        let mut delta = out.cycle_angle_deg - prev;
                        if delta < 0.0 {
                            delta += CYCLE;
                        }
                        unwrapped += delta as f64;
                    }
                    last_cycle_angle = Some(out.cycle_angle_deg);
                    edges.push((t_us, unwrapped));

                    // Gate: spark/fuel arming requires sync AND an armed
                    // safety monitor — the two independent permissions.
                    if !(decoder.spark_allowed() && rbw_ok) {
                        continue;
                    }

                    let position = ((out.cycle_angle_deg % 360.0) / PITCH) as u8;
                    let window = if position == WHEEL.effective_edges_per_rev() - 1 {
                        PITCH * (WHEEL.missing as f32 + 1.0)
                    } else {
                        PITCH
                    };
                    let dpu = deg_per_us_from_rpm(out.rpm);
                    let mut buf = ArmedBuf::new();
                    scheduler.on_tooth(t_us, out.cycle_angle_deg, dpu, window, &mut buf);

                    for armed in buf.iter() {
                        let angle = match armed.id {
                            EventId::Fire(c) => fire_angle(c),
                            EventId::InjClose(c) => inj_close_angle(c),
                            _ => continue,
                        };
                        let mut ahead = angle - out.cycle_angle_deg;
                        if ahead <= 0.0 {
                            ahead += CYCLE;
                        }
                        arms.push(ArmRecord {
                            id: armed.id,
                            t_us: armed.t_us,
                            target_unwrapped_deg: unwrapped + ahead as f64,
                        });
                        if matches!(armed.id, EventId::InjClose(_)) {
                            pulses.push(base_pulse_ms(
                                &PLACEHOLDER_VE,
                                &PLACEHOLDER_INJECTOR,
                                &SpeedDensityInputs {
                                    rpm: out.rpm,
                                    map_kpa: 45.0, // steady part throttle
                                    iat_c: 25.0,
                                    displacement_per_cyl_cc: CYL_CC,
                                    afr_target: 14.7,
                                    vbatt: 13.5,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }

    VirtualRun {
        edges,
        arms,
        monitor_state: monitor.state(),
        decoder_state: decoder.state(),
        final_rpm: decoder.rpm(),
        pulses_ms: pulses,
    }
}

/// Interpolate ground-truth time for an unwrapped angle from the edge curve.
fn truth_time_us(edges: &[(u64, f64)], target_deg: f64) -> Option<f64> {
    let after = edges.iter().position(|&(_, a)| a >= target_deg)?;
    if after == 0 {
        return None;
    }
    let (t0, a0) = edges[after - 1];
    let (t1, a1) = edges[after];
    if a1 <= a0 {
        return None;
    }
    Some(t0 as f64 + (target_deg - a0) / (a1 - a0) * (t1 - t0) as f64)
}

fn flare_and_idle() -> [ReplayPlan; 2] {
    [
        ReplayPlan::sweep(WHEEL, 300.0, 1_200.0, 8),
        ReplayPlan::constant(WHEEL, 1_200.0, 24),
    ]
}

#[test]
fn virtual_engine_starts_syncs_and_fires_on_angle() {
    let run = run(&flare_and_idle(), None);

    assert_eq!(run.decoder_state, SyncState::SyncFull);
    assert_eq!(run.monitor_state, RbwState::Armed);
    assert!((run.final_rpm - 1_200.0).abs() < 40.0);
    assert!(!run.arms.is_empty(), "no events armed");

    // Every armed event lands on its crank angle within half a local tooth.
    let mut checked = 0;
    for arm in &run.arms {
        let Some(truth) = truth_time_us(&run.edges, arm.target_unwrapped_deg) else {
            continue; // target beyond the capture tail
        };
        // Local tooth period around the event.
        let idx = run
            .edges
            .iter()
            .position(|&(_, a)| a >= arm.target_unwrapped_deg)
            .unwrap();
        let local_tooth_us = (run.edges[idx].0 - run.edges[idx - 1].0) as f64;
        let err = (arm.t_us as f64 - truth).abs();
        assert!(
            err <= local_tooth_us / 2.0,
            "{:?}: err {err:.0} µs vs tooth {local_tooth_us:.0} µs",
            arm.id
        );
        checked += 1;
    }
    assert!(checked > 30, "only {checked} events verified");
}

#[test]
fn virtual_engine_arms_three_of_each_per_cycle() {
    let run = run(&flare_and_idle(), None);

    // Count per full 720° span of the unwrapped axis, skipping the first
    // and last partial cycles.
    let total_deg = run.edges.last().unwrap().1;
    let full_cycles = (total_deg / CYCLE as f64) as u64;
    for cycle in 1..full_cycles.saturating_sub(1) {
        let lo = cycle as f64 * CYCLE as f64;
        let hi = lo + CYCLE as f64;
        let fires = run
            .arms
            .iter()
            .filter(|a| {
                matches!(a.id, EventId::Fire(_))
                    && a.target_unwrapped_deg >= lo
                    && a.target_unwrapped_deg < hi
            })
            .count();
        assert_eq!(fires, 3, "cycle {cycle}: {fires} fires");
    }
}

#[test]
fn virtual_engine_pulse_widths_are_physical() {
    let run = run(&flare_and_idle(), None);
    assert!(!run.pulses_ms.is_empty());
    for &pw in &run.pulses_ms {
        assert!(
            pw > 1.0 && pw < 10.0,
            "pulse {pw} ms out of plausible range"
        );
    }
}

#[test]
fn rbw_fault_stops_all_arming_while_engine_keeps_spinning() {
    // Fault at ~60 % through the run: APP disagreement.
    let healthy = run(&flare_and_idle(), None);
    let total_t = healthy.edges.last().unwrap().0;
    let fault_at = total_t * 6 / 10;

    let faulted = run(&flare_and_idle(), Some(fault_at));

    assert!(matches!(faulted.monitor_state, RbwState::Tripped(_)));
    // Decoder keeps tracking — the engine is still turning.
    assert_eq!(faulted.decoder_state, SyncState::SyncFull);
    // No event armed after the trip landed (hold window grace).
    let hold_us = 20_000;
    let late_arms = faulted
        .arms
        .iter()
        .filter(|a| a.t_us > fault_at + hold_us + 1_000)
        .count();
    assert_eq!(late_arms, 0, "{late_arms} events armed after RbW trip");
    // And it armed plenty before the fault.
    assert!(faulted.arms.len() > 20);
}
