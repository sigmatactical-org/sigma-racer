//! Subscribe to sigma-racer-vehicle and drive Slint properties from VSS snapshots.

use crate::log::log;
use sigma_instrumentation::{AlertItem, SigmaDashboard};
use sigma_racer_telemetry::TelemetryClient;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

use super::binding::apply_state;
use super::session::{RECONNECT_TICKS, Session, TELEMETRY_STALE, TICK};
use crate::connectivity;

/// Subscribe to sigma-racer-vehicle and drive Slint properties from VSS snapshots.
pub fn attach(ui: &SigmaDashboard) {
    let initial = TelemetryClient::connect();
    if initial.is_some() {
        log!("subscribed to vehicle telemetry");
    } else {
        log!("vehicle telemetry unavailable — retrying in background");
    }

    let session = Rc::new(RefCell::new(Session::new(initial)));
    connectivity::start(ui, &session);

    let ui_weak = ui.as_weak();
    slint::Timer::default().start(slint::TimerMode::Repeated, TICK, move || {
        let mut session = session.borrow_mut();
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        if session.client.is_none() {
            session.ticks_since_attempt += 1;
            if session.ticks_since_attempt >= RECONNECT_TICKS {
                session.ticks_since_attempt = 0;
                session.client = TelemetryClient::connect();
                if session.client.is_some() {
                    log!("subscribed to vehicle telemetry");
                }
            }
            ui.set_telemetry_live(false);
            return;
        }

        let messages: Vec<_> = session
            .client
            .as_ref()
            .map(|client| client.drain().collect())
            .unwrap_or_default();

        for msg in messages {
            // Any validated frame means the link is alive (incl. Heartbeat).
            session.last_msg_at = Some(std::time::Instant::now());
            if msg.msg == "Event" {
                // Anomaly alerts from the vehicle daemon's detector engine.
                session.ingest_alert(&msg);
            } else if let Some(data) = msg.vss_data() {
                session.state.apply_vss_map(data);
                // Per-signal freshness from the vehicle (replaces the old
                // global Vehicle.Service.SignalsLive wire value).
                session.state.apply_availability(msg.avail_data());
            }
        }

        // Link up is necessary but not sufficient: a 1 Hz heartbeat keeps the
        // IPC link "alive" even when the CAN bus is dead. Show live only when
        // the link is up AND the vehicle reports its core signals fresh.
        let link_live = session
            .last_msg_at
            .is_some_and(|t| t.elapsed() < TELEMETRY_STALE);
        session.state.signals_live = link_live && session.state.signals_live;
        // Always push state so the dial stays visible under a stale banner.
        apply_state(&ui, &session.state);
        apply_alerts(&ui, &session);
    });
}

/// Push the alert telltale + Alerts window rows from the session's tracker.
fn apply_alerts(ui: &SigmaDashboard, session: &Session) {
    let worst = session.alerts.worst_active();
    ui.set_alert_severity(SharedString::from(
        worst.map(|(_, sev)| sev.label()).unwrap_or(""),
    ));
    ui.set_alert_headline(SharedString::from(
        worst.map(|(id, _)| headline(id)).unwrap_or_default(),
    ));
    ui.set_alert_count(session.alerts.active().count() as i32);
    ui.set_alerts_latched(session.alerts.active().any(|slot| slot.latched));

    // Newest first; `active` greys out entries whose alert has since cleared.
    let rows: Vec<AlertItem> = session
        .alert_log
        .iter()
        .rev()
        .map(|entry| AlertItem {
            time: entry.time.clone().into(),
            severity: entry.severity.label().into(),
            message: entry.message.clone().into(),
            active: session.alerts.active().any(|slot| slot.id == entry.id),
        })
        .collect();
    ui.set_alerts(ModelRc::new(VecModel::from(rows)));
}

/// Rider-facing headline from an alert id: `coolant_overheat` → "COOLANT OVERHEAT".
fn headline(id: &str) -> String {
    id.replace('_', " ").to_uppercase()
}
