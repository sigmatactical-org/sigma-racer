//! End-to-end mTLS roundtrip for telemetry relay.

use sigma_racer_telemetry::tls::{accept_tls, connect_tls, server_name_for_host};
use sigma_racer_telemetry::{CertPin, TlsMaterial, client_config, server_config};
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::Arc;

fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn generate_pki(out: &Path) {
    let script =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gen-telemetry-tls.sh");
    let status = std::process::Command::new("bash")
        .arg(&script)
        .arg(out)
        .arg("IP:127.0.0.1")
        .status()
        .expect("run gen-telemetry-tls.sh");
    assert!(status.success(), "gen-telemetry-tls.sh failed");
}

#[test]
fn mtls_relay_roundtrip() {
    install_crypto();

    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path();
    generate_pki(out);

    let server_material = TlsMaterial::from_paths(
        out.join("ca.pem"),
        out.join("server.pem"),
        out.join("server.key"),
        None,
    );
    let server_tls = server_config(&server_material).expect("server config");

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let client_material = TlsMaterial::from_paths(
        out.join("ca.pem"),
        out.join("client.pem"),
        out.join("client.key"),
        None,
    );
    let pin =
        CertPin::parse_hex(&std::fs::read_to_string(out.join("server.pin")).expect("server.pin"))
            .expect("pin");
    let client_tls = client_config(&client_material).expect("client config");
    let server_name = server_name_for_host("127.0.0.1").expect("server name");

    let client_tls = Arc::clone(&client_tls);
    let client_handle = std::thread::spawn(move || {
        connect_tls(&client_tls, server_name, "127.0.0.1", port, Some(&pin)).expect("client tls")
    });

    let (tcp, _) = listener.accept().expect("tcp accept");
    let mut server = accept_tls(&server_tls, tcp).expect("server tls");
    let mut client = client_handle.join().expect("client thread");

    client
        .write_all(
            b"{\"version\":\"0.1\",\"msg\":\"Heartbeat\",\"ts\":\"t\",\"seq\":1,\"uptime_ms\":0}\n",
        )
        .unwrap();
    client.flush().unwrap();
    let mut reader = std::io::BufReader::new(&mut server);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains("Heartbeat"));
}

#[test]
fn rejects_wrong_server_pin() {
    install_crypto();

    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path();
    generate_pki(out);

    let client_material = TlsMaterial::from_paths(
        out.join("ca.pem"),
        out.join("client.pem"),
        out.join("client.key"),
        Some(CertPin::parse_hex(&"0".repeat(64)).expect("wrong pin")),
    );
    let client_tls = client_config(&client_material).expect("client config");
    let server_material = TlsMaterial::from_paths(
        out.join("ca.pem"),
        out.join("server.pem"),
        out.join("server.key"),
        None,
    );
    let server_tls = server_config(&server_material).expect("server config");
    let server_name = server_name_for_host("127.0.0.1").expect("server name");

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let wrong_pin = CertPin::parse_hex(&"0".repeat(64)).expect("wrong pin");

    let client_tls = Arc::clone(&client_tls);
    let client_handle = std::thread::spawn(move || {
        connect_tls(
            &client_tls,
            server_name,
            "127.0.0.1",
            port,
            Some(&wrong_pin),
        )
    });

    let (tcp, _) = listener.accept().expect("tcp accept");
    let _server = accept_tls(&server_tls, tcp).expect("server tls");
    let err = client_handle.join().expect("client thread").unwrap_err();
    assert!(err.contains("pin mismatch"), "unexpected error: {err}");
}
