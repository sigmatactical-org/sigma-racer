//! Fail-operational **heartbeat** to the ECU.
//!
//! 50 Hz liveness frame on `M7_HEARTBEAT` (0x080) so the ECU knows the safety
//! core is alive while Linux reboots or crashes.

use sigma_racer_sidearm::M7_HEARTBEAT;

use super::safety_bus::SafetyBus;

/// Heartbeat rate: once every N service ticks (200 Hz / 4 = 50 Hz).
const TICK_DIVISOR: u8 = 4;

/// Periodic ECU heartbeat emitter.
pub struct Heartbeat {
    sequence: u8,
    tick: u8,
}

impl Heartbeat {
    /// Heartbeat state starting at sequence 0.
    pub fn new() -> Self {
        Self {
            sequence: 0,
            tick: 0,
        }
    }

    /// Emit the heartbeat frame when the rate limiter allows.
    pub fn tick(&mut self, bus: &mut SafetyBus) {
        self.tick = self.tick.wrapping_add(1);
        if !self.tick.is_multiple_of(TICK_DIVISOR) {
            return;
        }
        self.sequence = self.sequence.wrapping_add(1);
        bus.transmit(M7_HEARTBEAT, &[self.sequence, 0, 0, 0, 0, 0, 0, 0]);
    }
}
