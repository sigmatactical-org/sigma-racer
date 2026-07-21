//! Fixed-capacity output buffer for one tooth window's armed events.

use crate::scheduler::Armed;

/// Capacity of the armed-event buffer.
pub const MAX_EVENTS: usize = 16;

/// Fixed-capacity output buffer for one tooth window.
#[derive(Debug, Default)]
pub struct ArmedBuf {
    items: [Option<Armed>; MAX_EVENTS],
    len: usize,
}

impl ArmedBuf {
    /// Empty buffer.
    pub const fn new() -> Self {
        Self {
            items: [None; MAX_EVENTS],
            len: 0,
        }
    }

    /// Drop every armed event.
    pub fn clear(&mut self) {
        self.items = [None; MAX_EVENTS];
        self.len = 0;
    }

    /// Append an armed event; silently drops when full (diagnosed upstream).
    pub(crate) fn push(&mut self, armed: Armed) {
        if self.len < MAX_EVENTS {
            self.items[self.len] = Some(armed);
            self.len += 1;
        }
    }

    /// Number of armed events.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether no events are armed.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Iterate armed events in arming order.
    pub fn iter(&self) -> impl Iterator<Item = &Armed> {
        self.items[..self.len].iter().flatten()
    }
}
