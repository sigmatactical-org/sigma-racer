//! One-shot database pull over the mTLS relay.
//!
//! Unlike [`TcpTelemetryClient`](super::TcpTelemetryClient), which subscribes to
//! a live stream, this opens a dedicated connection, sends a single
//! `DatabaseQuery`, reads the raw bytes of the bike's SQLite database until the
//! connection closes, and returns them. The relay classifies the connection by
//! that first line and proxies it to the bike's query socket.

use std::io::{ErrorKind, Read, Write};
use std::time::Duration;

use rustls::pki_types::ServerName;

use super::tcp::default_port;
use crate::protocol::Message;
use crate::tls::{TlsRole, client_config, connect_tls, load_material, server_name_for_host};

/// How long to wait for the bike (via the relay) to answer a query.
const QUERY_TIMEOUT: Duration = Duration::from_secs(20);
/// Guard against an implausibly large response (defends memory).
const MAX_DB_BYTES: u64 = 256 * 1024 * 1024;

/// Pull the bike's entire SQLite database through the relay, returning its bytes.
pub fn pull_database(host: &str, port: u16) -> Result<Vec<u8>, String> {
    install_crypto();
    let material = load_material(TlsRole::Client)?;
    let config = client_config(&material)?;
    let server_name: ServerName<'static> = server_name_for_host(host)?;
    let pin = material.server_pin.clone();

    let mut stream = connect_tls(&config, server_name, host, port, pin.as_ref())?;
    stream
        .get_ref()
        .set_read_timeout(Some(QUERY_TIMEOUT))
        .map_err(|e| format!("set query timeout: {e}"))?;

    let mut request = Message::database_query(1).to_line();
    request.push('\n');
    stream
        .write_all(request.as_bytes())
        .and_then(|_| stream.flush())
        .map_err(|e| format!("send database query: {e}"))?;

    // The response is framed by the bike/relay closing the connection. A clean
    // TLS shutdown yields Ok; a relay that closes without close_notify surfaces
    // as UnexpectedEof — benign here, so accept the bytes received so far.
    let mut bytes = Vec::new();
    match stream.take(MAX_DB_BYTES).read_to_end(&mut bytes) {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => {}
        Err(e) => return Err(format!("read database: {e}")),
    }
    Ok(bytes)
}

/// Pull on the [`default_port`].
pub fn pull_database_default(host: &str) -> Result<Vec<u8>, String> {
    pull_database(host, default_port())
}

/// Install the ring crypto provider once per process (idempotent).
fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
