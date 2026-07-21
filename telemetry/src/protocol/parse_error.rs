//! Telemetry frame parse failures.

#[derive(Debug)]
pub enum ParseError {
    Json(serde_json::Error),
    UnsupportedVersion(String),
    UnknownKind(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(err) => write!(f, "{err}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported protocol version {v}"),
            Self::UnknownKind(kind) => write!(f, "unknown message kind {kind}"),
        }
    }
}
