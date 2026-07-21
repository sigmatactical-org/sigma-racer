//! Decode CAN frames into [`VehicleState`].

use crate::state::VehicleState;
use sigma_racer_sidearm as m7;

use super::map::{from_signals, to_signals};

/// Decode one CAN frame into `state`. Returns `false` if the frame is not part
/// of the M7 dictionary or fails to decode.
pub fn decode_frame(id: u32, data: &[u8], state: &mut VehicleState) -> bool {
    let mut signals = to_signals(state);
    if !m7::decode(id, data, &mut signals) {
        return false;
    }
    from_signals(&signals, state);
    state.refresh_derived();
    true
}
