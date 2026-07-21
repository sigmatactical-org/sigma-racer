//! Transport-agnostic NDJSON pump shared by the Unix and mTLS readers.

use super::outcome::Outcome;
use crate::protocol::Message;
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError};
use std::time::Duration;

/// Bound on queued messages before the reader starts dropping frames.
pub const CHANNEL_CAPACITY: usize = 512;
/// Backoff between reconnect attempts when the service is down.
pub const RECONNECT_DELAY: Duration = Duration::from_millis(500);
/// Read timeout so a reader thread periodically observes shutdown requests.
pub const READ_TIMEOUT: Duration = Duration::from_millis(500);
/// Upper bound on a single NDJSON frame.
pub const MAX_LINE_BYTES: usize = 64 * 1024;

/// Read newline-delimited frames from `stream` until it ends, fails, or the
/// client is dropped, forwarding validated [`Message`]s over `tx`.
///
/// Frames the bounded channel cannot take are dropped (the protocol resyncs
/// via periodic snapshots); malformed frames are logged and skipped; a partial
/// line longer than [`MAX_LINE_BYTES`] is discarded.
pub(super) fn read_stream<R: Read>(
    stream: R,
    tx: &SyncSender<Message>,
    alive: &Arc<AtomicBool>,
) -> Outcome {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    loop {
        if !alive.load(Ordering::Relaxed) {
            return Outcome::Stop;
        }
        match reader.read_line(&mut line) {
            Ok(0) => return Outcome::Reconnect,
            Ok(_) => {
                if !line.ends_with('\n') {
                    if line.len() > MAX_LINE_BYTES {
                        line.clear();
                    }
                    continue;
                }
                match Message::parse_validated(&line) {
                    Ok(msg) => match tx.try_send(msg) {
                        Ok(()) => {}
                        Err(TrySendError::Full(_)) => {}
                        Err(TrySendError::Disconnected(_)) => return Outcome::Stop,
                    },
                    Err(err) => eprintln!("telemetry: ignore malformed frame: {err}"),
                }
                line.clear();
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => return Outcome::Reconnect,
        }
    }
}
