//! One step of a replay schedule: a crank or cam pulse after a delay.

/// One step of the replay schedule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Pulse the crank output after waiting `delay_us` from the previous step.
    Crank { delay_us: u32 },
    /// Pulse the cam output after waiting `delay_us` from the previous step.
    Cam { delay_us: u32 },
}

impl Step {
    /// Microseconds to wait before this edge.
    pub fn delay_us(&self) -> u32 {
        match *self {
            Step::Crank { delay_us } | Step::Cam { delay_us } => delay_us,
        }
    }
}
