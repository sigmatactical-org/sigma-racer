//! The bike's durable store: a SQLite database holding the maintenance log,
//! error history, and provisioned metadata (schedule version, engine hours).
//!
//! This is the bike's own record of what has happened to it. All access is from
//! the single-threaded daemon loop (telemetry sampling, query serving, and
//! error recording all run there), so the connection needs no locking.

use chrono::{SecondsFormat, TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use sigma_racer_telemetry::{AnomalyEvent, Edge, MaintenanceLogEntry};

/// Default on-bike database (under the systemd `StateDirectory=`).
pub const DEFAULT_DB_PATH: &str = "/var/lib/sigma-racer-wingman/wingman.db";

/// Schema applied on open (idempotent).
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS maintenance_log (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    item_id      TEXT NOT NULL,
    performed_at TEXT NOT NULL,
    odometer_km  REAL NOT NULL,
    engine_hours REAL,
    note         TEXT
);
CREATE TABLE IF NOT EXISTS error_history (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          TEXT NOT NULL,
    event       TEXT NOT NULL,
    edge        TEXT NOT NULL,
    severity    TEXT,
    vss         TEXT,
    message     TEXT NOT NULL,
    odometer_km REAL
);
CREATE INDEX IF NOT EXISTS idx_error_history_ts ON error_history (ts);
";

/// One recorded error-history row (read-back form). Read path for the
/// forthcoming error-history query; the daemon writes history today and this is
/// exercised by tests, so allow it ahead of that consumer landing.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorEntry {
    pub ts: String,
    pub event: String,
    pub edge: String,
    pub severity: Option<String>,
    pub vss: Option<String>,
    pub message: String,
    pub odometer_km: Option<f64>,
}

/// The bike's maintenance / diagnostics database.
pub struct MaintenanceDb {
    conn: Connection,
}

