//! The embedded DBC source and its one-time parse into a [`Dbc`].

use dbc_rs::{Dbc, Error};

/// The canonical M7 draft DBC source — the single source of truth.
pub const M7_DBC: &str = include_str!("../dbc/m7-draft.dbc");

/// Parse the embedded M7 dictionary into a [`Dbc`].
///
/// Allocation (in `alloc` mode) happens here, not on the decode/encode path.
/// Call once and cache — see [`crate::dbc`] under `std`.
pub fn parse() -> Result<Dbc, Error> {
    Dbc::parse(M7_DBC)
}
