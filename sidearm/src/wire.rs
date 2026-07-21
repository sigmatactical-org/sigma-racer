//! RPMsg wire format for [`M7Signals`] snapshots (M7 → Linux).
//!
//! Fixed little-endian layout, versioned, allocation-free encode/decode.

use crate::{M7Signals, PerformanceMode};

/// Magic `b"M7SG"`.
pub const MAGIC: u32 = 0x4D37_5347;
/// Current wire revision.
pub const VERSION: u16 = 1;
/// Header size in bytes: MAGIC (4) + VERSION (2) + seq (2).
pub const HEADER_LEN: usize = 8;
/// Total wire packet length: [`HEADER_LEN`] header + fixed signal body.
///
/// A single constant used by both [`encode`] and [`decode`] so their length
/// contracts can never drift (a partial packet would otherwise encode "ok" but
/// fail to decode).
pub const PACKET_LEN: usize = 60;

/// Serialize `state` into `buf`. Returns bytes written or `None` if `buf` is too small.
pub fn encode(seq: u16, state: &M7Signals, buf: &mut [u8]) -> Option<usize> {
    if buf.len() < PACKET_LEN {
        return None;
    }
    buf[0..4].copy_from_slice(&MAGIC.to_le_bytes());
    buf[4..6].copy_from_slice(&VERSION.to_le_bytes());
    buf[6..8].copy_from_slice(&seq.to_le_bytes());
    buf[8..12].copy_from_slice(&state.engine_rpm.to_le_bytes());
    buf[12..14].copy_from_slice(&state.coolant_c.to_le_bytes());
    buf[14..16].copy_from_slice(&state.oil_c.to_le_bytes());
    buf[16] = u8::from(state.redline);
    buf[17..21].copy_from_slice(&state.throttle_pct.to_le_bytes());
    buf[21] = state.gear as u8;
    buf[22] = state.performance_mode.to_raw();
    buf[23] = u8::from(state.side_stand);
    buf[24..28].copy_from_slice(&state.ground_speed.to_le_bytes());
    buf[28..32].copy_from_slice(&state.lean_angle.to_le_bytes());
    buf[32..36].copy_from_slice(&state.long_accel.to_le_bytes());
    buf[36..40].copy_from_slice(&state.fuel_pct.to_le_bytes());
    buf[40..44].copy_from_slice(&state.battery_v.to_le_bytes());
    buf[44] = state.can_load;
    buf[45] = u8::from(state.abs_active);
    buf[46] = u8::from(state.tc_active);
    buf[47] = state.dtc_count;
    buf[48..52].copy_from_slice(&state.odometer.to_le_bytes());
    buf[52..56].copy_from_slice(&state.trip1.to_le_bytes());
    buf[56..60].copy_from_slice(&state.trip2.to_le_bytes());
    Some(PACKET_LEN)
}

/// Decode a wire packet into `out`. Returns `None` on bad magic/version/length.
pub fn decode(buf: &[u8], out: &mut M7Signals) -> Option<u16> {
    if buf.len() < PACKET_LEN {
        return None;
    }
    let magic = u32::from_le_bytes(buf[0..4].try_into().ok()?);
    if magic != MAGIC {
        return None;
    }
    let version = u16::from_le_bytes(buf[4..6].try_into().ok()?);
    if version != VERSION {
        return None;
    }
    let seq = u16::from_le_bytes(buf[6..8].try_into().ok()?);
    out.engine_rpm = f32::from_le_bytes(buf[8..12].try_into().ok()?);
    out.coolant_c = i16::from_le_bytes(buf[12..14].try_into().ok()?);
    out.oil_c = i16::from_le_bytes(buf[14..16].try_into().ok()?);
    out.redline = buf[16] != 0;
    out.throttle_pct = f32::from_le_bytes(buf[17..21].try_into().ok()?);
    out.gear = buf[21] as i8;
    out.performance_mode = PerformanceMode::from_raw(buf[22]);
    out.side_stand = buf[23] != 0;
    out.ground_speed = f32::from_le_bytes(buf[24..28].try_into().ok()?);
    out.lean_angle = f32::from_le_bytes(buf[28..32].try_into().ok()?);
    out.long_accel = f32::from_le_bytes(buf[32..36].try_into().ok()?);
    out.fuel_pct = f32::from_le_bytes(buf[36..40].try_into().ok()?);
    out.battery_v = f32::from_le_bytes(buf[40..44].try_into().ok()?);
    out.can_load = buf[44];
    out.abs_active = buf[45] != 0;
    out.tc_active = buf[46] != 0;
    out.dtc_count = buf[47];
    out.odometer = f32::from_le_bytes(buf[48..52].try_into().ok()?);
    out.trip1 = f32::from_le_bytes(buf[52..56].try_into().ok()?);
    out.trip2 = f32::from_le_bytes(buf[56..60].try_into().ok()?);
    Some(seq)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn wire_round_trip() {
        let sample = M7Signals {
            engine_rpm: 3_500.0,
            coolant_c: 82,
            oil_c: 95,
            redline: false,
            throttle_pct: 15.0,
            gear: 2,
            performance_mode: PerformanceMode::Sport,
            side_stand: true,
            ground_speed: 60.0,
            lean_angle: 5.0,
            long_accel: -0.1,
            fuel_pct: 70.0,
            battery_v: 13.8,
            can_load: 5,
            abs_active: false,
            tc_active: true,
            dtc_count: 0,
            odometer: 12_345.6,
            trip1: 10.0,
            trip2: 2.5,
        };
        let mut buf = [0u8; 64];
        let n = encode(42, &sample, &mut buf).unwrap();
        assert_eq!(n, PACKET_LEN);
        let mut out = M7Signals::default();
        let seq = decode(&buf[..n], &mut out).unwrap();
        assert_eq!(seq, 42);
        // Every field must survive the round trip byte-exactly.
        assert_eq!(out, sample);
    }

    #[test]
    fn rejects_short_buffers() {
        let sample = M7Signals::default();
        let mut buf = [0u8; 64];
        // A buffer one byte short of a full packet must fail to encode…
        assert_eq!(encode(1, &sample, &mut buf[..PACKET_LEN - 1]), None);
        // …and a full packet truncated by one byte must fail to decode, rather
        // than silently succeeding (the old 56/60 mismatch).
        let n = encode(1, &sample, &mut buf).unwrap();
        let mut out = M7Signals::default();
        assert_eq!(decode(&buf[..n - 1], &mut out), None);
    }
}
