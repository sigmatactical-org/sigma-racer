//! Optional server certificate pinning (SHA-256 of leaf cert DER).

use rustls::pki_types::CertificateDer;
use std::fmt;

/// SHA-256 fingerprint of the Wingman server leaf certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CertPin([u8; 32]);

impl CertPin {
    /// Parse a 64-char hex SHA-256 into a pin.
    pub fn parse_hex(hex: &str) -> Result<Self, String> {
        let hex = hex.trim().trim_start_matches("sha256:");
        if hex.len() != 64 {
            return Err("TELEMETRY_TLS_SERVER_PIN must be 64 hex chars (SHA-256)".into());
        }
        let mut out = [0u8; 32];
        for (i, chunk) in out.iter_mut().enumerate() {
            *chunk = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| "invalid hex in TELEMETRY_TLS_SERVER_PIN")?;
        }
        Ok(Self(out))
    }

    /// Pin of a certificate: SHA-256 over its DER encoding.
    pub fn from_cert_der(cert: &CertificateDer<'_>) -> Self {
        use sha2::{Digest, Sha256};
        let digest = Sha256::digest(cert.as_ref());
        let mut out = [0u8; 32];
        out.copy_from_slice(&digest);
        Self(out)
    }
}

impl fmt::Display for CertPin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

/// Check the server leaf certificate against the expected pin.
pub fn verify_server_pin(certs: &[CertificateDer<'_>], pin: &CertPin) -> Result<(), String> {
    let leaf = certs
        .first()
        .ok_or_else(|| "TLS server presented no certificate".to_string())?;
    let actual = CertPin::from_cert_der(leaf);
    if actual != *pin {
        return Err(format!(
            "TLS server certificate pin mismatch (expected {}, got {actual})",
            pin
        ));
    }
    Ok(())
}
