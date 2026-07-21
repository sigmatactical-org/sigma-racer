//! mTLS relay: subscribe to the local Unix telemetry socket and fan out NDJSON.

mod broadcaster;

use crate::protocol::{Message, QUERY_SOCKET_PATH};
use crate::tls::{TlsServerStream, accept_tls};
use broadcaster::TlsLineBroadcaster;
use rustls::ServerConfig;
use std::io;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpListener;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub use broadcaster::TlsLineBroadcaster as Broadcaster;

/// Default TCP port for shop-tool telemetry (Mechanic). Traffic is TLS 1.3 + mTLS only.
pub use crate::protocol::DEFAULT_TCP_PORT;

const RECONNECT_DELAY: Duration = Duration::from_millis(500);
const READ_TIMEOUT: Duration = Duration::from_millis(500);
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(15);
/// How long to wait for a client's first line (Subscribe hello or a query)
/// before assuming it's a legacy silent subscriber.
const CLASSIFY_TIMEOUT: Duration = Duration::from_secs(2);
/// Bound on the bike answering a proxied maintenance query.
const QUERY_TIMEOUT: Duration = Duration::from_secs(8);

/// Run until interrupted. Forwards NDJSON from `socket_path` to mTLS clients on `listen_addr`.
pub fn run(listen_addr: &str, socket_path: &Path, tls: Arc<ServerConfig>) -> io::Result<()> {
    let listener = TcpListener::bind(listen_addr)?;
    listener.set_nonblocking(true)?;
    eprintln!(
        "sigma-telemetry-relay: mTLS listening on {listen_addr}, source {}",
        socket_path.display()
    );

    let (line_tx, line_rx) = mpsc::channel::<String>();
    let (client_tx, client_rx) = mpsc::channel::<TlsServerStream>();
    spawn_unix_subscriber(socket_path.to_path_buf(), line_tx);

    // Maintenance queries are proxied to this separate bike-side socket; the
    // telemetry fan-out above never touches it.
    let query_socket = default_query_socket_path();

    let mut clients = TlsLineBroadcaster::new();
    loop {
        accept_tls_clients(
            &listener,
            Arc::clone(&tls),
            client_tx.clone(),
            &query_socket,
        );
        while let Ok(stream) = client_rx.try_recv() {
            clients.add(stream);
        }
        while let Ok(line) = line_rx.try_recv() {
            clients.send_line(&line);
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn accept_tls_clients(
    listener: &TcpListener,
    tls: Arc<ServerConfig>,
    client_tx: mpsc::Sender<TlsServerStream>,
    query_socket: &Path,
) {
    loop {
        match listener.accept() {
            Ok((tcp, addr)) => {
                let cfg = Arc::clone(&tls);
                let tx = client_tx.clone();
                let query_socket = query_socket.to_path_buf();
                thread::spawn(move || {
                    let _ = tcp.set_read_timeout(Some(TLS_HANDSHAKE_TIMEOUT));
                    match accept_tls(&cfg, tcp) {
                        Ok(mut stream) => {
                            // Classify by the client's first line: a query
                            // connection is proxied to the bike and closed; a
                            // Subscribe hello (or a silent legacy client) joins
                            // the telemetry fan-out.
                            let _ = stream.get_ref().set_read_timeout(Some(CLASSIFY_TIMEOUT));
                            let first = read_control_line(&mut stream);
                            if is_database_query(first.as_deref()) {
                                eprintln!("sigma-telemetry-relay: database query from {addr}");
                                proxy_database_query(
                                    stream,
                                    &first.unwrap_or_default(),
                                    &query_socket,
                                );
                            } else {
                                eprintln!("sigma-telemetry-relay: mTLS client {addr}");
                                let _ = tx.send(stream);
                            }
                        }
                        Err(e) => {
                            eprintln!("sigma-telemetry-relay: rejected {addr}: {e}");
                        }
                    }
                });
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => break,
            Err(e) => {
                eprintln!("sigma-telemetry-relay: accept: {e}");
                break;
            }
        }
    }
}

/// Read one newline-terminated control line from a TLS client without buffering
/// past it (so a subscriber stream can be handed to the broadcaster intact).
/// Returns `None` on timeout, EOF, or a implausibly long line.
fn read_control_line(stream: &mut TlsServerStream) -> Option<String> {
    let mut buf = Vec::with_capacity(128);
    let mut byte = [0u8; 1];
    loop {
        match stream.read(&mut byte) {
            Ok(0) => return None,
            Ok(_) => {
                if byte[0] == b'\n' {
                    break;
                }
                buf.push(byte[0]);
                if buf.len() > 8192 {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
    String::from_utf8(buf).ok()
}

/// True when a classification line parses as a `DatabaseQuery`.
fn is_database_query(line: Option<&str>) -> bool {
    line.and_then(|l| Message::parse_line(l).ok())
        .map(|m| m.msg == "DatabaseQuery")
        .unwrap_or(false)
}

/// Forward a database query to the bike's query socket and stream the raw
/// database bytes back to the shop-tool client until the bike closes.
fn proxy_database_query(mut client: TlsServerStream, request_line: &str, query_socket: &Path) {
    match forward_request(query_socket, request_line) {
        Ok(mut bike) => {
            if let Err(e) = io::copy(&mut bike, &mut client).and_then(|_| client.flush()) {
                eprintln!("sigma-telemetry-relay: database proxy copy: {e}");
            }
            // Graceful TLS shutdown so the client sees a clean EOF (the response
            // is framed by the connection close, not a length prefix).
            client.conn.send_close_notify();
            let _ = client.flush();
        }
        Err(e) => {
            eprintln!("sigma-telemetry-relay: database query failed: {e}");
            // Best-effort error line so the shop tool surfaces a reason. An
            // empty/non-SQLite response is read as "unprovisioned".
            let _ = client.write_all(format!("{{\"error\":\"database query: {e}\"}}").as_bytes());
        }
    }
}

/// Connect to the bike's Unix query socket and send the request; the caller
/// streams the response (raw DB bytes, until EOF) from the returned stream.
fn forward_request(query_socket: &Path, request_line: &str) -> io::Result<UnixStream> {
    let stream = UnixStream::connect(query_socket)?;
    stream.set_read_timeout(Some(QUERY_TIMEOUT))?;
    stream.set_write_timeout(Some(QUERY_TIMEOUT))?;
    let mut req = request_line.to_string();
    if !req.ends_with('\n') {
        req.push('\n');
    }
    (&stream).write_all(req.as_bytes())?;
    (&stream).flush()?;
    Ok(stream)
}

/// The bike-side maintenance query socket: `TELEMETRY_QUERY_SOCKET` or default.
pub fn default_query_socket_path() -> PathBuf {
    PathBuf::from(
        std::env::var("TELEMETRY_QUERY_SOCKET").unwrap_or_else(|_| QUERY_SOCKET_PATH.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn classifies_query_vs_subscribe() {
        let query = Message::database_query(1).to_line();
        assert!(is_database_query(Some(&query)));
        assert!(!is_database_query(Some(&Message::subscribe().to_line())));
        assert!(!is_database_query(Some("not json")));
        assert!(!is_database_query(None));
    }

    #[test]
    fn forwards_request_and_streams_bike_response() {
        // Stand in for the bike: a Unix socket that reads the request line and
        // streams back some bytes (fake database), then closes.
        let dir = std::env::temp_dir().join(format!("relay-fwd-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let sock = dir.join("bike-query.sock");
        let _ = std::fs::remove_file(&sock);
        let listener = UnixListener::bind(&sock).expect("bind fake bike");

        let bike = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut reader = BufReader::new(stream);
            let mut req = String::new();
            reader.read_line(&mut req).expect("read req");
            assert!(req.contains("DatabaseQuery"));
            reader
                .into_inner()
                .write_all(b"SQLite format 3\0fake-db-bytes")
                .expect("write resp");
        });

        let request = Message::database_query(1).to_line();
        let mut stream = forward_request(&sock, &request).expect("forward");
        let mut got = Vec::new();
        stream.read_to_end(&mut got).expect("read response");
        assert!(got.starts_with(b"SQLite format 3\0"));

        bike.join().unwrap();
        let _ = std::fs::remove_file(&sock);
    }
}

fn spawn_unix_subscriber(socket_path: PathBuf, line_tx: mpsc::Sender<String>) {
    thread::spawn(move || {
        loop {
            if let Ok(stream) = UnixStream::connect(&socket_path) {
                let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
                eprintln!(
                    "sigma-telemetry-relay: connected to {}",
                    socket_path.display()
                );
                if read_unix_lines(stream, &line_tx).is_err() {
                    eprintln!("sigma-telemetry-relay: unix socket dropped, reconnecting");
                }
            }
            thread::sleep(RECONNECT_DELAY);
        }
    });
}

fn read_unix_lines(stream: UnixStream, line_tx: &mpsc::Sender<String>) -> io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => return Err(io::Error::from(ErrorKind::UnexpectedEof)),
            Ok(_) => {
                if line.ends_with('\n') && !line.trim().is_empty() {
                    let _ = line_tx.send(line.clone());
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }
}

/// The relay's telemetry source: same resolution as [`crate::client::default_socket`].
pub fn default_socket_path() -> PathBuf {
    PathBuf::from(crate::client::default_socket())
}

/// The relay bind address: `0.0.0.0` on `TELEMETRY_RELAY_PORT` or the default port.
pub fn default_listen_addr() -> String {
    let port = std::env::var("TELEMETRY_RELAY_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TCP_PORT);
    format!("0.0.0.0:{port}")
}
