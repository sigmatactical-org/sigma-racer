//! PEM material discovery for the private Sigma telemetry PKI.

use super::pin::CertPin;
use super::role::TlsRole;
use std::path::PathBuf;

/// PEM paths for the private Sigma telemetry PKI.
#[derive(Debug, Clone)]
pub struct TlsMaterial {
    /// Telemetry CA bundle.
    pub ca_path: PathBuf,
    /// This side's certificate chain.
    pub cert_path: PathBuf,
    /// This side's private key.
    pub key_path: PathBuf,
    /// Optional SHA-256 (hex) of the Wingman server leaf certificate DER (client only).
    pub server_pin: Option<CertPin>,
}

impl TlsMaterial {
    /// Resolve material for `role` from env overrides / default paths.
    pub fn load(role: TlsRole) -> Result<Self, String> {
        load_material(role)
    }

    /// Build material from explicit PEM paths (tests and provisioning tools).
    pub fn from_paths(
        ca_path: PathBuf,
        cert_path: PathBuf,
        key_path: PathBuf,
        server_pin: Option<CertPin>,
    ) -> Self {
        Self {
            ca_path,
            cert_path,
            key_path,
            server_pin,
        }
    }
}

/// Resolve PEM paths for `role` (env overrides win) and require each file to
/// exist — telemetry refuses to run without mTLS material.
pub fn load_material(role: TlsRole) -> Result<TlsMaterial, String> {
    let (ca_path, cert_path, key_path) = match role {
        TlsRole::Server => (
            env_path("TELEMETRY_TLS_CA", default_server_ca()),
            env_path("TELEMETRY_TLS_CERT", default_server_cert()),
            env_path("TELEMETRY_TLS_KEY", default_server_key()),
        ),
        TlsRole::Client => (
            env_path("TELEMETRY_TLS_CA", default_client_ca()),
            env_path("TELEMETRY_TLS_CERT", default_client_cert()),
            env_path("TELEMETRY_TLS_KEY", default_client_key()),
        ),
    };

    for path in [&ca_path, &cert_path, &key_path] {
        if !path.is_file() {
            return Err(format!(
                "TLS material missing: {} (telemetry requires TLS 1.3 mTLS)",
                path.display()
            ));
        }
    }

    let server_pin = std::env::var("TELEMETRY_TLS_SERVER_PIN")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| CertPin::parse_hex(&s))
        .transpose()?;

    Ok(TlsMaterial {
        ca_path,
        cert_path,
        key_path,
        server_pin,
    })
}

/// Path from `var`, falling back to `default` when unset/empty.
fn env_path(var: &str, default: PathBuf) -> PathBuf {
    std::env::var(var)
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or(default)
}

fn default_server_ca() -> PathBuf {
    PathBuf::from("/etc/sigma-racer-wingman/telemetry-tls/ca.pem")
}

fn default_server_cert() -> PathBuf {
    PathBuf::from("/etc/sigma-racer-wingman/telemetry-tls/server.pem")
}

fn default_server_key() -> PathBuf {
    PathBuf::from("/etc/sigma-racer-wingman/telemetry-tls/server.key")
}

fn default_client_ca() -> PathBuf {
    home_config_dir().join("tls").join("ca.pem")
}

fn default_client_cert() -> PathBuf {
    home_config_dir().join("tls").join("client.pem")
}

fn default_client_key() -> PathBuf {
    home_config_dir().join("tls").join("client.key")
}

/// Mechanic's config dir: `SIGMA_RACER_MECHANIC_CONFIG_DIR`, else
/// `~/.config/sigma-racer-mechanic`.
fn home_config_dir() -> PathBuf {
    std::env::var("SIGMA_RACER_MECHANIC_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME").map(|h| {
                PathBuf::from(h)
                    .join(".config")
                    .join("sigma-racer-mechanic")
            })
        })
        .unwrap_or_else(|_| PathBuf::from(".config/sigma-racer-mechanic"))
}
