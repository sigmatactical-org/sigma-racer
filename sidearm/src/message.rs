//! M7 safety-bus message IDs — the dictionary's addressing surface.

/// Engine status (RPM, temperatures, redline flag).
pub const ENGINE_STATUS: u32 = 0x0A0;
/// Throttle position, current gear, ride mode, side-stand.
pub const THROTTLE_GEAR: u32 = 0x0C0;
/// Ground speed, lean angle, longitudinal acceleration.
pub const WHEEL_SPEED: u32 = 0x120;
/// Fuel, battery, bus load, ABS/TC flags, DTC count.
pub const CHASSIS_ELECTRICAL: u32 = 0x200;
/// Odometer and trip meters.
pub const TRIP_ODOMETER: u32 = 0x220;

/// M7 → ECU fail-operational heartbeat (not in the DBC; single-byte sequence).
pub const M7_HEARTBEAT: u32 = 0x080;

/// Every message ID in the dictionary, in transmit order.
pub const MESSAGE_IDS: [u32; 5] = [
    ENGINE_STATUS,
    THROTTLE_GEAR,
    WHEEL_SPEED,
    CHASSIS_ELECTRICAL,
    TRIP_ODOMETER,
];
