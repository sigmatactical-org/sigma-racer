//! NDJSON telemetry envelopes (schemas/telemetry/vehicle-messages.yaml v0.1).

mod constants;
mod diff;
mod message;
mod parse_error;

pub use constants::{
    DEFAULT_TCP_PORT, PROTOCOL_VERSION, QUERY_SOCKET_PATH, SNAPSHOT_INTERVAL_MS, SOCKET_PATH,
};
pub use diff::diff_vss;
pub use message::{Message, now_iso};
pub use parse_error::ParseError;
