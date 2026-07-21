//! Stderr logging with the binary's `sigma-racer-cluster:` prefix.
//!
//! Stderr (collected by systemd/journald) is the cluster's only log sink;
//! this macro keeps the prefix consistent across every module.

/// Write one prefixed line to stderr, `format!`-style.
macro_rules! log {
    ($($arg:tt)*) => {
        eprintln!("sigma-racer-cluster: {}", format_args!($($arg)*))
    };
}

pub(crate) use log;
