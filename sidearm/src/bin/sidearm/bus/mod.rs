//! Safety-bus domain: the M7's CAN-FD endpoint to the ECU and the liveness
//! heartbeat it emits over that bus.
//!
//! - [`frame`] — the received [`frame::Frame`] value
//! - [`safety_bus`] — the [`SafetyBus`] controller (RX/TX)
//! - [`heartbeat`] — the periodic [`Heartbeat`] emitter

pub mod frame;
pub mod heartbeat;
pub mod safety_bus;

pub use heartbeat::Heartbeat;
pub use safety_bus::SafetyBus;
