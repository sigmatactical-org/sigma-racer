//! Shared receiving half of a telemetry subscription.

use crate::protocol::Message;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

/// The consumer side of a reader thread: a bounded message channel plus the
/// liveness flag that asks the thread to exit when the subscription drops.
pub(super) struct Subscription {
    rx: Receiver<Message>,
    alive: Arc<AtomicBool>,
    _thread: JoinHandle<()>,
}

impl Subscription {
    /// Bundle a channel, liveness flag, and reader-thread handle.
    pub(super) fn new(
        rx: Receiver<Message>,
        alive: Arc<AtomicBool>,
        thread: JoinHandle<()>,
    ) -> Self {
        Self {
            rx,
            alive,
            _thread: thread,
        }
    }

    /// Take the next queued message, if any, without blocking.
    pub(super) fn try_recv(&self) -> Option<Message> {
        self.rx.try_recv().ok()
    }

    /// Iterate over every currently queued message without blocking.
    pub(super) fn drain(&self) -> impl Iterator<Item = Message> + '_ {
        std::iter::from_fn(|| self.try_recv())
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Relaxed);
    }
}