impl MaintenanceDb {
    /// Open (creating if absent) and apply the schema.
    pub fn open(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("open db {path}: {e}"))?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| format!("db schema: {e}"))?;
        Ok(Self { conn })
    }

    /// A consistent snapshot of the whole database as bytes, for shipping to the
    /// shop tool. `VACUUM INTO` writes a defragmented, standalone copy that is
    /// safe to read even while the live DB is in use.
    pub fn snapshot_bytes(&self) -> Result<Vec<u8>, String> {
        let tmp = std::env::temp_dir().join(format!("wingman-snapshot-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&tmp);
        // The path is process-local and controlled; escape quotes defensively.
        let target = tmp.display().to_string().replace('\'', "''");
        self.conn
            .execute_batch(&format!("VACUUM INTO '{target}'"))
            .map_err(|e| format!("snapshot: {e}"))?;
        let bytes = std::fs::read(&tmp).map_err(|e| format!("read snapshot: {e}"))?;
        let _ = std::fs::remove_file(&tmp);
        Ok(bytes)
    }

    /// Persist the current odometer reading into `meta` so the pulled database
    /// is self-contained for a shop-tool audit (odometer is a live signal, not
    /// otherwise stored).
    pub fn set_odometer(&self, odometer_km: f64) -> Result<(), String> {
        self.meta_set("odometer_km", &format!("{odometer_km}"))
    }

    /// Whether the bike has a provisioned schedule version.
    pub fn is_provisioned(&self) -> bool {
        self.meta_get("schedule_version")
            .ok()
            .flatten()
            .is_some_and(|v| !v.is_empty())
    }

    /// The provisioned schedule version, if any.
    pub fn schedule_version(&self) -> String {
        self.meta_get("schedule_version")
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    /// Count of maintenance-log rows.
    pub fn maintenance_count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM maintenance_log", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// Record a completed service in the maintenance log.
    pub fn record_service(&self, entry: &MaintenanceLogEntry) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO maintenance_log (item_id, performed_at, odometer_km, engine_hours, note) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    entry.item_id,
                    entry.performed_at,
                    entry.odometer_km,
                    entry.engine_hours,
                    entry.note,
                ],
            )
            .map(|_| ())
            .map_err(|e| format!("record service: {e}"))
    }

    /// Record an anomaly transition in the error history.
    pub fn record_error(&self, ev: &AnomalyEvent, odometer_km: f64) -> Result<(), String> {
        let ts = Utc
            .timestamp_millis_opt(ev.ts_ms)
            .single()
            .unwrap_or_else(Utc::now)
            .to_rfc3339_opts(SecondsFormat::Millis, true);
        let edge = match ev.edge {
            Edge::Raised => "raised",
            Edge::Cleared => "cleared",
        };
        self.conn
            .execute(
                "INSERT INTO error_history (ts, event, edge, severity, vss, message, odometer_km) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    ts,
                    ev.id,
                    edge,
                    ev.severity.label(),
                    ev.vss,
                    ev.message,
                    odometer_km,
                ],
            )
            .map(|_| ())
            .map_err(|e| format!("record error: {e}"))
    }

    /// The most recent error-history rows, newest first. Read path for the
    /// forthcoming error-history query (writes land today via [`Self::record_error`]).
    #[allow(dead_code)]
    pub fn recent_errors(&self, limit: u32) -> Result<Vec<ErrorEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT ts, event, edge, severity, vss, message, odometer_km \
                 FROM error_history ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([limit], |r| {
                Ok(ErrorEntry {
                    ts: r.get(0)?,
                    event: r.get(1)?,
                    edge: r.get(2)?,
                    severity: r.get(3)?,
                    vss: r.get(4)?,
                    message: r.get(5)?,
                    odometer_km: r.get(6)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Set the provisioned schedule version.
    pub fn set_schedule_version(&self, version: &str) -> Result<(), String> {
        self.meta_set("schedule_version", version)
    }

    /// Seed a plausible store for sim/demo bikes on first run, so a dev stack
    /// answers queries with real-looking history. No-op if already provisioned.
    pub fn seed_demo(&self, version: &str) -> Result<(), String> {
        if self.is_provisioned() || self.maintenance_count() > 0 {
            return Ok(());
        }
        self.set_schedule_version(version)?;
        self.meta_set("engine_hours", "310.5")?;
        for (item_id, note) in [
            ("engine-oil", Some("break-in service")),
            ("final-drive", None),
        ] {
            self.record_service(&MaintenanceLogEntry {
                item_id: item_id.into(),
                performed_at: "2025-09-01T09:00:00.000Z".into(),
                odometer_km: 6_000.0,
                engine_hours: Some(150.0),
                note: note.map(str::to_owned),
            })?;
        }
        Ok(())
    }

    fn meta_get(&self, key: &str) -> Result<Option<String>, String> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()
            .map_err(|e| format!("meta {key}: {e}"))
    }

    fn meta_set(&self, key: &str, value: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT INTO meta (key, value) VALUES (?1, ?2) \
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            )
            .map(|_| ())
            .map_err(|e| format!("meta set {key}: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sigma_racer_telemetry::Severity;
    use sigma_racer_telemetry::anomaly::Category;

    fn mem_db() -> MaintenanceDb {
        MaintenanceDb::open(":memory:").expect("open in-memory db")
    }

    #[test]
    fn seed_is_idempotent() {
        let db = mem_db();
        db.seed_demo("2026.1").unwrap();
        assert_eq!(db.schedule_version(), "2026.1");
        assert_eq!(db.maintenance_count(), 2);
        db.seed_demo("2026.1").unwrap();
        assert_eq!(db.maintenance_count(), 2);
    }

    #[test]
    fn snapshot_is_a_readable_sqlite_db() {
        // A file-backed DB so VACUUM INTO has real content to copy.
        let dir = std::env::temp_dir().join(format!("wingman-db-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wingman.db");
        let _ = std::fs::remove_file(&path);
        let db = MaintenanceDb::open(path.to_str().unwrap()).unwrap();
        db.seed_demo("2026.1").unwrap();
        db.set_odometer(12_345.0).unwrap();

        let bytes = db.snapshot_bytes().unwrap();
        assert!(bytes.starts_with(b"SQLite format 3\0"), "not a sqlite file");

        // Re-open the snapshot and confirm the maintenance log survived.
        let restored =
            std::env::temp_dir().join(format!("wingman-restored-{}.db", std::process::id()));
        std::fs::write(&restored, &bytes).unwrap();
        let reopened = Connection::open(&restored).unwrap();
        let n: i64 = reopened
            .query_row("SELECT COUNT(*) FROM maintenance_log", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 2);
        let odo: String = reopened
            .query_row("SELECT value FROM meta WHERE key='odometer_km'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(odo, "12345");
        let _ = std::fs::remove_file(&restored);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn records_error_history() {
        let db = mem_db();
        let ev = AnomalyEvent {
            id: "coolant_overheat".into(),
            severity: Severity::Critical,
            category: Category::StateBased,
            edge: Edge::Raised,
            ts_ms: 1_700_000_000_000,
            message: "Coolant 118 °C".into(),
            vss: "Vehicle.OBD.CoolantTemperature".into(),
            value: serde_json::Value::from(118),
        };
        db.record_error(&ev, 12_345.0).unwrap();
        let rows = db.recent_errors(10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].event, "coolant_overheat");
        assert_eq!(rows[0].edge, "raised");
        assert_eq!(rows[0].severity.as_deref(), Some("CRITICAL"));
        assert_eq!(rows[0].odometer_km, Some(12_345.0));
    }
}
