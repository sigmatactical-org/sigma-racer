//! Telemetry subscribers: local Unix socket and remote mTLS.
//!
//! Both readers run on their own thread and **auto-reconnect**: if the
//! producer restarts (or the connection drops), the thread keeps retrying
//! until the client is dropped. Messages are delivered over a bounded channel
//! so a slow UI cannot cause unbounded memory growth — excess frames are
//! dropped (the protocol sends periodic full snapshots, so the UI re-syncs on
//! the next one).

mod line_pump;
mod outcome;
mod query;
mod reader;
mod subscription;
mod tcp;
mod tcp_reader;
mod unix;

pub use line_pump::RECONNECT_DELAY;
pub use query::{pull_database, pull_database_default};
pub use tcp::{DEFAULT_TCP_PORT as TCP_DEFAULT_PORT, TcpTelemetryClient, default_port};
pub use unix::{TelemetryClient, connect_error, default_socket};
