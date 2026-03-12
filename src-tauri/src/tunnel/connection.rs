use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::client;
use russh::*;
use russh_keys::key;
use tracing::{debug, info, warn};

use crate::config::store::ConfigStore;
use crate::errors::TunnelError;

// ── Pending host key storage ─────────────────────────────────────────

/// Module-level storage for host keys awaiting user acceptance (TOFU).
/// Keyed by "host:port".
static PENDING_HOST_KEYS: std::sync::LazyLock<std::sync::Mutex<HashMap<String, key::PublicKey>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));

/// Accept a previously-rejected unknown host key and save it to known_hosts.
pub fn accept_pending_host_key(host: &str, port: u16) -> Result<(), TunnelError> {
    let map_key = format!("{}:{}", host, port);
    let pubkey = PENDING_HOST_KEYS
        .lock()
        .unwrap()
        .remove(&map_key)
        .ok_or_else(|| {
            TunnelError::SshError(format!("No pending host key for {}:{}", host, port))
        })?;

    russh_keys::known_hosts::learn_known_hosts(host, port, &pubkey)
        .map_err(|e| TunnelError::SshError(format!("Failed to save host key: {}", e)))?;

    info!("Saved host key for {}:{} to known_hosts", host, port);
    Ok(())
}

// ── Host key check result ────────────────────────────────────────────

enum HostKeyCheckResult {
    Unknown(key::PublicKey),
    Changed,
}

// ── SSH client handler ───────────────────────────────────────────────

/// Wrapper around russh client session
pub struct SshConnection {
    session: client::Handle<SshClientHandler>,
}

struct SshClientHandler {
    host: String,
    port: u16,
    check_result: Arc<std::sync::Mutex<Option<HostKeyCheckResult>>>,
}

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        match russh_keys::known_hosts::check_known_hosts(
            &self.host,
            self.port,
            server_public_key,
        ) {
            Ok(true) => {
                debug!("Host key verified for {}:{}", self.host, self.port);
                Ok(true)
            }
            Ok(false) => {
                // Unknown host — store key for potential TOFU acceptance
                info!(
                    "Unknown host key for {}:{} ({})",
                    self.host,
                    self.port,
                    server_public_key.name()
                );
                *self.check_result.lock().unwrap() =
                    Some(HostKeyCheckResult::Unknown(server_public_key.clone()));
                Ok(false)
            }
            Err(russh_keys::Error::KeyChanged { line }) => {
                warn!(
                    "HOST KEY CHANGED for {}:{} at known_hosts line {}!",
                    self.host, self.port, line
                );
                *self.check_result.lock().unwrap() = Some(HostKeyCheckResult::Changed);
                Ok(false)
            }
            Err(e) => {
                warn!("Known hosts check error for {}:{}: {}", self.host, self.port, e);
                // Fail safe — treat as unknown
                *self.check_result.lock().unwrap() =
                    Some(HostKeyCheckResult::Unknown(server_public_key.clone()));
                Ok(false)
            }
        }
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
        // Enable russh's built-in keepalive (sends keepalive@openssh.com global request).
        // HealthMonitor checks is_closed() to detect when the connection drops.
        let config = client::Config {
            inactivity_timeout: Some(Duration::from_secs(timeout_secs * 3)),
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        };

        let check_result = Arc::new(std::sync::Mutex::new(None));
        let handler = SshClientHandler {
            host: host.to_string(),
            port,
            check_result: check_result.clone(),
        };

        // Connect with timeout
        let addr = format!("{}:{}", host, port);
        let mut session = match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client::connect(Arc::new(config), &addr, handler),
        )
        .await
        {
            Ok(Ok(session)) => session,
            Ok(Err(e)) => {
                // Check if this was a host key rejection
                if let Some(result) = check_result.lock().unwrap().take() {
                    match result {
                        HostKeyCheckResult::Unknown(pubkey) => {
                            let fingerprint = pubkey.fingerprint();
                            let key_type = pubkey.name().to_string();
                            // Store for later acceptance
                            let map_key = format!("{}:{}", host, port);
                            PENDING_HOST_KEYS
                                .lock()
                                .unwrap()
                                .insert(map_key, pubkey);
                            return Err(TunnelError::HostKeyUnknown {
                                host: host.to_string(),
                                port,
                                key_type,
                                fingerprint,
                            });
                        }
                        HostKeyCheckResult::Changed => {
                            return Err(TunnelError::HostKeyChanged {
                                host: host.to_string(),
                                port,
                            });
                        }
                    }
                }
                return Err(TunnelError::SshError(format!(
                    "Connection failed: {}",
                    e
                )));
            }
            Err(_) => return Err(TunnelError::ConnectionTimeout),
        };

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
    /// Relies on russh's built-in keepalive (keepalive@openssh.com global request)
    /// to detect dead connections. We just check the sender state.
    pub fn is_alive(&self) -> bool {
        !self.session.is_closed()
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
