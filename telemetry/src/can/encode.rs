//! Encode [`VehicleState`] as simulated CAN frames.

use crate::state::VehicleState;
use sigma_racer_sidearm as m7;

use super::map::to_signals;

/// Encode the current state as the full set of simulated CAN frames (used by
/// the `sim` source). Logs and returns an empty vec when encoding fails.
pub fn encode_sim_frames(state: &VehicleState) -> Vec<(u32, [u8; 8])> {
    match m7::encode_all(&to_signals(state)) {
        Ok(frames) => frames.to_vec(),
        Err(err) => {
            eprintln!("sigma-racer-telemetry/can: encode failed: {err}");
            Vec::new()
        }
    }
}
