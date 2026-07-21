//! Telemetry session state for the cluster UI.

use sigma_racer_telemetry::anomaly::{AnomalyEngine, Edge, Severity};
use sigma_racer_telemetry::{Message, TelemetryClient, VehicleState};

/// UI tick period (~30 Hz).
pub const TICK: std::time::Duration = std::time::Duration::from_millis(33);
/// Retry `connect()` roughly once per second while the service is unavailable.
pub const RECONNECT_TICKS: u32 = 30;
/// Hide live values when no validated telemetry arrived recently.
pub const TELEMETRY_STALE: std::time::Duration = std::time::Duration::from_secs(2);
/// Newest alert events kept for the Alerts window.
const ALERT_LOG_CAP: usize = 8;

/// One received anomaly event, formatted for the Alerts window.
pub struct AlertEntry {
    pub id: String,
    pub time: String,
    pub severity: Severity,
    pub message: String,
}

/// Mutable per-run UI state shared between telemetry and navigation.
pub struct Session {
    pub client: Option<TelemetryClient>,
    pub state: VehicleState,
    pub ticks_since_attempt: u32,
    pub last_msg_at: Option<std::time::Instant>,
    pub current_window: i32,
    /// Consumer-side alert tracker: the bike detects, the cluster displays.
    pub alerts: AnomalyEngine,
    pub alert_log: Vec<AlertEntry>,
}

impl Session {
    /// Start a session on window 0 with an optional IPC client.
    pub fn new(client: Option<TelemetryClient>) -> Self {
        Self {
            client,
            state: VehicleState::idle(),
            ticks_since_attempt: 0,
            last_msg_at: None,
            current_window: 0,
            alerts: AnomalyEngine::sigma_defaults(),
            alert_log: Vec::new(),
        }
    }

    /// Merge an `Event` message from the vehicle daemon into the alert state.
    pub fn ingest_alert(&mut self, msg: &Message) {
        let Some(ev) = self.alerts.ingest_event(msg) else {
            return;
        };
        // Cleared edges only update slot state; the log keeps raise entries.
        if ev.edge == Edge::Raised {
            if self.alert_log.len() >= ALERT_LOG_CAP {
                self.alert_log.remove(0);
            }
            self.alert_log.push(AlertEntry {
                id: ev.id.clone(),
                time: time_of_day(ev.ts_ms),
                severity: ev.severity,
                message: ev.message,
            });
        }
    }

    /// Acknowledge every latched alert (Select on the Alerts window).
    pub fn ack_latched_alerts(&mut self) {
        let latched: Vec<&'static str> = self
            .alerts
            .active()
            .filter(|slot| slot.latched)
            .map(|slot| slot.id)
            .collect();
        for id in latched {
            self.alerts.ack(id);
        }
    }
}

/// HH:MM:SS (UTC) from epoch milliseconds.
fn time_of_day(ts_ms: i64) -> String {
    let ms_of_day = ts_ms.rem_euclid(86_400_000);
    let (h, rem) = (ms_of_day / 3_600_000, ms_of_day % 3_600_000);
    let (m, s) = (rem / 60_000, rem % 60_000 / 1_000);
    format!("{h:02}:{m:02}:{s:02}")
}
