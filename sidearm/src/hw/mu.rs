//! MU1 mailbox doorbell for virtio RPMsg kicks (A53 ↔ M7).

use super::reg::{self, MU1_BASE};

/// MU transmit register for channel 0 (M7 → A53 kick).
const MU_TR0: usize = MU1_BASE + 0x20;
/// MU control/status register for channel 0.
const MU_CSR0: usize = MU1_BASE + 0x10;
/// Transmit empty flag in CSR.
const MU_CSR_TE: u32 = 1 << 23;

/// Bounded spins waiting for the MU TX register to drain. Prevents a stalled or
/// absent A53 from hanging the caller forever — critical on the safety loop,
/// whose heartbeat must keep running exactly when Linux is down.
const KICK_TIMEOUT_SPINS: u32 = 100_000;

/// Notify the A53 that the TX virtio vring has a new buffer. Returns `false` if
/// the mailbox never drained (host stalled/down) so the caller can drop the
/// notification rather than block.
#[must_use]
pub fn kick_host() -> bool {
    let mut spins = 0u32;
    while reg::read(MU_CSR0) & MU_CSR_TE == 0 {
        spins += 1;
        if spins >= KICK_TIMEOUT_SPINS {
            return false;
        }
    }
    reg::write(MU_TR0, 0);
    true
}
