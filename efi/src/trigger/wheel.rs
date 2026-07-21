//! Trigger wheel geometry and RPM estimation.

/// Wheel definition for crank decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerWheel {
    pub teeth: u8,
    pub missing: u8,
}

impl TriggerWheel {
    /// The stock 60-2 crank wheel.
    pub const fn sixty_minus_two() -> Self {
        Self {
            teeth: 60,
            missing: 2,
        }
    }

    /// 12-1 wheel — common aftermarket pattern for high-RPM motorcycle ECU swaps.
    pub const fn twelve_minus_one() -> Self {
        Self {
            teeth: 12,
            missing: 1,
        }
    }

    /// Edges per revolution after missing-tooth subtraction.
    pub const fn effective_edges_per_rev(self) -> u8 {
        self.teeth.saturating_sub(self.missing)
    }
}

/// RPM from the period between consecutive trigger edges.
pub fn rpm_from_period_us(period_us: u32, teeth_per_rev: u8) -> f32 {
    if period_us == 0 || teeth_per_rev == 0 {
        return 0.0;
    }
    60_000_000.0 / (period_us as f32 * teeth_per_rev as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpm_from_period_at_idle() {
        // 1,200 RPM, 11 crank edges/rev (12-1): period ≈ 4,545 µs
        let rpm = rpm_from_period_us(4_545, 11);
        assert!((rpm - 1_200.0).abs() < 50.0);
    }
}
