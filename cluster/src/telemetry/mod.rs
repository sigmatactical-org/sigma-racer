//! sigma-racer-vehicle telemetry → sigma-racer-cluster UI bindings.
//!
//! The live source is selected at runtime by `CLUSTER_TELEMETRY_SOURCE`:
//!
//! | value              | source                                                       |
//! |--------------------|--------------------------------------------------------------|
//! | `ipc` *(default)*  | subscribe to sigma-racer-vehicle over its Unix socket        |
//! | `replay`           | replay a candump log — baked sample or `CLUSTER_REPLAY_LOG`  |
//! | `can` / `socketcan`| read live SocketCAN (needs the `can-socket` feature)         |

mod attach;
mod binding;
#[cfg(feature = "can-socket")]
mod can_source;
mod replay;
pub(crate) mod session;

use crate::log::log;
use sigma_instrumentation::SigmaDashboard;

/// Attach the telemetry source chosen by `CLUSTER_TELEMETRY_SOURCE` (default `ipc`).
pub fn attach(ui: &SigmaDashboard) {
    let source = std::env::var("CLUSTER_TELEMETRY_SOURCE").unwrap_or_else(|_| "ipc".to_owned());
    match source.as_str() {
        "ipc" => attach::attach(ui),
        "replay" => replay::attach(ui),
        "can" | "socketcan" => attach_can(ui),
        other => {
            log!("unknown CLUSTER_TELEMETRY_SOURCE '{other}', using ipc");
            attach::attach(ui);
        }
    }
}

#[cfg(feature = "can-socket")]
fn attach_can(ui: &SigmaDashboard) {
    can_source::attach(ui);
}

#[cfg(not(feature = "can-socket"))]
fn attach_can(ui: &SigmaDashboard) {
    log!(
        "CLUSTER_TELEMETRY_SOURCE=can needs a build with \
         --features can-socket; using ipc"
    );
    attach::attach(ui);
}
