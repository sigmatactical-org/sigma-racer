//! Maintenance query socket: a request/response path orthogonal to the
//! write-only telemetry fan-out.
//!
//! A client (the mTLS relay, on behalf of a shop tool) opens the query socket
//! and sends one `DatabaseQuery` line; the bike replies with the raw bytes of a
//! consistent snapshot of its SQLite database and closes the connection. The
//! shop tool opens that database locally to read whatever it needs (maintenance
//! log, error history, metadata). This never touches the telemetry broadcaster.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::time::Duration;

use sigma_racer_telemetry::protocol::Message;

use crate::db::MaintenanceDb;
use crate::log::log;

/// Bound on how long we wait for a client's request line before dropping it, so
/// a stalled connection can't wedge the single-threaded daemon loop.
const QUERY_TIMEOUT: Duration = Duration::from_secs(5);

/// Accept and answer every pending database query (non-blocking listener).
pub fn accept_queries(listener: &UnixListener, db: Option<&MaintenanceDb>) {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                if let Err(err) = handle_query(stream, db) {
                    log!("query: {err}");
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(err) => {
                log!("query accept: {err}");
                break;
            }
        }
    }
}

/// Read one request line, answer it with the database bytes, and let the
/// connection close (EOF frames the response).
fn handle_query(stream: UnixStream, db: Option<&MaintenanceDb>) -> Result<(), String> {
    stream
        .set_read_timeout(Some(QUERY_TIMEOUT))
        .and_then(|_| stream.set_write_timeout(Some(QUERY_TIMEOUT)))
        .map_err(|e| e.to_string())?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("read request: {e}"))?;
    if line.trim().is_empty() {
        return Ok(()); // client hung up without asking anything
    }

    let request = Message::parse_validated(&line).map_err(|e| format!("bad request: {e}"))?;
    match request.msg.as_str() {
        "DatabaseQuery" => {
            // No store → empty response; the shop tool reads that as unprovisioned.
            let bytes = match db {
                Some(db) => db.snapshot_bytes().unwrap_or_else(|e| {
                    log!("query snapshot: {e}");
                    Vec::new()
                }),
                None => Vec::new(),
            };
            let mut out = reader.into_inner();
            out.write_all(&bytes)
                .map_err(|e| format!("write response: {e}"))?;
            out.flush().map_err(|e| format!("flush response: {e}"))?;
            Ok(())
        }
        other => Err(format!("unexpected query kind {other}")),
    }
}
