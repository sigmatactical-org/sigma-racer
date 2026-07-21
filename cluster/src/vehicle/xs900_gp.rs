//! Yamaha XSR900 GP — default Sigma Racer vehicle profile.

use super::profile::VehicleProfile;

/// Yamaha XSR900 GP track build — the launch vehicle.
pub const XSR900_GP: VehicleProfile = VehicleProfile {
    idle_rpm: 1_200.0,
    rev_limit_rpm: 11_500.0,
    max_speed_kmh: 235.0,
    redline_rpm: 11_250.0,
    gauge_max_rpm: 12_000.0,
};
