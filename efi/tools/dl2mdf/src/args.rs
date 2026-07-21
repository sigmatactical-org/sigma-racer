//! [`Args`].

#[allow(unused_imports)]
use super::*;

/// Parsed command-line arguments.
pub(crate) struct Args {
    pub(crate) dl_path: String,
    pub(crate) can_path: Option<String>,
    pub(crate) can_offset_s: f64,
    pub(crate) out_path: String,
}
