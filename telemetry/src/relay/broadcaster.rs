//! Fan-out NDJSON lines to connected mTLS clients.

use crate::io::BacklogWriter;
use crate::tls::{TlsServerStream, tls_set_nonblocking};

/// Broadcasts NDJSON lines to every connected mTLS client, dropping clients
/// whose backlog exceeds the [`crate::io::MAX_BACKLOG`] bound.
pub struct TlsLineBroadcaster {
    clients: Vec<BacklogWriter<TlsServerStream>>,
}

impl TlsLineBroadcaster {
    /// Create a broadcaster with no clients.
    pub fn new() -> Self {
        Self {
            clients: Vec::new(),
        }
    }

    /// Adopt a freshly handshaken stream; dropped if it cannot be switched to
    /// non-blocking mode.
    pub fn add(&mut self, mut stream: TlsServerStream) {
        if tls_set_nonblocking(&mut stream, true).is_err() {
            return;
        }
        self.clients.push(BacklogWriter::new(stream));
    }

    /// Send one NDJSON line (newline appended if missing) to every client,
    /// dropping any that fall behind.
    pub fn send_line(&mut self, line: &str) {
        let mut bytes = line.to_string();
        if !bytes.ends_with('\n') {
            bytes.push('\n');
        }
        self.clients
            .retain_mut(|client| client.enqueue_and_flush(bytes.as_bytes()));
    }
}

impl Default for TlsLineBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}
