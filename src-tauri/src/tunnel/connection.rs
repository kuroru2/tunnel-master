use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use russh::client;
use russh::*;
use russh_keys::key;
use tokio::io::{AsyncRead, AsyncWrite};
use tracing::{debug, info, warn};

use crate::config::store::ConfigStore;
use crate::errors::TunnelError;

/// Slot for keyboard-interactive response synchronization.
pub type KiResponseSlot = Arc<std::sync::Mutex<Option<tokio::sync::oneshot::Sender<Vec<String>>>>>;

/// Authentication credentials for SSH connections.
pub enum AuthCredentials {
    Key {
        key_path: String,
        passphrase: Option<String>,
    },
    Password(String),
    Agent,
    KeyboardInteractive {
        ki_slot: KiResponseSlot,
        app_handle: tauri::AppHandle,
        tunnel_id: String,
    },
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardInteractivePrompt {
    pub tunnel_id: String,
    pub name: String,
    pub instructions: String,
    pub prompts: Vec<KiPromptEntry>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KiPromptEntry {
    pub text: String,
    pub echo: bool,
}

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
    ki_slot: KiResponseSlot,
    app_handle: Option<tauri::AppHandle>,
    tunnel_id: String,
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
    /// Connect to an SSH server and authenticate with the given credentials.
    pub async fn connect(
        host: &str,
        port: u16,
        user: &str,
        credentials: AuthCredentials,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        info!("Connecting to {}@{}:{}", user, host, port);

        // Extract ki_slot, app_handle, tunnel_id from credentials for the handler
        let (ki_slot, app_handle, tunnel_id) = match &credentials {
            AuthCredentials::KeyboardInteractive {
                ki_slot,
                app_handle,
                tunnel_id,
            } => (ki_slot.clone(), Some(app_handle.clone()), tunnel_id.clone()),
            _ => (
                Arc::new(std::sync::Mutex::new(None)),
                None,
                String::new(),
            ),
        };

        // Configure the SSH client
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
            ki_slot,
            app_handle,
            tunnel_id,
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
                return Self::handle_host_key_error(host, port, check_result, e);
            }
            Err(_) => return Err(TunnelError::ConnectionTimeout),
        };

        // Authenticate
        Self::authenticate(&mut session, user, credentials).await?;

