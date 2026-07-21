//! TLE8888 output roles (default rusEFI assignment on microRusEFI).

/// TLE8888-driven outputs by connector role (default rusEFI assignment).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TleOutput {
    /// Injector 1 — connector pin 37.
    Injector1,
    /// Injector 2 — connector pin 38.
    Injector2,
    /// Injector 3 — connector pin 41.
    Injector3,
    /// Injector 4 — connector pin 42.
    Injector4,
    /// Fuel pump — connector pin 35.
    GpOut1,
    /// Radiator fan — connector pin 34.
    GpOut2,
    /// General purpose — connector pin 33.
    GpOut3,
    /// General purpose — connector pin 43.
    GpOut4,
    /// VVT / high-current solenoid — connector pin 7.
    LowSide1,
    /// Idle IAC solenoid — connector pin 3.
    LowSide2,
}
