//! Bridge between the shared M7 CAN codec and the telemetry [`VehicleState`].
//!
//! The CAN contract itself — message IDs, the `.dbc`, and the frame⇄signal
//! codec — lives in [`sigma_racer_sidearm`]. This module only maps that
//! crate's neutral [`M7Signals`] onto the `std`-side [`VehicleState`].

mod candump;
mod decode;
mod encode;
mod map;

pub use candump::{CandumpFrame, parse as parse_candump};
pub use decode::decode_frame;
pub use encode::encode_sim_frames;
pub use map::from_signals;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::m7_dbc::m7_dbc;
    use crate::state::VehicleState;

    #[test]
    fn dbc_parses() {
        assert_eq!(m7_dbc().messages().len(), 5);
    }

    #[test]
    fn round_trip_idle() {
        let idle = VehicleState::idle();
        let mut decoded = VehicleState::idle();
        decoded.speed = 0.0;
        for (id, payload) in encode_sim_frames(&idle) {
            decode_frame(id, &payload, &mut decoded);
        }
        assert!((decoded.rpm - idle.rpm).abs() < 1.0);
        assert_eq!(decoded.gear, idle.gear);
        assert_eq!(decoded.side_stand, idle.side_stand);
        assert_eq!(decoded.riding_mode, idle.riding_mode);
    }

    #[test]
    fn preserves_can_redline_bit() {
        let mut state = VehicleState::idle();
        state.rpm = 5_000.0;
        state.redline_can = true;
        state.refresh_derived();
        assert!(state.at_redline);
    }
}
