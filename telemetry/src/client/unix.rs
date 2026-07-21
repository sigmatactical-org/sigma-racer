//! Unix socket telemetry subscriber (sigma-racer-cluster).

use super::line_pump::CHANNEL_CAPACITY;
use super::reader::{configure, spawn_reader};
use super::subscription::Subscription;
use crate::protocol::{Message, SOCKET_PATH};
use std::io::Error;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;

/// Subscriber for the local sigma-racer-vehicle Unix socket.
///
/// The reader runs on its own thread and auto-reconnects; dropping the client
/// asks the thread to exit.
pub struct TelemetryClient {
    inner: Subscription,
}

impl TelemetryClient {
    /// Connect to sigma-racer-vehicle; spawns a reader thread. Returns `None` if the
    /// service is not currently reachable (callers may retry later).
    pub fn connect() -> Option<Self> {
        Self::connect_path(default_socket())
    }

    /// [`TelemetryClient::connect`] against an explicit socket path.
    pub fn connect_path(path: impl AsRef<Path>) -> Option<Self> {
        let path = path.as_ref().to_path_buf();
        let stream = UnixStream::connect(&path).ok()?;
        configure(&stream);

        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let alive = Arc::new(AtomicBool::new(true));
        let thread = spawn_reader(path, Some(stream), tx, Arc::clone(&alive));
        Some(Self {
            inner: Subscription::new(rx, alive, thread),
        })
    }

    /// Take the next queued message, if any, without blocking.
    pub fn try_recv(&self) -> Option<Message> {
        self.inner.try_recv()
    }

    /// Iterate over every currently queued message without blocking.
    pub fn drain(&self) -> impl Iterator<Item = Message> + '_ {
        self.inner.drain()
    }
}

/// The telemetry socket path: `SIGMA_RACER_WINGMAN_TELEMETRY_SOCKET` or the
/// protocol default.
pub fn default_socket() -> String {
    std::env::var("SIGMA_RACER_WINGMAN_TELEMETRY_SOCKET")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| SOCKET_PATH.into())
}

/// Human-readable connect failure for callers that surface it to users.
pub fn connect_error(path: &Path, err: &Error) -> String {
    format!("telemetry: could not connect to {}: {err}", path.display())
}
