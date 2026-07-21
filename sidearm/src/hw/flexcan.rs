//! FLEXCAN1 driver for the M7 safety bus (classic CAN 8-byte frames).
//!
//! Verdin pinmux: CAN_1_TX/RX on SODIMM 20/22 (SPDIF_TX/RX alt2). Linux must
//! release FLEXCAN1 to the M7 via the HMP device-tree overlay.
//!
//! # Not yet silicon-validated
//!
//! Two things must be confirmed against the i.MX8M Plus reference manual before
//! trusting RX on hardware — they are register-offset dependent, so they are
//! left as TODOs rather than guessed here:
//!
//! 1. **Register map.** The offsets in [`regoff`] (`CTRL2`, `ESR1`, `IMASK1`,
//!    `IFLAG1`) do not match the canonical FlexCAN-FD layout; verify them.
//! 2. **RX mailbox unlock.** After reading a received mailbox its internal lock
//!    must be released by reading the free-running TIMER register; and the RX
//!    global mask (`RXMGMASK`) should be set (0 = accept-all) so the single RX
//!    mailbox reliably matches. Both need the confirmed offsets from (1).

mod rx_frame;

pub use rx_frame::RxFrame;

use super::reg::{self, FLEXCAN1_BASE, IOMUXC_BASE};

const PAD_CONF_CAN: u32 = 0x0000_0154;

/// FlexCAN register offsets (see module docs: not yet silicon-validated).
mod regoff {
    /// Module configuration.
    pub const MCR: usize = 0x00;
    /// Control 1 (bit timing).
    pub const CTRL1: usize = 0x04;
    /// Control 2.
    pub const CTRL2: usize = 0x28;
    /// Error/status 1.
    pub const ESR1: usize = 0x40;
    /// Interrupt mask 1.
    pub const IMASK1: usize = 0x44;
    /// Interrupt flags 1.
    pub const IFLAG1: usize = 0x48;
    /// Message-buffer RAM base.
    pub const MB_RAM: usize = 0x80;
}

const MCR_MDIS: u32 = 1 << 31;
const MCR_FRZ: u32 = 1 << 30;
const MCR_HALT: u32 = 1 << 28;
const MCR_NOT_RDY: u32 = 1 << 29;

const MB_SIZE: usize = 0x10;
const RX_MB: usize = 0;
const TX_MB: usize = 1;

const MB_CS_CODE_MASK: u32 = 0x0F00_0000;
const MB_CS_CODE_SHIFT: u32 = 24;
const MB_CS_IDE: u32 = 1 << 21;
const MB_CS_DLC_SHIFT: u32 = 16;

const CODE_RX_EMPTY: u32 = 0x4;
const CODE_RX_FULL: u32 = 0x2;
/// A newer frame arrived before the previous one was read: the mailbox holds the
/// newest frame, and at least one earlier frame was lost.
const CODE_RX_OVERRUN: u32 = 0x6;
const CODE_TX_INACTIVE: u32 = 0x8;
const CODE_TX_DATA: u32 = 0xC;

const IFLAG1_RX: u32 = 1 << RX_MB;
const IFLAG1_TX: u32 = 1 << TX_MB;

/// Blocking FLEXCAN1 controller.
pub struct FlexCan1 {
    rx_queue: [Option<RxFrame>; 8],
    rx_head: usize,
    rx_tail: usize,
    /// Frames lost to hardware overruns or a full software queue (diagnostic).
    rx_dropped: u32,
}

impl FlexCan1 {
    /// Pinmux, reset, and start the controller at 1 Mbit/s.
    pub fn new() -> Self {
        pinmux_can1();
        init_controller();
        Self {
            rx_queue: [None; 8],
            rx_head: 0,
            rx_tail: 0,
            rx_dropped: 0,
        }
    }

    /// Count of RX frames lost to controller overruns or a full software queue.
    pub fn dropped(&self) -> u32 {
        self.rx_dropped
    }

