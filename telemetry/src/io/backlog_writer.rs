//! Non-blocking writer with a bounded per-consumer backlog.

use std::collections::VecDeque;
use std::io::{ErrorKind, Write};

/// Maximum number of bytes buffered for a single slow consumer before giving
/// up on it. A backlog this large means the consumer is not keeping up.
pub const MAX_BACKLOG: usize = 256 * 1024;

/// Wraps a non-blocking [`Write`] stream with a bounded outbound buffer.
///
/// Bytes the kernel does not accept immediately are queued and retried on the
/// next flush; once the queue exceeds [`MAX_BACKLOG`] the consumer is treated
/// as dead. This keeps one stalled consumer from blocking a broadcast loop.
pub struct BacklogWriter<W: Write> {
    writer: W,
    /// Bytes not yet accepted by the underlying stream.
    pending: VecDeque<u8>,
}

impl<W: Write> BacklogWriter<W> {
    /// Wrap an already non-blocking stream with an empty backlog.
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            pending: VecDeque::new(),
        }
    }

    /// Queue bytes and attempt to flush. Returns `false` if the consumer
    /// should be dropped (fatal error or backlog exceeded).
    pub fn enqueue_and_flush(&mut self, bytes: &[u8]) -> bool {
        self.pending.extend(bytes.iter().copied());
        self.flush()
    }

    /// Try to drain the pending buffer without blocking.
    pub fn flush(&mut self) -> bool {
        while !self.pending.is_empty() {
            let (head, _) = self.pending.as_slices();
            match self.writer.write(head) {
                Ok(0) => return false,
                Ok(n) => {
                    self.pending.drain(..n);
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // Stream buffer full; keep the backlog for the next tick
                    // unless the consumer has fallen too far behind.
                    return self.pending.len() <= MAX_BACKLOG;
                }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(_) => return false,
            }
        }
        true
    }
}
