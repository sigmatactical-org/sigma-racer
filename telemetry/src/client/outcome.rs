//! Result of pumping a single telemetry connection.

/// Why [`super::line_pump::read_stream`] returned.
pub(super) enum Outcome {
    /// The client was dropped or the channel closed — stop the thread.
    Stop,
    /// The connection ended or failed — try to reconnect.
    Reconnect,
}
