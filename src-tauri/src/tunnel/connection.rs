use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::client;
use russh::*;
use russh_keys::key;
use tracing::{debug, info};

use crate::config::store::ConfigStore;
use crate::errors::TunnelError;

/// Wrapper around russh client session
pub struct SshConnection {
    session: client::Handle<SshClientHandler>,
}

/// Handler for russh client callbacks
struct SshClientHandler;

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // TODO: In production, verify against known_hosts.
        // For POC, accept all keys.
        Ok(true)
    }
}

impl SshConnection {
    /// Connect to an SSH server and authenticate with a key file.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        key_path: &str,
        passphrase: Option<&str>,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        let expanded_key_path = ConfigStore::expand_tilde(key_path);

        info!("Connecting to {}@{}:{}", user, host, port);

        // Load the private key
        let key_pair = russh_keys::load_secret_key(&expanded_key_path, passphrase)
            .map_err(|e| TunnelError::AuthFailed(format!("Failed to load key: {}", e)))?;

        // Configure the SSH client
        // Use russh's built-in keepalive mechanism
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(timeout_secs * 3)),
            keepalive_interval: None, // We handle keepalive ourselves via HealthMonitor
            ..Default::default()
        };

        // Connect with timeout
        let addr = format!("{}:{}", host, port);
        let mut session = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client::connect(Arc::new(config), &addr, SshClientHandler),
        )
        .await
        .map_err(|_| TunnelError::ConnectionTimeout)?
        .map_err(|e| TunnelError::SshError(format!("Connection failed: {}", e)))?;

        // Authenticate
        let auth_result = session
            .authenticate_publickey(user, Arc::new(key_pair))
            .await
            .map_err(|e| TunnelError::AuthFailed(format!("Auth error: {}", e)))?;

        if !auth_result {
            return Err(TunnelError::AuthFailed(
                "Server rejected public key".to_string(),
            ));
        }

        info!("SSH connection established to {}:{}", host, port);
        Ok(Self { session })
    }

    /// Request a direct-tcpip channel for local port forwarding.
    pub async fn open_direct_tcpip(
        &self,
        remote_host: &str,
        remote_port: u16,
        local_host: &str,
        local_port: u16,
    ) -> Result<Channel<client::Msg>, TunnelError> {
        let channel = self
            .session
            .channel_open_direct_tcpip(
                remote_host,
                remote_port.into(),
                local_host,
                local_port.into(),
            )
            .await
            .map_err(|e| TunnelError::SshError(format!("Failed to open channel: {}", e)))?;

        debug!(
            "Opened direct-tcpip channel: {}:{} -> {}:{}",
            local_host, local_port, remote_host, remote_port
        );
        Ok(channel)
    }

    /// Check if the SSH connection is still alive.
    /// Uses a lightweight probe: attempts to open and immediately close a session channel.
    /// This verifies the SSH session is responsive end-to-end.
    pub async fn send_keepalive(&self) -> Result<(), TunnelError> {
        // Check if the underlying sender is closed first (cheap check)
        if self.session.is_closed() {
            return Err(TunnelError::SshError(
                "SSH session is closed".to_string(),
            ));
        }

        // Try to open a session channel as a health probe
        let channel = self
            .session
            .channel_open_session()
            .await
            .map_err(|e| TunnelError::SshError(format!("Keepalive failed: {}", e)))?;

        // Close the probe channel immediately
        let _ = channel.close().await;

        Ok(())
    }

    /// Disconnect the SSH session.
    pub async fn disconnect(&self) {
        let _ = self
            .session
            .disconnect(Disconnect::ByApplication, "Client disconnecting", "en")
            .await;
        info!("SSH session disconnected");
    }
}
