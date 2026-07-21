//! Angle-domain scheduling: convert commanded crank angles into armed
//! hardware timestamps (`efi.md` §3).
//!
//! - [`event`] — [`EventId`], [`AngleEvent`], [`Armed`]
//! - [`buffer`] — [`ArmedBuf`], the fixed-capacity per-tooth output
//! - [`core`] — the [`Scheduler`] predict-and-arm engine

pub mod buffer;
pub mod core;
pub mod event;

pub use buffer::{ArmedBuf, MAX_EVENTS};
pub use core::{Scheduler, TableFull, deg_per_us_from_rpm};
pub use event::{AngleEvent, Armed, EventId};
