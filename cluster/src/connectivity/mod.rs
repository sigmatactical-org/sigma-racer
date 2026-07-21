//! Live Connectivity window: BlueZ + connman over D-Bus, driven by face buttons.

mod agent;
mod backends;
mod bluez;
mod connman;
mod nav;

use backends::Backends;
use sigma_instrumentation::SigmaDashboard;
use sigma_instrumentation::connectivity::{Controller, WINDOW as CONN_WINDOW};
use slint::ComponentHandle;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use crate::telemetry::session::Session;

/// Wire Connectivity polling and face-button navigation (incl. in-window focus).
pub fn start(ui: &SigmaDashboard, session: &Rc<RefCell<Session>>) {
    let ctrl = Rc::new(RefCell::new(Controller::new()));
    let backends = Rc::new(Backends::open());

    {
        let mut c = ctrl.borrow_mut();
        backends.refresh(&mut c.snap);
        c.paint(ui);
    }

    nav::wire_nav(ui, session, ctrl.clone(), backends.clone());

    let ui_weak = ui.as_weak();
    let ctrl_tick = ctrl;
    let backends_tick = backends;
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        Duration::from_secs(2),
        move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            let mut c = ctrl_tick.borrow_mut();
            backends_tick.refresh(&mut c.snap);
            if ui.get_current_window() == CONN_WINDOW {
                c.paint(&ui);
            }
        },
    );
    std::mem::forget(timer);
}
