//! Face-button navigation: window stepping and in-window focus routing.

use super::backends::Backends;
use crate::telemetry::session::{Session, TICK};
use sigma_instrumentation::connectivity::{BackResult, Controller, WINDOW as CONN_WINDOW};
use sigma_instrumentation::updates::{self as updates_nav, WINDOW as UPDATES_WINDOW};
use sigma_instrumentation::{SigmaDashboard, camera, windows};
use slint::ComponentHandle;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Hook the four face buttons (next/prev/back/select) into window navigation
/// and the Connectivity/Updates in-window focus models.
pub(super) fn wire_nav(
    ui: &SigmaDashboard,
    session: &Rc<RefCell<Session>>,
    ctrl: Rc<RefCell<Controller>>,
    backends: Rc<Backends>,
) {
    let stopped = Rc::new(Cell::new(true));

    {
        let session = session.clone();
        let stopped = stopped.clone();
        let ctrl = ctrl.clone();
        let ui_weak = ui.as_weak();
        ui.on_nav_next(move || {
            if let Some(ui) = ui_weak.upgrade() {
                step_or_focus(&ui, &session, &ctrl, stopped.get(), 1);
            }
        });
    }
    {
        let session = session.clone();
        let stopped = stopped.clone();
        let ctrl = ctrl.clone();
        let ui_weak = ui.as_weak();
        ui.on_nav_prev(move || {
            if let Some(ui) = ui_weak.upgrade() {
                step_or_focus(&ui, &session, &ctrl, stopped.get(), -1);
            }
        });
    }
    {
        let session = session.clone();
        let ctrl = ctrl.clone();
        let ui_weak = ui.as_weak();
        ui.on_nav_back(move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            let mut session = session.borrow_mut();
            if session.current_window == CONN_WINDOW {
                let mut c = ctrl.borrow_mut();
                match c.menu.back() {
                    BackResult::Stay => {
                        c.paint(&ui);
                        return;
                    }
                    BackResult::LeaveWindow => {
                        c.menu.reset();
                    }
                }
            }
            if session.current_window == UPDATES_WINDOW {
                updates_nav::reset_focus(&ui);
            }
            session.current_window = 0;
            ui.set_current_window(0);
        });
    }
    {
        let session = session.clone();
        let ctrl = ctrl.clone();
        let backends = backends.clone();
        let ui_weak = ui.as_weak();
        ui.on_nav_select(move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            let win = session.borrow().current_window;
            if win == CONN_WINDOW {
                let mut c = ctrl.borrow_mut();
                let snap = c.snap.clone();
                if let Some(action) = c.menu.select(&snap) {
                    backends.run_action(action, &mut c.snap);
                }
                c.paint(&ui);
            } else if win == UPDATES_WINDOW {
                updates_nav::activate_focused(&ui);
            } else if win == camera::WINDOW {
                camera::toggle_feed(&ui);
            } else if win == windows::ALERTS {
                session.borrow_mut().ack_latched_alerts();
            }
        });
    }

    let session_tick = session.clone();
    let stopped_tick = stopped;
    let ui_weak = ui.as_weak();
    slint::Timer::default().start(slint::TimerMode::Repeated, TICK, move || {
        if let Some(ui) = ui_weak.upgrade() {
            stopped_tick.set(ui.get_speed() == 0);
            ui.set_nav_blocked_hint(
                !stopped_tick.get() && session_tick.borrow().current_window > windows::PANEL_MAX,
            );
        }
    });
}

fn max_window(stopped: bool) -> i32 {
    if stopped {
        windows::COUNT - 1
    } else {
        windows::PANEL_MAX
    }
}

fn step_or_focus(
    ui: &SigmaDashboard,
    session: &Rc<RefCell<Session>>,
    ctrl: &Rc<RefCell<Controller>>,
    stopped: bool,
    delta: i32,
) {
    let mut session = session.borrow_mut();
    if session.current_window == CONN_WINDOW {
        let mut c = ctrl.borrow_mut();
        let snap = c.snap.clone();
        if let Some(leave) = c.menu.move_focus(&snap, delta) {
            let next = (CONN_WINDOW + leave).clamp(0, max_window(stopped));
            c.menu.reset();
            session.current_window = next;
            ui.set_current_window(next);
            if next == UPDATES_WINDOW {
                updates_nav::reset_focus(ui);
            }
            return;
        }
        c.paint(ui);
        return;
    }

    if session.current_window == UPDATES_WINDOW {
        if let Some(leave) = updates_nav::move_focus(ui, delta) {
            let next = (UPDATES_WINDOW + leave).clamp(0, max_window(stopped));
            updates_nav::reset_focus(ui);
            session.current_window = next;
            ui.set_current_window(next);
            if next == CONN_WINDOW {
                ctrl.borrow_mut().menu.reset();
                ctrl.borrow_mut().paint(ui);
            }
            return;
        }
        return;
    }

    let next = (session.current_window + delta).clamp(0, max_window(stopped));
    if next == CONN_WINDOW {
        ctrl.borrow_mut().menu.reset();
    }
    if next == UPDATES_WINDOW {
        updates_nav::reset_focus(ui);
    }
    session.current_window = next;
    ui.set_current_window(next);
    if next == CONN_WINDOW {
        ctrl.borrow_mut().paint(ui);
    }
}
