//! Angle-domain events and their armed (timestamped) form.

mod armed;
mod event_id;
pub use armed::Armed;
pub use event_id::EventId;

/// An event pinned to a crank-cycle angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AngleEvent {
    pub id: EventId,
    /// Degrees within the engine cycle, `0..cycle_deg`.
    pub angle_deg: f32,
}
