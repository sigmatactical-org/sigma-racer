//! Speed-density fueling: `air = f(MAP, IAT, RPM, VE)`, `fuel = air / AFR`,
//! turned into an injector pulse width (`efi.md` §5).
//!
//! Pure `no_std`; every calibration is ⚠ [MEASURE] — the shapes here exist
//! so the pipeline is testable, the dyno owns the numbers.

use crate::fuel::{InjectorModel, Table};

/// Ideal-gas constant for dry air, J/(kg·K).
const R_AIR: f32 = 287.05;

/// ⚠ [MEASURE] placeholder VE shape — flat ~80 % with a mild torque hump.
/// Exists so the pipeline runs end to end; the dyno replaces it (M6).
pub const PLACEHOLDER_VE: Table<4, 4> = Table {
    row_axis: [1_000.0, 4_000.0, 7_000.0, 10_500.0],
    col_axis: [30.0, 60.0, 85.0, 100.0],
    values: [
        [55.0, 65.0, 72.0, 75.0],
        [60.0, 75.0, 85.0, 88.0],
        [62.0, 78.0, 90.0, 93.0],
        [58.0, 72.0, 84.0, 87.0],
    ],
};

/// Cylinder air mass per intake event, milligrams — ideal gas at manifold
/// conditions scaled by volumetric efficiency.
///
/// `displacement_cc` is **per cylinder**; `ve_pct` from the VE table;
/// `map_kpa` absolute; `iat_c` charge temperature.
pub fn cylinder_air_mass_mg(displacement_cc: f32, ve_pct: f32, map_kpa: f32, iat_c: f32) -> f32 {
    let t_kelvin = iat_c + 273.15;
    if t_kelvin <= 0.0 || map_kpa <= 0.0 {
        return 0.0;
    }
    // ρ [kg/m³] = P / (R·T); volume in m³ = cc × 1e-6; mass mg = kg × 1e6.
    let density = (map_kpa * 1_000.0) / (R_AIR * t_kelvin);
    density * displacement_cc * (ve_pct / 100.0)
}

/// Fuel mass for one injection event, milligrams.
pub fn fuel_mass_mg(air_mass_mg: f32, afr_target: f32) -> f32 {
    if afr_target <= 0.0 {
        return 0.0;
    }
    air_mass_mg / afr_target
}

/// Operating point for one base-pulse computation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeedDensityInputs {
    pub rpm: f32,
    pub map_kpa: f32,
    pub iat_c: f32,
    pub displacement_per_cyl_cc: f32,
    pub afr_target: f32,
    pub vbatt: f32,
}

/// One end-to-end base pulse computation: VE lookup → air → fuel → pulse.
pub fn base_pulse_ms(ve: &Table<4, 4>, injector: &InjectorModel, sd: &SpeedDensityInputs) -> f32 {
    let ve_pct = ve.lookup(sd.rpm, sd.map_kpa);
    let air = cylinder_air_mass_mg(sd.displacement_per_cyl_cc, ve_pct, sd.map_kpa, sd.iat_c);
    injector.pulse_width_ms(fuel_mass_mg(air, sd.afr_target), sd.vbatt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fuel::PLACEHOLDER_INJECTOR;

    /// CP3: 890 cc / 3 cylinders.
    const CYL_CC: f32 = 890.0 / 3.0;

    #[test]
    fn air_mass_matches_ideal_gas_at_standard_conditions() {
        // 100 kPa, 20 °C, VE 100 %: ρ ≈ 1.189 kg/m³ → ~352 mg per 296.7 cc.
        let mg = cylinder_air_mass_mg(CYL_CC, 100.0, 100.0, 20.0);
        assert!((mg - 352.0).abs() < 5.0, "air mass {mg} mg");
    }

    #[test]
    fn air_mass_scales_with_map_and_inverse_temperature() {
        let base = cylinder_air_mass_mg(CYL_CC, 100.0, 100.0, 20.0);
        assert!((cylinder_air_mass_mg(CYL_CC, 100.0, 50.0, 20.0) - base / 2.0).abs() < 0.5);
        // Hotter charge = less air.
        assert!(cylinder_air_mass_mg(CYL_CC, 100.0, 100.0, 60.0) < base);
        assert_eq!(cylinder_air_mass_mg(CYL_CC, 100.0, 0.0, 20.0), 0.0);
    }

    #[test]
    fn stoich_fuel_for_standard_cylinder_fill_is_plausible() {
        // ~352 mg air / 14.7 ≈ 24 mg fuel ≈ 6.8 ms at 3.5 mg/ms + dead time.
        let air = cylinder_air_mass_mg(CYL_CC, 100.0, 100.0, 20.0);
        let fuel = fuel_mass_mg(air, 14.7);
        assert!((fuel - 24.0).abs() < 1.0, "fuel {fuel} mg");
        let pw = PLACEHOLDER_INJECTOR.pulse_width_ms(fuel, 13.5);
        assert!(pw > 6.0 && pw < 9.0, "pulse {pw} ms");
    }

    #[test]
    fn base_pulse_end_to_end_moves_with_load() {
        let idle = base_pulse_ms(
            &PLACEHOLDER_VE,
            &PLACEHOLDER_INJECTOR,
            &SpeedDensityInputs {
                rpm: 1_200.0,
                map_kpa: 35.0,
                iat_c: 30.0,
                displacement_per_cyl_cc: CYL_CC,
                afr_target: 14.7,
                vbatt: 13.5,
            },
        );
        let wot = base_pulse_ms(
            &PLACEHOLDER_VE,
            &PLACEHOLDER_INJECTOR,
            &SpeedDensityInputs {
                rpm: 7_000.0,
                map_kpa: 100.0,
                iat_c: 30.0,
                displacement_per_cyl_cc: CYL_CC,
                afr_target: 13.0,
                vbatt: 13.5,
            },
        );
        assert!(idle > 0.5 && idle < wot, "idle {idle} ms, wot {wot} ms");
        assert!(wot < 12.0, "wot {wot} ms");
    }
}
