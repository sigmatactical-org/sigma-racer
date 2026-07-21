//! TLS 1.3 + mTLS for Wingman telemetry relay and Mechanic clients.

mod config;
mod material;
mod pem;
mod pin;
mod role;
mod stream;

pub use config::{client_config, server_config, server_name_for_host};
pub use material::{TlsMaterial, load_material};
pub use pem::{load_certs, load_private_key};
pub use pin::{CertPin, verify_server_pin};
pub use role::TlsRole;
pub use stream::{
    TlsClientStream, TlsServerStream, accept_tls, connect_tls, set_nonblocking, set_read_timeout,
    tls_read_timeout, tls_set_nonblocking,
};