        info!("SSH connection established to {}:{}", host, port);
        Ok(Self { session })
    }

    /// Connect to an SSH server over an existing stream (e.g. a ProxyJump channel).
    pub async fn connect_stream<R: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        stream: R,
        host: &str,
        port: u16,
        user: &str,
        credentials: AuthCredentials,
        timeout_secs: u64,
    ) -> Result<Self, TunnelError> {
        info!("Connecting to {}@{}:{} via stream", user, host, port);

        // Extract ki_slot, app_handle, tunnel_id from credentials for the handler
        let (ki_slot, app_handle, tunnel_id) = match &credentials {
            AuthCredentials::KeyboardInteractive {
                ki_slot,
                app_handle,
                tunnel_id,
            } => (ki_slot.clone(), Some(app_handle.clone()), tunnel_id.clone()),
            _ => (
                Arc::new(std::sync::Mutex::new(None)),
                None,
                String::new(),
            ),
        };

        // Configure the SSH client
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
            ki_slot,
            app_handle,
            tunnel_id,
        };

        // Connect over stream with timeout
        let mut session = match tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            client::connect_stream(Arc::new(config), stream, handler),
        )
        .await
        {
            Ok(Ok(session)) => session,
            Ok(Err(e)) => {
                return Self::handle_host_key_error(host, port, check_result, e);
            }
            Err(_) => return Err(TunnelError::ConnectionTimeout),
        };

        // Authenticate
        Self::authenticate(&mut session, user, credentials).await?;

        info!("SSH connection established to {}:{} via stream", host, port);
        Ok(Self { session })
    }

    /// Handle host key errors from a failed connection attempt.
    fn handle_host_key_error(
        host: &str,
        port: u16,
        check_result: Arc<std::sync::Mutex<Option<HostKeyCheckResult>>>,
        e: russh::Error,
    ) -> Result<Self, TunnelError> {
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
        Err(TunnelError::SshError(format!(
            "Connection failed: {}",
            e
        )))
    }

    /// Dispatch authentication based on the credential type.
    async fn authenticate(
        session: &mut client::Handle<SshClientHandler>,
        user: &str,
        credentials: AuthCredentials,
    ) -> Result<(), TunnelError> {
        match credentials {
            AuthCredentials::Key {
                key_path,
                passphrase,
            } => {
                let expanded_key_path = ConfigStore::expand_tilde(&key_path);
                let key_pair =
                    russh_keys::load_secret_key(&expanded_key_path, passphrase.as_deref())
                        .map_err(|e| {
                            TunnelError::AuthFailed(format!("Failed to load key: {}", e))
                        })?;

                let auth_result = session
                    .authenticate_publickey(user, Arc::new(key_pair))
                    .await
                    .map_err(|e| TunnelError::AuthFailed(format!("Auth error: {}", e)))?;

                if !auth_result {
                    return Err(TunnelError::AuthFailed(
                        "Server rejected public key".to_string(),
                    ));
                }
            }
            AuthCredentials::Password(password) => {
                let auth_result = session
                    .authenticate_password(user, &password)
                    .await
                    .map_err(|e| TunnelError::AuthFailed(format!("Auth error: {}", e)))?;

                if !auth_result {
                    return Err(TunnelError::AuthFailed(
                        "Server rejected password".to_string(),
                    ));
                }
            }
            AuthCredentials::Agent => {
                Self::authenticate_with_agent(session, user).await?;
            }
            AuthCredentials::KeyboardInteractive {
                ki_slot,
                app_handle,
                tunnel_id,
            } => {
                use tauri::Emitter;

                let mut response = session
                    .authenticate_keyboard_interactive_start(user, None)
                    .await
                    .map_err(|e| TunnelError::AuthFailed(format!("Auth error: {}", e)))?;

                loop {
                    match response {
                        client::KeyboardInteractiveAuthResponse::Success => break,
                        client::KeyboardInteractiveAuthResponse::Failure => {
                            return Err(TunnelError::AuthFailed(
                                "Keyboard-interactive authentication failed".to_string(),
                            ));
                        }
                        client::KeyboardInteractiveAuthResponse::InfoRequest {
                            name,
                            instructions,
                            prompts,
                        } => {
                            let (tx, rx) = tokio::sync::oneshot::channel();
                            *ki_slot.lock().unwrap() = Some(tx);

                            let prompt = KeyboardInteractivePrompt {
                                tunnel_id: tunnel_id.clone(),
                                name,
                                instructions,
                                prompts: prompts
                                    .iter()
                                    .map(|p| KiPromptEntry {
                                        text: p.prompt.clone(),
                                        echo: p.echo,
                                    })
                                    .collect(),
                            };
                            let _ = app_handle.emit("keyboard-interactive-prompt", &prompt);

                            let answers = rx.await.map_err(|_| {
                                TunnelError::AuthFailed(
                                    "Keyboard-interactive dialog cancelled".to_string(),
                                )
                            })?;

                            response = session
                                .authenticate_keyboard_interactive_respond(answers)
                                .await
                                .map_err(|e| {
                                    TunnelError::AuthFailed(format!("Auth error: {}", e))
                                })?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Authenticate using the SSH agent.
    async fn authenticate_with_agent(
        session: &mut client::Handle<SshClientHandler>,
        user: &str,
    ) -> Result<(), TunnelError> {
        #[cfg(unix)]
        let mut agent = russh_keys::agent::client::AgentClient::connect_env()
            .await
            .map_err(|e| {
                TunnelError::AgentUnavailable(format!(
                    "SSH agent not available — {}. Try launching from a terminal or ensure your agent is running.",
                    e
                ))
            })?;

        #[cfg(windows)]
        let mut agent = {
            let pipe = tokio::net::windows::named_pipe::ClientOptions::new()
                .open(r"\\.\pipe\openssh-ssh-agent")
                .map_err(|e| {
                    TunnelError::AgentUnavailable(format!("SSH agent not available — {}", e))
                })?;
            russh_keys::agent::client::AgentClient::connect(pipe)
        };

        let identities = agent.request_identities().await.map_err(|e| {
            TunnelError::AgentUnavailable(format!("Failed to list agent keys: {}", e))
        })?;

        if identities.is_empty() {
            return Err(TunnelError::AgentUnavailable(
                "SSH agent has no keys loaded".to_string(),
            ));
        }

        let mut accepted = false;
        for pubkey in identities {
            let (returned_agent, result) = session.authenticate_future(user, pubkey, agent).await;
            agent = returned_agent;
            match result {
                Ok(true) => {
                    accepted = true;
                    break;
                }
                Ok(false) => continue,
                Err(e) => {
                    return Err(TunnelError::AuthFailed(format!("Agent auth error: {}", e)))
                }
            }
        }

        if !accepted {
            return Err(TunnelError::AuthFailed(
                "No agent key accepted by server".to_string(),
            ));
        }
        Ok(())
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
