//! rustls config builders (TLS 1.3 + mTLS only).

use super::material::TlsMaterial;
use super::pem::{load_certs, load_private_key};
use rustls::RootCertStore;
use rustls::pki_types::ServerName;
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, ServerConfig, version::TLS13};
use std::net::IpAddr;
use std::sync::Arc;

/// Server config requiring TLS 1.3 and a client certificate signed by the
/// telemetry CA (no anonymous clients).
pub fn server_config(material: &TlsMaterial) -> Result<Arc<ServerConfig>, String> {
    let roots = ca_roots(material)?;
    let client_verifier = WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .map_err(|e| format!("client verifier: {e}"))?;

    let certs = load_certs(&material.cert_path)?;
    let key = load_private_key(&material.key_path)?;

    let config = ServerConfig::builder_with_protocol_versions(&[&TLS13])
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)
        .map_err(|e| format!("server TLS config: {e}"))?;

    Ok(Arc::new(config))
}

/// Client config presenting the client certificate and trusting only the
/// telemetry CA, TLS 1.3 only.
pub fn client_config(material: &TlsMaterial) -> Result<Arc<ClientConfig>, String> {
    let roots = ca_roots(material)?;
    let client_certs = load_certs(&material.cert_path)?;
    let client_key = load_private_key(&material.key_path)?;

    let config = ClientConfig::builder_with_protocol_versions(&[&TLS13])
        .with_root_certificates(roots)
        .with_client_auth_cert(client_certs, client_key)
        .map_err(|e| format!("client TLS config: {e}"))?;

    Ok(Arc::new(config))
}

/// SNI value for `host`: an IP address when it parses as one, else a DNS name.
pub fn server_name_for_host(host: &str) -> Result<ServerName<'static>, String> {
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ServerName::IpAddress(ip.into()));
    }
    ServerName::try_from(host.to_string()).map_err(|_| format!("invalid TLS server name: {host}"))
}

/// Load the telemetry CA bundle into a root store.
fn ca_roots(material: &TlsMaterial) -> Result<RootCertStore, String> {
    let ca = load_certs(&material.ca_path)?;
    let mut roots = RootCertStore::empty();
    for cert in ca {
        roots
            .add(cert)
            .map_err(|e| format!("invalid CA {}: {e}", material.ca_path.display()))?;
    }
    Ok(roots)
}
