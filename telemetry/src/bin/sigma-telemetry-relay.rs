//! mTLS relay daemon — forwards local Unix telemetry to authenticated WiFi clients.

#![forbid(unsafe_code)]

use sigma_racer_telemetry::relay::{default_listen_addr, default_socket_path, run};
use sigma_racer_telemetry::tls::{TlsRole, load_material, server_config};

fn main() {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("install ring crypto provider");

    let listen = std::env::var("TELEMETRY_RELAY_LISTEN").unwrap_or_else(|_| default_listen_addr());
    let socket = std::env::var("TELEMETRY_RELAY_SOCKET")
        .ok()
        .map(Into::into)
        .unwrap_or_else(default_socket_path);

    let material = load_material(TlsRole::Server).unwrap_or_else(|e| {
        eprintln!("sigma-telemetry-relay: {e}");
        std::process::exit(1);
    });
    let tls = server_config(&material).unwrap_or_else(|e| {
        eprintln!("sigma-telemetry-relay: {e}");
        std::process::exit(1);
    });

    if let Err(err) = run(&listen, &socket, tls) {
        eprintln!("sigma-telemetry-relay: {err}");
        std::process::exit(1);
    }
}
