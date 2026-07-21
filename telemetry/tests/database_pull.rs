//! End-to-end database pull: shop-tool client → mTLS relay → bike query socket.
//! Exercises the request/response path added on top of the write-only telemetry
//! fan-out — the bike ships raw SQLite bytes, streamed back through the relay.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::Path;

use sigma_racer_telemetry::{TlsMaterial, pull_database, server_config};

const FAKE_DB: &[u8] = b"SQLite format 3\0...pretend-this-is-a-whole-database...";

fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn generate_pki(out: &Path) {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/gen-telemetry-tls.sh");
    let status = std::process::Command::new("bash")
        .arg(&script)
        .arg(out)
        .arg("IP:127.0.0.1")
        .status()
        .expect("run gen-telemetry-tls.sh");
    assert!(status.success(), "gen-telemetry-tls.sh failed");
}

#[test]
fn database_pull_through_relay() {
    install_crypto();
    let dir = tempfile::tempdir().expect("tempdir");
    let out = dir.path();
    generate_pki(out);

    // Fake bike: a query socket that reads the request and streams DB bytes back.
    let query_sock = out.join("vehicle-query.sock");
    let listener = UnixListener::bind(&query_sock).expect("bind bike query socket");
    let bike = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let mut reader = BufReader::new(stream);
        let mut req = String::new();
        reader.read_line(&mut req).expect("read query");
        assert!(req.contains("DatabaseQuery"), "got: {req}");
        reader.into_inner().write_all(FAKE_DB).expect("respond");
    });

    unsafe {
        std::env::set_var("TELEMETRY_QUERY_SOCKET", &query_sock);
        std::env::set_var("TELEMETRY_TLS_CA", out.join("ca.pem"));
        std::env::set_var("TELEMETRY_TLS_CERT", out.join("client.pem"));
        std::env::set_var("TELEMETRY_TLS_KEY", out.join("client.key"));
    }

    // Reserve a port, then hand the address to the relay.
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);

    let server_material = TlsMaterial::from_paths(
        out.join("ca.pem"),
        out.join("server.pem"),
        out.join("server.key"),
        None,
    );
    let server_tls = server_config(&server_material).expect("server config");
    let telemetry_sock = out.join("vehicle.sock"); // no producer; relay just retries
    std::thread::spawn(move || {
        let _ = sigma_racer_telemetry::run_tls_relay(
            &format!("127.0.0.1:{port}"),
            &telemetry_sock,
            server_tls,
        );
    });

    std::thread::sleep(std::time::Duration::from_millis(300));

    let bytes = pull_database("127.0.0.1", port).expect("database pull");
    assert_eq!(bytes, FAKE_DB);

    bike.join().unwrap();
}
