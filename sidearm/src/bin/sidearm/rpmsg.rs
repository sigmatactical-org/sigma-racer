//! **RPMsg / OpenAMP** gateway to the A53 Linux side.
//!
//! Publishes [`M7Signals`] snapshots on the `sigma-m7-signals` virtio endpoint
//! for `sigma-racer-vehicle` to consume.

use sigma_racer_sidearm::{M7Signals, encode_wire, hw::RpmsgTx};

/// RPMsg endpoint publishing state to the Linux cluster.
pub struct RpmsgLink {
    tx: RpmsgTx,
    buf: [u8; 64],
}

impl RpmsgLink {
    /// Idle link; virtio bring-up completes lazily on first send.
    pub fn new() -> Self {
        let mut tx = RpmsgTx::new();
        tx.init();
        Self { tx, buf: [0; 64] }
    }

    /// Publish the latest vehicle state to Linux.
    pub fn publish(&mut self, state: &M7Signals) {
        let seq = self.tx.sequence();
        let Some(len) = encode_wire(seq, state, &mut self.buf) else {
            return;
        };
        let _ = self.tx.send(&self.buf[..len]);
    }
}
