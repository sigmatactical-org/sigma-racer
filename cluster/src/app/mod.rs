//! Application lifecycle helpers for the cluster binary.

mod clock;
mod idle;

pub use clock::start as start_clock;
pub use idle::push as push_idle;
