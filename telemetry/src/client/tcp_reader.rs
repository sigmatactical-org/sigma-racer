//! Background reader thread for mTLS telemetry.

use super::line_pump::{READ_TIMEOUT, RECONNECT_DELAY, read_stream};
use super::outcome::Outcome;
use crate::protocol::Message;
use crate::tls::{CertPin, connect_tls, tls_read_timeout};
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::thread;

/// Spawn the auto-reconnecting mTLS reader thread.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_tls_reader(
    host: String,
    port: u16,
    config: Arc<ClientConfig>,
    pin: Option<CertPin>,
    server_name: ServerName<'static>,
    tx: SyncSender<Message>,
    alive: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || run(host, port, config, pin, server_name, tx, alive))
}

/// Handshake, pump frames, and reconnect until the client is dropped.
fn run(
    host: String,
    port: u16,
    config: Arc<ClientConfig>,
    pin: Option<CertPin>,
    server_name: ServerName<'static>,
    tx: SyncSender<Message>,
    alive: Arc<AtomicBool>,
) {
    while alive.load(Ordering::Relaxed) {
        match connect_tls(&config, server_name.clone(), &host, port, pin.as_ref()) {
            Ok(mut stream) => {
                tls_read_timeout(&mut stream, READ_TIMEOUT);
                // Announce ourselves so the relay routes us to the telemetry
                // fan-out (not the maintenance-query proxy). Best-effort: an
                // older relay simply ignores the unread hello.
                let mut hello = Message::subscribe().to_line();
                hello.push('\n');
                let _ = stream
                    .write_all(hello.as_bytes())
                    .and_then(|_| stream.flush());
                match read_stream(stream, &tx, &alive) {
                    Outcome::Stop => return,
                    Outcome::Reconnect => {}
                }
            }
            Err(err) => {
                eprintln!("telemetry: mTLS connect failed: {err}");
            }
        }
        if alive.load(Ordering::Relaxed) {
            thread::sleep(RECONNECT_DELAY);
        }
    }
}
