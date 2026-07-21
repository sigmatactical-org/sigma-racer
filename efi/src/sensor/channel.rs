//! Logical ADC channels on the ECU connector.

/// Logical ADC channel on the ECU connector (maps to MCU ADC inputs per board).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AdcChannel {
    /// Connector pin 1 — battery sense (after divider).
    Battery,
    /// AN volt 1 — connector pin 27.
    AnalogVolt1,
    /// AN volt 2 — connector pin 26.
    AnalogVolt2,
    /// AN volt 3 — connector pin 31.
    AnalogVolt3,
    /// AN volt 4 — connector pin 19.
    AnalogVolt4,
    /// AN volt 5 — connector pin 20.
    AnalogVolt5,
    /// AN volt 6 — connector pin 32 (often wideband).
    AnalogVolt6,
    /// AN volt 7 — connector pin 30.
    AnalogVolt7,
    /// AN temp 1 — connector pin 18 (CLT default on MRE).
    CoolantTemp,
    /// AN temp 2 — connector pin 23 (IAT default on MRE).
    IntakeTemp,
    /// AN temp 3 — connector pin 24.
    AuxTemp3,
    /// AN temp 4 — connector pin 22.
    AuxTemp4,
    /// TPS — connector pin 28 (MAP default on MRE; TPS optional).
    Tps,
    /// MAP — shares TPS pin on default MRE wiring.
    Map,
}
