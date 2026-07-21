//! Stage-1 trigger edge capture — tooth-pattern discovery for mule Phase 1.
//!
//! Characterization-grade, not control-grade: EXTI wakeups timestamped with
//! the 1 MHz embassy-time tick carry executor jitter, which is fine for
//! reading a tooth pattern at cranking/idle speed (a tooth is ~1 ms at
//! 1000 rpm on a 60-tooth wheel). The timing-grade input-capture decoder
//! replaces this path for engine control.
//!
//! Record formats:
//!
//! `DL,T,<line C|V>,<count>,<t_us>,<period_us>,<gap_ratio_x100>`
//! `DL,X,dropped_edges,<n>`
//!
//! A steady `gap_ratio_x100` near 100 is tooth-to-tooth; ~200 marks a
//! single-missing-tooth gap (~300 for two missing); ~50 is the tooth after
//! the gap. That signature *is* the wheel pattern (runbook Phase 1, item 6).

use core::sync::atomic::{AtomicU32, Ordering};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::mode::Async;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::Instant;
use sigma_racer_efi::trigger::{EdgeEvent, EdgeIntervals, TriggerLine};

/// Captured edges awaiting the logger. Sized for a burst of a full crank
/// revolution at cranking speed.
pub static EDGES: Channel<CriticalSectionRawMutex, EdgeEvent, 128> = Channel::new();

static DROPPED: AtomicU32 = AtomicU32::new(0);

fn push(event: EdgeEvent) {
    if EDGES.try_send(event).is_err() {
        DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

/// Crank trigger (pin 45). The VR conditioner outputs one clean logic edge
/// per tooth; count a single polarity so periods are tooth periods.
#[embassy_executor::task]
pub async fn crank_capture(mut pin: ExtiInput<'static, Async>) {
    loop {
        pin.wait_for_falling_edge().await;
        push(EdgeEvent {
            t_us: Instant::now().as_micros(),
            line: TriggerLine::Crank,
            rising: false,
        });
    }
}

/// Cam hall input (pin 25). Both polarities logged — cam patterns are
/// asymmetric and the pulse widths are part of the signature.
#[embassy_executor::task]
pub async fn cam_capture(mut pin: ExtiInput<'static, Async>) {
    loop {
        pin.wait_for_any_edge().await;
        let rising = pin.is_high();
        push(EdgeEvent {
            t_us: Instant::now().as_micros(),
            line: TriggerLine::Cam,
            rising,
        });
    }
}

/// Drain captured edges, keep per-line interval statistics, emit records.
#[embassy_executor::task]
pub async fn edge_logger() {
    let mut crank = EdgeIntervals::new();
    let mut cam = EdgeIntervals::new();

    loop {
        let event = EDGES.receive().await;
        let (tag, intervals) = match event.line {
            TriggerLine::Crank => ("C", &mut crank),
            TriggerLine::Cam => ("V", &mut cam),
        };
        if let Some(report) = intervals.record(event.t_us) {
            defmt::info!(
                "DL,T,{=str},{=u32},{=u64},{=u32},{=u32}",
                tag,
                report.count,
                event.t_us,
                report.period_us,
                report.gap_ratio_x100
            );
        }

        let dropped = DROPPED.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            defmt::warn!("DL,X,dropped_edges,{=u32}", dropped);
        }
    }
}
