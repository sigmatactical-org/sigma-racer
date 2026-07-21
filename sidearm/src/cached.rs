//! `std`-only conveniences over a lazily-parsed, cached database.
//!
//! `no_std` firmware parses once at boot and threads the [`Dbc`] itself; Linux
//! consumers reach for these helpers instead.

use std::sync::OnceLock;

use dbc_rs::{Dbc, Error};

use crate::{M7Signals, decode_into, encode_frames, parse};

static PARSED: OnceLock<Dbc> = OnceLock::new();

/// Lazily-parsed, cached M7 database (`std` only, thread-safe).
pub fn dbc() -> &'static Dbc {
    PARSED.get_or_init(|| parse().expect("m7-draft.dbc must parse"))
}

/// Decode one frame into `out` using the cached database (`std` convenience).
pub fn decode(id: u32, data: &[u8], out: &mut M7Signals) -> bool {
    decode_into(dbc(), id, data, out)
}

/// Encode all frames using the cached database (`std` convenience).
pub fn encode_all(s: &M7Signals) -> Result<[(u32, [u8; 8]); 5], Error> {
    encode_frames(dbc(), s)
}

#[cfg(test)]
mod tests {
    use super::dbc;

    #[test]
    fn dbc_parses_five_messages() {
        assert_eq!(dbc().messages().len(), 5);
    }
}