    /// Non-blocking receive.
    pub fn poll(&mut self) -> Option<RxFrame> {
        self.drain_hw();
        if self.rx_head == self.rx_tail {
            return None;
        }
        let frame = self.rx_queue[self.rx_tail].take();
        self.rx_tail = (self.rx_tail + 1) % self.rx_queue.len();
        frame
    }

    /// Queue a standard 11-bit frame for transmission (up to 8 bytes).
    pub fn transmit(&mut self, id: u32, payload: &[u8]) {
        self.drain_hw();
        let base = FLEXCAN1_BASE + regoff::MB_RAM + TX_MB * MB_SIZE;
        let len = payload.len().min(8);

        // Wait (bounded) until the TX mailbox is inactive. A wedged controller
        // must not hang the safety loop, so bail rather than spin forever.
        if !spin_until(|| mb_code(base) == CODE_TX_INACTIVE) {
            return;
        }

        reg::write(base + 0x04, id << 18);
        let mut data = [0u32; 2];
        for (i, b) in payload[..len].iter().enumerate() {
            let word = i / 4;
            let shift = (3 - (i % 4)) * 8;
            data[word] |= u32::from(*b) << shift;
        }
        reg::write(base + 0x08, data[0]);
        reg::write(base + 0x0C, data[1]);

        let cs = (CODE_TX_DATA << MB_CS_CODE_SHIFT) | ((len as u32) << MB_CS_DLC_SHIFT);
        reg::write(base, cs);

        // Wait (bounded) for completion. On a bus-off / missing ECU there is no
        // ACK and the frame never completes; abort it and move on so a dead bus
        // can't wedge the heartbeat TX.
        if !spin_until(|| reg::read(FLEXCAN1_BASE + regoff::IFLAG1) & IFLAG1_TX != 0) {
            mb_set_code(base, CODE_TX_INACTIVE);
            return;
        }
        reg::write(FLEXCAN1_BASE + regoff::IFLAG1, IFLAG1_TX);
        mb_set_code(base, CODE_TX_INACTIVE);
    }

    fn drain_hw(&mut self) {
        let iflag = reg::read(FLEXCAN1_BASE + regoff::IFLAG1);
        if iflag & IFLAG1_RX == 0 {
            return;
        }
        let base = FLEXCAN1_BASE + regoff::MB_RAM + RX_MB * MB_SIZE;
        let cs = reg::read(base);
        let code = (cs & MB_CS_CODE_MASK) >> MB_CS_CODE_SHIFT;

        // Read the frame on FULL or OVERRUN; anything else means no valid frame
        // is latched, so just re-arm. OVERRUN additionally tells us at least one
        // earlier frame was dropped before we got here.
        if code != CODE_RX_FULL && code != CODE_RX_OVERRUN {
            mb_set_code(base, CODE_RX_EMPTY);
            reg::write(FLEXCAN1_BASE + regoff::IFLAG1, IFLAG1_RX);
            return;
        }
        if code == CODE_RX_OVERRUN {
            self.rx_dropped = self.rx_dropped.wrapping_add(1);
        }

        // The M7 dictionary is standard-ID only; drop extended frames rather
        // than mis-read their ID from the standard field.
        if cs & MB_CS_IDE == 0 {
            let len = ((cs >> MB_CS_DLC_SHIFT) & 0xF) as usize;
            let id = reg::read(base + 0x04) >> 18;
            let w0 = reg::read(base + 0x08);
            let w1 = reg::read(base + 0x0C);
            let mut data = [0u8; 8];
            for i in 0..len.min(8) {
                let word = if i < 4 { w0 } else { w1 };
                let shift = (3 - (i % 4)) * 8;
                data[i] = (word >> shift) as u8;
            }
            let next = (self.rx_head + 1) % self.rx_queue.len();
            if next != self.rx_tail {
                self.rx_queue[self.rx_head] = Some(RxFrame { id, data, len });
                self.rx_head = next;
            } else {
                // Software queue full — the new frame is lost, not the old ones.
                self.rx_dropped = self.rx_dropped.wrapping_add(1);
            }
        }

        // TODO(silicon): read the free-running TIMER register here to release the
        // mailbox lock (see the module docs); needs the confirmed offset.
        mb_set_code(base, CODE_RX_EMPTY);
        reg::write(FLEXCAN1_BASE + regoff::IFLAG1, IFLAG1_RX);
    }
}

