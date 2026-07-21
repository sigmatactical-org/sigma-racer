//! mTLS telemetry subscriber (Mechanic / shop tools over WiFi).

use super::line_pump::CHANNEL_CAPACITY;
use super::subscription::Subscription;
use super::tcp_reader::spawn_tls_reader;
use crate::protocol::Message;
use crate::tls::{TlsRole, client_config, load_material, server_name_for_host};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;

pub use crate::protocol::DEFAULT_TCP_PORT;

/// Subscriber for the Wingman relay over TLS 1.3 + mTLS.
///
/// The reader runs on its own thread and auto-reconnects; dropping the client
/// asks the thread to exit.
pub struct TcpTelemetryClient {
    inner: Subscription,
}

impl TcpTelemetryClient {
    /// Connect with TLS 1.3 + mTLS using PEM material from env / default paths.
    pub fn connect(host: &str, port: u16) -> Result<Self, String> {
        install_crypto();
        let material = load_material(TlsRole::Client)?;
        let config = client_config(&material)?;
        let server_name = server_name_for_host(host)?;
        let pin = material.server_pin.clone();

        let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
        let alive = Arc::new(AtomicBool::new(true));
        let host = host.to_string();
        let thread = spawn_tls_reader(host, port, config, pin, server_name, tx, Arc::clone(&alive));
        Ok(Self {
            inner: Subscription::new(rx, alive, thread),
        })
    }

    /// [`TcpTelemetryClient::connect`] on [`default_port`].
    pub fn connect_default(host: &str) -> Result<Self, String> {
        Self::connect(host, default_port())
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

/// The relay port: `TELEMETRY_TCP_PORT` or [`DEFAULT_TCP_PORT`].
pub fn default_port() -> u16 {
    std::env::var("TELEMETRY_TCP_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TCP_PORT)
}

/// Install the ring crypto provider once per process (idempotent).
fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
