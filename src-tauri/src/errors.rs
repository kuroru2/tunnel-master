use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
pub enum TunnelError {
    #[error("Config file not found")]
    ConfigNotFound,

    #[error("Invalid config: {0}")]
    ConfigInvalid(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Port {0} is already in use")]
    PortInUse(u16),

    #[error("Connection timed out")]
    ConnectionTimeout,

    #[error("SSH error: {0}")]
    SshError(String),

    #[error("UNKNOWN_HOST_KEY:{host}:{port}:{key_type}:{fingerprint}")]
    HostKeyUnknown {
        host: String,
        port: u16,
        key_type: String,
        fingerprint: String,
    },

    #[error("HOST_KEY_CHANGED: Host key for {host}:{port} has changed! This could indicate a man-in-the-middle attack.")]
    HostKeyChanged {
        host: String,
        port: u16,
    },

    #[error("Tunnel not found: {0}")]
    TunnelNotFound(String),
}

impl From<TunnelError> for String {
    fn from(e: TunnelError) -> String {
        e.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        assert_eq!(
            TunnelError::PortInUse(5432).to_string(),
            "Port 5432 is already in use"
        );
        assert_eq!(
            TunnelError::TunnelNotFound("abc".into()).to_string(),
            "Tunnel not found: abc"
        );
    }

    #[test]
    fn error_serializes_to_json() {
        let err = TunnelError::ConnectionTimeout;
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("ConnectionTimeout"));
    }
}
