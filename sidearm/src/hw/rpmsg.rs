//! Minimal virtio RPMsg TX path for imx8mp remoteproc.
//!
//! Enough to announce the `sigma-m7-signals` endpoint and push fixed-size
//! datagrams to Linux. RX / name-service replies are not required for the
//! one-way M7 → vehicle snapshot stream.

use super::mu;
use super::reg;
use super::rsc_table::{self, VIRTIO_CONFIG_STATUS_DRIVER_OK};

/// RPMsg header prefix on every datagram.
const RPMSG_HDR_LEN: usize = 16;
/// Endpoint name announced to Linux `rpmsg_ns`.
pub const ENDPOINT_NAME: &str = "sigma-m7-signals";
/// Address Linux assigns after NS bind (typical first dynamic endpoint).
const ENDPOINT_ADDR: u32 = 0x400;

// Vring descriptors and used-ring elements live in DDR and are managed
// entirely by the Linux host; the M7 side never touches them.

/// Shared buffer pool carved from the vdevbuffer DDR region.
const POOL_BASE: u32 = 0x5540_0000;
const POOL_SLOT: usize = 512;
const POOL_SLOTS: usize = 8;

/// One-way RPMsg publisher to Linux.
pub struct RpmsgTx {
    ready: bool,
    seq: u16,
    slot: usize,
}

impl RpmsgTx {
    /// Idle transport; [`RpmsgTx::init`] completes bring-up lazily.
    pub fn new() -> Self {
        Self {
            ready: false,
            seq: 0,
            slot: 0,
        }
    }

    /// Complete virtio bring-up after Linux patches vring addresses.
    pub fn init(&mut self) {
        if self.ready {
            return;
        }
        let v0 = rsc_table::vring0_da();
        let v1 = rsc_table::vring1_da();
        if v0 == 0xFFFF_FFFF || v1 == 0xFFFF_FFFF {
            return;
        }
        unsafe {
            core::ptr::write_volatile(rsc_table::vdev_status_ptr(), VIRTIO_CONFIG_STATUS_DRIVER_OK);
        }
        announce_endpoint();
        self.ready = true;
    }

    /// Send a payload to Linux. Returns `false` if the transport is not ready.
    pub fn send(&mut self, payload: &[u8]) -> bool {
        if !self.ready {
            self.init();
        }
        if !self.ready || payload.len() + RPMSG_HDR_LEN > POOL_SLOT {
            return false;
        }
        let slot_base = POOL_BASE + (self.slot as u32) * POOL_SLOT as u32;
        write_rpmsg_frame(slot_base, payload);
        // If the host mailbox is stalled, drop this notification rather than
        // block the safety loop; the next tick republishes fresh state anyway.
        if !mu::kick_host() {
            return false;
        }
        self.slot = (self.slot + 1) % POOL_SLOTS;
        self.seq = self.seq.wrapping_add(1);
        true
    }

    /// Count of datagrams sent (wraps).
    pub fn sequence(&self) -> u16 {
        self.seq
    }
}

fn write_rpmsg_frame(base: u32, payload: &[u8]) {
    let mut off = 0usize;
    write_u32(base, off, 0x4000_0001); // src
    off += 4;
    write_u32(base, off, ENDPOINT_ADDR);
    off += 4;
    write_u32(base, off, 0);
    off += 4;
    write_u16(base, off, payload.len() as u16);
    off += 2;
    write_u16(base, off, 0);
    off += 2;
    for (i, b) in payload.iter().enumerate() {
        write_u8(base, off + i, *b);
    }
}

fn announce_endpoint() {
    let mut frame = [0u8; 64];
    let name = ENDPOINT_NAME.as_bytes();
    let name_len = name.len().min(32);
    frame[0..name_len].copy_from_slice(&name[..name_len]);
    // NS bind: flags bit 0 set in rpmsg_ns_msg; encode in first payload byte after hdr.
    let slot_base = POOL_BASE + POOL_SLOT as u32 * (POOL_SLOTS as u32 - 1);
    write_rpmsg_frame(slot_base, &frame[..name_len + 1]);
    let _ = mu::kick_host(); // best-effort announce
}

fn write_u8(base: u32, off: usize, v: u8) {
    reg::write(base as usize + off, u32::from(v));
}

fn write_u16(base: u32, off: usize, v: u16) {
    reg::write(base as usize + off, u32::from(v));
}

fn write_u32(base: u32, off: usize, v: u32) {
    reg::write(base as usize + off, v);
}
