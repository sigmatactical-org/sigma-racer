//! Unix domain socket setup for sigma-racer-vehicle telemetry publisher.

mod permissions;

pub use permissions::{RUNTIME_DIR_MODE, SOCKET_MODE, bind_listener, prepare_socket_path};
