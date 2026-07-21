//! Blocking TLS handshakes and stream helpers.

use super::pin::{CertPin, verify_server_pin};
use rustls::{ClientConnection, ServerConnection, StreamOwned};
use std::io;
use std::net::TcpStream;
use std::sync::Arc;

/// Server side of an established mTLS session.
pub type TlsServerStream = StreamOwned<ServerConnection, TcpStream>;
/// Client side of an established mTLS session.
pub type TlsClientStream = StreamOwned<ClientConnection, TcpStream>;

/// Perform a blocking server-side TLS handshake (mTLS required).
pub fn accept_tls(
    config: &Arc<rustls::ServerConfig>,
    mut tcp: TcpStream,
) -> Result<TlsServerStream, String> {
    let mut conn = ServerConnection::new(Arc::clone(config))
        .map_err(|e| format!("TLS server connection: {e}"))?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)
            .map_err(|e| format!("TLS server handshake: {e}"))?;
    }
    Ok(StreamOwned::new(conn, tcp))
}

/// Connect and perform a blocking client-side TLS handshake (mTLS + optional pin).
pub fn connect_tls(
    config: &Arc<rustls::ClientConfig>,
    server_name: rustls::pki_types::ServerName<'static>,
    host: &str,
    port: u16,
    pin: Option<&CertPin>,
) -> Result<TlsClientStream, String> {
    let addr = format!("{host}:{port}");
    let mut tcp = TcpStream::connect(&addr).map_err(|e| format!("TCP connect {addr}: {e}"))?;
    let mut conn = ClientConnection::new(Arc::clone(config), server_name)
        .map_err(|e| format!("TLS client connection: {e}"))?;
    while conn.is_handshaking() {
        conn.complete_io(&mut tcp)
            .map_err(|e| format!("TLS client handshake: {e}"))?;
    }

    if let Some(pin) = pin {
        let peer = conn
            .peer_certificates()
            .ok_or_else(|| "TLS peer did not present a certificate".to_string())?;
        verify_server_pin(peer, pin)?;
    }

    Ok(StreamOwned::new(conn, tcp))
}

/// Best-effort read timeout on a raw TCP stream.
pub fn set_read_timeout(stream: &TcpStream, timeout: std::time::Duration) {
    let _ = stream.set_read_timeout(Some(timeout));
}

/// Toggle non-blocking mode on a raw TCP stream.
pub fn set_nonblocking(stream: &TcpStream, nonblocking: bool) -> io::Result<()> {
    stream.set_nonblocking(nonblocking)
}

/// Read timeout helper for TLS streams (uses underlying TCP socket).
pub fn tls_read_timeout(stream: &mut TlsClientStream, timeout: std::time::Duration) {
    let _ = stream.get_ref().set_read_timeout(Some(timeout));
}

/// Toggle non-blocking mode on the TCP socket under a server TLS stream.
pub fn tls_set_nonblocking(stream: &mut TlsServerStream, nonblocking: bool) -> io::Result<()> {
    stream.get_mut().set_nonblocking(nonblocking)
}
