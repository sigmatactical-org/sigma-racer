//! Fuel domain: interpolation tables, the injector model, and the
//! speed-density base-pulse pipeline (`efi.md` §5).
//!
//! - [`table`] — [`Curve`] / [`Table`] interpolation primitives (also used
//!   by [`crate::ignition`])
//! - [`injector`] — [`InjectorModel`] flow + dead time
//! - [`speed_density`] — air/fuel mass and [`base_pulse_ms`]

pub mod injector;
pub mod speed_density;
pub mod table;

pub use injector::{InjectorModel, PLACEHOLDER_INJECTOR};
pub use speed_density::{
    PLACEHOLDER_VE, SpeedDensityInputs, base_pulse_ms, cylinder_air_mass_mg, fuel_mass_mg,
};
pub use table::{Curve, Table};