fn init_controller() {
    let base = FLEXCAN1_BASE;
    reg::write(base + regoff::MCR, MCR_MDIS);
    reg::write(base + regoff::MCR, MCR_FRZ | MCR_HALT);

    // 1 Mbit/s @ 40 MHz PE: prescaler=2, 20 TQ (PROPSEG=7, PSEG1=6, PSEG2=6, RJW=1).
    let ctrl1 = (2 << 24) | (1 << 22) | (6 << 19) | (6 << 16) | 7;
    reg::write(base + regoff::CTRL1, ctrl1);
    reg::write(base + regoff::CTRL2, 0);

    configure_rx_mb(base);
    configure_tx_mb(base);

    reg::write(base + regoff::IMASK1, IFLAG1_RX | IFLAG1_TX);

    let mut mcr = reg::read(base + regoff::MCR);
    mcr &= !(MCR_FRZ | MCR_HALT | MCR_MDIS);
    reg::write(base + regoff::MCR, mcr);
    while reg::read(base + regoff::MCR) & MCR_NOT_RDY != 0 {}
}

fn configure_rx_mb(base: usize) {
    let mb = base + regoff::MB_RAM + RX_MB * MB_SIZE;
    reg::write(mb + 0x04, 0);
    mb_set_code(mb, CODE_RX_EMPTY);
}

fn configure_tx_mb(base: usize) {
    let mb = base + regoff::MB_RAM + TX_MB * MB_SIZE;
    mb_set_code(mb, CODE_TX_INACTIVE);
}

/// Upper bound on spins waiting for a TX mailbox transition. Sized well past the
/// worst-case frame time at 1 Mbit/s so a healthy bus always completes, while a
/// bus-off / absent ECU falls through instead of hanging the safety loop.
const TX_TIMEOUT_SPINS: u32 = 1_000_000;

/// Poll `cond` up to [`TX_TIMEOUT_SPINS`] times; `true` if it held before then.
fn spin_until(mut cond: impl FnMut() -> bool) -> bool {
    for _ in 0..TX_TIMEOUT_SPINS {
        if cond() {
            return true;
        }
    }
    false
}

fn mb_code(mb: usize) -> u32 {
    (reg::read(mb) & MB_CS_CODE_MASK) >> MB_CS_CODE_SHIFT
}

fn mb_set_code(mb: usize, code: u32) {
    let cs = reg::read(mb) & !MB_CS_CODE_MASK;
    reg::write(mb, cs | (code << MB_CS_CODE_SHIFT));
}

fn pinmux_can1() {
    // SPDIF_RX → CAN1_RX (alt 2), SPDIF_TX → CAN1_TX (alt 2).
    mux_pad(0x0A4, 0x314, 2, PAD_CONF_CAN);
    mux_pad(0x0A0, 0x310, 2, PAD_CONF_CAN);
}

fn mux_pad(mux_off: u32, conf_off: u32, alt: u32, pad_conf: u32) {
    reg::write(IOMUXC_BASE + mux_off as usize, alt);
    reg::write(IOMUXC_BASE + conf_off as usize, pad_conf);
}

#[cfg(test)]
mod tests {
    use super::clock::CAN_PE_CLK_HZ;

    #[test]
    fn bitrate_math() {
        let tq = 1 + 7 + 6 + 6;
        let prescaler = 2u32;
        let bitrate = CAN_PE_CLK_HZ / (prescaler * tq);
        assert_eq!(bitrate, 1_000_000);
    }
}
