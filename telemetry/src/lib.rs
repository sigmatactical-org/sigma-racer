//! Sigma Racer vehicle telemetry — VSS state, CAN decode (M7 draft), JSON/NDJSON IPC.

#![forbid(unsafe_code)]

pub mod anomaly;
pub mod availability;
pub mod can;
pub mod client;
pub mod io;
pub mod m7_dbc;
pub mod maintenance;
pub mod protocol;
pub mod relay;
pub mod socket;
pub mod state;
pub mod tls;

pub use anomaly::{AnomalyEngine, AnomalyEvent, Edge, Severity, parse_ts_millis};
pub use client::TelemetryClient;
pub use client::{
    TCP_DEFAULT_PORT, TcpTelemetryClient, default_port, pull_database, pull_database_default,
};
pub use maintenance::{
    MaintenanceInterval, MaintenanceItem, MaintenanceLogEntry, MaintenanceReport,
    MaintenanceSchedule,
};
pub use protocol::{Message, ParseError, QUERY_SOCKET_PATH, SNAPSHOT_INTERVAL_MS, SOCKET_PATH};
pub use relay::{
    Broadcaster, DEFAULT_TCP_PORT, default_listen_addr, default_socket_path, run as run_tls_relay,
};
pub use socket::{bind_listener, prepare_socket_path};
pub use state::VehicleState;
pub use tls::{CertPin, TlsMaterial, TlsRole, client_config, load_material, server_config};
