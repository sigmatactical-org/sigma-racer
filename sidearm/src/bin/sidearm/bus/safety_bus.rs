//! CAN-FD **safety bus** endpoint to the ECU (`sigma-racer-efi`).
//!
//! The M7 owns FLEXCAN1 on Verdin SODIMM 20/22. Linux `flexcan1` must be
//! disabled in the HMP overlay so only the M7 touches the transceiver.

use sigma_racer_sidearm::hw::FlexCan1;

use super::frame::Frame;

/// Owns the M7's safety-bus CAN-FD controller.
pub struct SafetyBus {
    can: FlexCan1,
}

impl SafetyBus {
    /// Bring up FLEXCAN1 and the decode state.
    pub fn new() -> Self {
        Self {
            can: FlexCan1::new(),
        }
    }

    /// Non-blocking receive: returns the next queued frame, if any.
    pub fn poll(&mut self) -> Option<Frame> {
        self.can
            .poll()
            .map(|rx| Frame::new(rx.id, &rx.data[..rx.len]))
    }

    /// Queue a frame for transmission on the safety bus.
    pub fn transmit(&mut self, id: u32, payload: &[u8]) {
        self.can.transmit(id, payload);
    }
}
