//! Fuel injector flow model — flow slope plus battery-dependent dead time.
//!
//! ⚠ [MEASURE] per injector on the bench (runbook Phase 1, item 9 measures
//! resistance; flow slope and dead-time curve come from injector data or
//! the flow bench).

use crate::fuel::Curve;

/// Injector model: flow slope + battery-dependent dead time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InjectorModel {
    /// Static flow, mg of fuel per millisecond of open time.
    pub flow_mg_per_ms: f32,
    /// Dead time vs battery voltage, ms (opening minus closing latency).
    pub dead_time_ms: Curve<4>,
}

/// Placeholder CP3-class injector: ~10 g/min-ish four-hole, generic
/// dead-time shape. ⚠ [MEASURE].
pub const PLACEHOLDER_INJECTOR: InjectorModel = InjectorModel {
    flow_mg_per_ms: 3.5,
    dead_time_ms: Curve {
        x: [8.0, 11.0, 13.5, 16.0],
        y: [1.8, 1.2, 0.9, 0.7],
    },
};

impl InjectorModel {
    /// Pulse width for a fuel mass at a battery voltage, ms.
    ///
    /// Zero fuel commands zero pulse — dead time is only added to real
    /// deliveries (an injector commanded "dead time only" still clicks and
    /// wears; never emit it as an idle artifact).
    pub fn pulse_width_ms(&self, fuel_mg: f32, vbatt: f32) -> f32 {
        if fuel_mg <= 0.0 || self.flow_mg_per_ms <= 0.0 {
            return 0.0;
        }
        fuel_mg / self.flow_mg_per_ms + self.dead_time_ms.lookup(vbatt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_fuel_commands_zero_pulse_not_dead_time() {
        assert_eq!(PLACEHOLDER_INJECTOR.pulse_width_ms(0.0, 13.5), 0.0);
    }

    #[test]
    fn dead_time_grows_as_battery_sags() {
        let sagged = PLACEHOLDER_INJECTOR.pulse_width_ms(24.0, 9.0);
        let healthy = PLACEHOLDER_INJECTOR.pulse_width_ms(24.0, 14.0);
        assert!(sagged > healthy);
    }
}
