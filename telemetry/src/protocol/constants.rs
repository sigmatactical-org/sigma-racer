//! NDJSON telemetry protocol constants.

/// NDJSON envelope version (schemas/telemetry/vehicle-messages.yaml).
pub const PROTOCOL_VERSION: &str = "0.1";
/// Default Unix socket the vehicle daemon serves telemetry on.
pub const SOCKET_PATH: &str = "/run/sigma-racer-wingman/vehicle.sock";

/// Default Unix socket the vehicle daemon serves maintenance queries on. Kept
/// separate from the telemetry socket so the request/response query path stays
/// fully orthogonal to the write-only telemetry fan-out.
pub const QUERY_SOCKET_PATH: &str = "/run/sigma-racer-wingman/vehicle-query.sock";

/// Default TCP port for the Wingman mTLS telemetry relay.
pub const DEFAULT_TCP_PORT: u16 = 7357;

/// Full snapshot rate when nothing changed (10 Hz per schema).
pub const SNAPSHOT_INTERVAL_MS: u64 = 100;
