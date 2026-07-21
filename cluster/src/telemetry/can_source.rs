//! Live SocketCAN receive source (bench/test).
//!
//! Reads CAN frames straight off a SocketCAN interface (`vcan0`, `can1`, …),
//! decodes them through the real M7 DBC codec, and drives the dashboard — the
//! same decode path the vehicle daemon uses, but rendered by the actual cluster
//! UI. Feed it from the M7 gateway, `cansend`, a `candump` replay, or a real bus.
//!
//! Enabled by the `can-socket` cargo feature and selected at runtime with
//! `CLUSTER_TELEMETRY_SOURCE=can` (interface via `CLUSTER_CAN_IFACE`, default
//! `vcan0`).

use crate::log::log;
use sigma_instrumentation::SigmaDashboard;
use sigma_racer_telemetry::can::decode_frame;
use slint::ComponentHandle;
use socketcan::frame::CanDataFrame;
use socketcan::{CanFrame, CanSocket, EmbeddedFrame, Frame, Socket};
use std::cell::RefCell;
use std::io::ErrorKind;
use std::rc::Rc;
use std::time::{Duration, Instant};

use super::binding::apply_state;
use super::session::{Session, TICK};
use crate::connectivity;

/// Treat the bus as live only while frames keep arriving.
const CAN_STALE: Duration = Duration::from_millis(500);

/// Open the interface and drive the dashboard from decoded frames.
pub fn attach(ui: &SigmaDashboard) {
    let iface = std::env::var("CLUSTER_CAN_IFACE").unwrap_or_else(|_| "vcan0".to_owned());

    let socket = match CanSocket::open(&iface).and_then(|s| {
        s.set_nonblocking(true)?;
        Ok(s)
    }) {
        Ok(socket) => socket,
        Err(err) => {
            log!(
                "open CAN interface '{iface}': {err} — is it up? \
                 (see scripts/vcan-up.sh)"
            );
            ui.set_telemetry_live(false);
            return;
        }
    };
    log!("receiving CAN on {iface}");

    let session = Rc::new(RefCell::new(Session::new(None)));
    connectivity::start(ui, &session);

    let socket = Rc::new(socket);
    let last_frame_at: Rc<RefCell<Option<Instant>>> = Rc::new(RefCell::new(None));
    let ui_weak = ui.as_weak();

    slint::Timer::default().start(slint::TimerMode::Repeated, TICK, move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        let mut session = session.borrow_mut();
        let mut last_frame_at = last_frame_at.borrow_mut();

        // Drain everything queued since the last tick.
        loop {
            match socket.read_frame() {
                Ok(CanFrame::Data(frame)) => {
                    if handle_frame(&frame, &mut session) {
                        *last_frame_at = Some(Instant::now());
                    }
                }
                Ok(_) => continue, // remote/error frames — ignore
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => {
                    log!("CAN read: {err}");
                    break;
                }
            }
        }

        let live = last_frame_at.is_some_and(|t| t.elapsed() < CAN_STALE);
        session.state.refresh_derived();
        session.state.signals_live = live;
        apply_state(&ui, &session.state);
    });
}

/// Decode one data frame into the session state. Returns whether it decoded.
fn handle_frame(frame: &CanDataFrame, session: &mut Session) -> bool {
    let id = frame.raw_id();
    let data = frame.data();
    let len = data.len().min(8);
    if decode_frame(id, &data[..len], &mut session.state) {
        true
    } else {
        log!("ignore undecodable CAN frame 0x{id:03X}");
        false
    }
}
