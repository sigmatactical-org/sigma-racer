//! mTLS role selection.

/// Which role this process plays in telemetry mTLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsRole {
    /// Wingman relay (`sigma-telemetry-relay`).
    Server,
    /// Shop tool (`sigma-racer-mechanic`).
    Client,
}
