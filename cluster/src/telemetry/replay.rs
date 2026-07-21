//! Replay a recorded CAN log into the dashboard.
//!
//! Reads a `candump -L` text log — either the baked-in sample ride or a file
//! named by `CLUSTER_REPLAY_LOG` — and plays the frames back through the real
//! DBC decoder with their original inter-frame timing, looping forever. Lets you
//! reproduce a captured ride deterministically with no CAN hardware.

use crate::log::log;
use sigma_instrumentation::SigmaDashboard;
use sigma_racer_telemetry::can::{decode_frame, parse_candump};
use sigma_racer_telemetry::state::VehicleState;
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use super::binding::apply_state;
use super::session::{Session, TICK};
use crate::connectivity;

/// Baked-in sample so `CLUSTER_TELEMETRY_SOURCE=replay` works with no arguments.
const SAMPLE_LOG: &str = include_str!("../../testdata/sample-ride.log");

/// Replay a candump log, driving the dashboard from decoded frames.
pub fn attach(ui: &SigmaDashboard) {
    let (source, text) = match std::env::var("CLUSTER_REPLAY_LOG") {
        Ok(path) => match std::fs::read_to_string(&path) {
            Ok(text) => (path, text),
            Err(err) => {
                log!("replay {path}: {err} — using baked sample");
                ("baked sample".to_owned(), SAMPLE_LOG.to_owned())
            }
        },
        Err(_) => ("baked sample".to_owned(), SAMPLE_LOG.to_owned()),
    };

    let frames = parse_candump(&text);
    if frames.is_empty() {
        log!("replay log '{source}' had no usable frames");
        ui.set_telemetry_live(false);
        return;
    }
    let span = frames.last().map(|f| f.at).unwrap_or(0.0);
    log!(
        "replaying {} frames ({:.1}s) from {source}",
        frames.len(),
        span
    );

    let session = Rc::new(RefCell::new(Session::new(None)));
    connectivity::start(ui, &session);

    let frames = Rc::new(frames);
    let start = Rc::new(RefCell::new(Instant::now()));
    let cursor = Rc::new(RefCell::new(0usize));
    let ui_weak = ui.as_weak();

    slint::Timer::default().start(slint::TimerMode::Repeated, TICK, move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        let mut session = session.borrow_mut();
        let mut cursor = cursor.borrow_mut();
        let mut start = start.borrow_mut();

        // Loop: once past the end, rewind and start the ride over from idle.
        if *cursor >= frames.len() {
            *cursor = 0;
            *start = Instant::now();
            session.state = VehicleState::idle();
        }

        let now = start.elapsed().as_secs_f64();
        while *cursor < frames.len() && frames[*cursor].at <= now {
            let f = &frames[*cursor];
            decode_frame(f.id, &f.data, &mut session.state);
            *cursor += 1;
        }
        session.state.refresh_derived();
        session.state.signals_live = true;

        apply_state(&ui, &session.state);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_racer_telemetry::can::decode_frame;

    #[test]
    fn baked_sample_is_replayable() {
        let frames = parse_candump(SAMPLE_LOG);
        assert!(frames.len() > 100, "sample should have many frames");
        assert!(frames.windows(2).all(|w| w[0].at <= w[1].at));
    }

    #[test]
    fn baked_sample_decodes_into_a_moving_ride() {
        let frames = parse_candump(SAMPLE_LOG);
        let mut state = VehicleState::idle();
        let mut max_speed = 0.0f32;
        let mut max_rpm = 0.0f32;
        let mut max_gear = 0i8;
        for f in &frames {
            decode_frame(f.id, &f.data, &mut state);
            state.refresh_derived();
            max_speed = max_speed.max(state.speed);
            max_rpm = max_rpm.max(state.rpm);
            max_gear = max_gear.max(state.gear);
        }
        assert!(
            max_speed > 100.0,
            "speed should reach highway pace, got {max_speed}"
        );
        assert!(max_rpm > 8_000.0, "rpm should rev out, got {max_rpm}");
        assert!(max_gear >= 5, "should shift up, got gear {max_gear}");
    }
}
