use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

use crate::config::store::ConfigStore;
use crate::errors::TunnelError;
use crate::keychain;
use crate::tunnel::connection::{AuthCredentials, KiResponseSlot, SshConnection};
use crate::tunnel::forwarder::PortForwarder;
use crate::tunnel::health::HealthMonitor;
use crate::types::{AppConfig, AuthMethod, TunnelConfig, TunnelInfo, TunnelStatus, TunnelStatusEvent};

/// Messages the TunnelManager actor receives
#[derive(Debug)]
pub enum ManagerCommand {
    ListTunnels {
        reply: oneshot::Sender<Vec<TunnelInfo>>,
    },
    Connect {
        id: String,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    Disconnect {
        id: String,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    ReloadConfig {
        config: AppConfig,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    GetKeyPath {
        id: String,
        reply: oneshot::Sender<Result<String, TunnelError>>,
    },
    TunnelDied {
        id: String,
        error: String,
        generation: u64,
    },
    AddTunnel {
        config: TunnelConfig,
        reply: oneshot::Sender<Result<TunnelInfo, TunnelError>>,
    },
    UpdateTunnel {
        config: TunnelConfig,
        reply: oneshot::Sender<Result<TunnelInfo, TunnelError>>,
    },
    RemoveTunnel {
        id: String,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    GetTunnelConfig {
        id: String,
        reply: oneshot::Sender<Result<TunnelConfig, TunnelError>>,
    },
    RespondKeyboardInteractive {
        id: String,
        responses: Vec<String>,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    CancelKeyboardInteractive {
        id: String,
        reply: oneshot::Sender<Result<(), TunnelError>>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

/// Runtime state for a single tunnel
struct TunnelState {
    config: TunnelConfig,
    status: TunnelStatus,
    error_message: Option<String>,
    /// Handles to abort the tunnel's background tasks on disconnect
    abort_handles: Vec<tokio::task::AbortHandle>,
    /// Active SSH connection, if connected
    ssh_connection: Option<Arc<SshConnection>>,
    /// Jump host SSH connection, if connected via ProxyJump
    jump_connection: Option<Arc<SshConnection>>,
    /// Keyboard-interactive response slot
    ki_slot: Option<KiResponseSlot>,
    /// Generation counter to detect stale TunnelDied messages
    generation: u64,
}

impl TunnelState {
    fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            status: TunnelStatus::Disconnected,
            error_message: None,
            abort_handles: Vec::new(),
            ssh_connection: None,
            jump_connection: None,
            ki_slot: None,
            generation: 0,
        }
    }
}

pub type ManagerHandle = mpsc::Sender<ManagerCommand>;

/// Spawn the TunnelManager actor. Returns a sender to communicate with it.
pub fn spawn_manager(
    config: AppConfig,
    event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
    error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
    app_handle: Option<tauri::AppHandle>,
) -> ManagerHandle {
    let (tx, rx) = mpsc::channel(32);
    let manager_tx = tx.clone();

    tauri::async_runtime::spawn(async move {
        let mut manager = TunnelManagerActor::new(config, event_tx, error_tx, manager_tx, app_handle);
        manager.run(rx).await;
    });

    tx
}

struct TunnelManagerActor {
    tunnels: HashMap<String, TunnelState>,
    event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
    error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
    manager_tx: mpsc::Sender<ManagerCommand>,
    settings: crate::types::Settings,
    app_handle: Option<tauri::AppHandle>,
}

impl TunnelManagerActor {
    fn new(
        config: AppConfig,
        event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
        error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
        manager_tx: mpsc::Sender<ManagerCommand>,
        app_handle: Option<tauri::AppHandle>,
    ) -> Self {
        let mut tunnels = HashMap::new();
        for tc in config.tunnels {
            tunnels.insert(tc.id.clone(), TunnelState::new(tc));
        }

        Self {
            tunnels,
            event_tx,
            error_tx,
            manager_tx,
            settings: config.settings,
            app_handle,
        }
    }

    /// Build a TunnelInfo from a TunnelState, resolving the jump host name
    fn tunnel_to_info(&self, tunnel: &TunnelState) -> TunnelInfo {
        let jump_host_name = tunnel.config.jump_host.as_ref().and_then(|jump_id| {
            self.tunnels.get(jump_id).map(|jt| jt.config.name.clone())
        });
        TunnelInfo {
            id: tunnel.config.id.clone(),
            name: tunnel.config.name.clone(),
            status: tunnel.status.clone(),
            local_port: tunnel.config.local_port,
            remote_host: tunnel.config.remote_host.clone(),
            remote_port: tunnel.config.remote_port,
            error_message: tunnel.error_message.clone(),
            auth_method: tunnel.config.auth_method.clone(),
            jump_host_name,
        }
    }

    /// Validate that a jump host chain doesn't loop or exceed max depth.
    fn validate_jump_chain(&self, tunnel_id: &str) -> Result<(), TunnelError> {
        let mut visited = HashSet::new();
        visited.insert(tunnel_id.to_string());
        let mut current_id = tunnel_id.to_string();

        for _depth in 0..5 {
            let jump_host = match self.tunnels.get(&current_id) {
                Some(t) => t.config.jump_host.clone(),
                None => return Err(TunnelError::TunnelNotFound(current_id)),
            };
            match jump_host {
                None => return Ok(()), // end of chain
                Some(jump_id) => {
                    if !self.tunnels.contains_key(&jump_id) {
                        return Err(TunnelError::JumpHostNotFound(jump_id));
                    }
                    if !visited.insert(jump_id.clone()) {
                        return Err(TunnelError::ConfigInvalid(
                            format!("Jump host chain contains a loop involving '{}'", jump_id),
                        ));
                    }
                    current_id = jump_id;
                }
            }
        }
        Err(TunnelError::ConfigInvalid(
            "Jump host chain exceeds maximum depth of 5".to_string(),
        ))
    }

    /// Build AuthCredentials from a tunnel's auth_method
    fn build_credentials(&mut self, tunnel_id: &str) -> Result<AuthCredentials, TunnelError> {
        let tunnel = self.tunnels.get(tunnel_id)
            .ok_or_else(|| TunnelError::TunnelNotFound(tunnel_id.to_string()))?;

        match &tunnel.config.auth_method {
            AuthMethod::Key => {
                let expanded_key_path = ConfigStore::expand_tilde(&tunnel.config.key_path);
                let passphrase = keychain::get_passphrase(
                    expanded_key_path.to_string_lossy().as_ref(),
                );
                Ok(AuthCredentials::Key {
                    key_path: tunnel.config.key_path.clone(),
                    passphrase,
                })
            }
            AuthMethod::Password => {
                match keychain::get_password(tunnel_id) {
                    Some(password) => Ok(AuthCredentials::Password(password)),
                    None => Err(TunnelError::PasswordRequired(tunnel_id.to_string())),
                }
            }
            AuthMethod::Agent => Ok(AuthCredentials::Agent),
            AuthMethod::KeyboardInteractive => {
                let app_handle = self.app_handle.clone().ok_or_else(|| {
                    TunnelError::AuthFailed(
                        "Keyboard-interactive auth requires an app handle".to_string(),
                    )
                })?;
                let ki_slot: KiResponseSlot = Arc::new(std::sync::Mutex::new(None));
                // Store the slot in the tunnel state
                let tunnel = self.tunnels.get_mut(tunnel_id).unwrap();
                tunnel.ki_slot = Some(ki_slot.clone());
                Ok(AuthCredentials::KeyboardInteractive {
                    ki_slot,
                    app_handle,
                    tunnel_id: tunnel_id.to_string(),
                })
            }
        }
    }

    /// Connect to a destination via a jump host using direct-tcpip channel.
    async fn connect_via_jump(
        &mut self,
        id: &str,
        jump_id: &str,
    ) -> Result<Arc<SshConnection>, TunnelError> {
        // Build credentials and connect to the jump host
        let jump_credentials = self.build_credentials(jump_id)?;
        let (jump_host, jump_port, jump_user) = {
            let jt = self.tunnels.get(jump_id).unwrap();
            (jt.config.host.clone(), jt.config.port, jt.config.user.clone())
        };

        let timeout_secs = self.settings.connection_timeout_secs;

        let jump_ssh = SshConnection::connect(
            &jump_host,
            jump_port,
            &jump_user,
            jump_credentials,
            timeout_secs,
        )
        .await
        .map_err(|e| TunnelError::JumpHostFailed(format!("Failed to connect to jump host: {}", e)))?;

        let jump_ssh = Arc::new(jump_ssh);

        // Open direct-tcpip channel to the destination
        let (dest_host, dest_port, dest_user) = {
            let t = self.tunnels.get(id).unwrap();
            (t.config.host.clone(), t.config.port, t.config.user.clone())
        };

        let channel = jump_ssh
            .open_direct_tcpip(&dest_host, dest_port, "127.0.0.1", 0)
            .await
            .map_err(|e| TunnelError::JumpHostFailed(format!("Failed to open channel through jump host: {}", e)))?;

        let stream = channel.into_stream();

        // Build credentials for the destination
        let dest_credentials = self.build_credentials(id)?;

        let dest_ssh = SshConnection::connect_stream(
            stream,
            &dest_host,
            dest_port,
            &dest_user,
            dest_credentials,
            timeout_secs,
        )
        .await?;

        // Store the jump connection
        let tunnel = self.tunnels.get_mut(id).unwrap();
        tunnel.jump_connection = Some(jump_ssh);

        Ok(Arc::new(dest_ssh))
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<ManagerCommand>) {
        info!("TunnelManager started with {} tunnels", self.tunnels.len());

        while let Some(cmd) = rx.recv().await {
            match cmd {
                ManagerCommand::ListTunnels { reply } => {
                    let infos: Vec<TunnelInfo> =
                        self.tunnels.values().map(|t| self.tunnel_to_info(t)).collect();
                    let _ = reply.send(infos);
                }

                ManagerCommand::Connect { id, reply } => {
                    let result = self.handle_connect(&id).await;
                    let _ = reply.send(result);
                }

                ManagerCommand::Disconnect { id, reply } => {
                    let result = self.handle_disconnect(&id).await;
                    let _ = reply.send(result);
                }

                ManagerCommand::ReloadConfig { config, reply } => {
                    let result = self.handle_reload(config);
                    let _ = reply.send(result);
                }

                ManagerCommand::GetKeyPath { id, reply } => {
                    let result = match self.tunnels.get(&id) {
                        Some(t) => Ok(t.config.key_path.clone()),
                        None => Err(TunnelError::TunnelNotFound(id)),
                    };
                    let _ = reply.send(result);
                }

                ManagerCommand::TunnelDied { id, error, generation } => {
                    self.handle_tunnel_died(&id, &error, generation).await;
                }

                ManagerCommand::AddTunnel { config, reply } => {
                    let result = self.handle_add_tunnel(config);
                    let _ = reply.send(result);
                }

                ManagerCommand::UpdateTunnel { config, reply } => {
                    let result = self.handle_update_tunnel(config).await;
                    let _ = reply.send(result);
                }

                ManagerCommand::RemoveTunnel { id, reply } => {
                    let result = self.handle_remove_tunnel(&id).await;
                    let _ = reply.send(result);
                }

                ManagerCommand::GetTunnelConfig { id, reply } => {
                    let result = match self.tunnels.get(&id) {
                        Some(t) => Ok(t.config.clone()),
                        None => Err(TunnelError::TunnelNotFound(id)),
                    };
                    let _ = reply.send(result);
                }

                ManagerCommand::RespondKeyboardInteractive { id, responses, reply } => {
                    let result = self.handle_respond_ki(&id, responses);
                    let _ = reply.send(result);
                }

                ManagerCommand::CancelKeyboardInteractive { id, reply } => {
                    let result = self.handle_cancel_ki(&id);
                    let _ = reply.send(result);
                }

                ManagerCommand::Shutdown { reply } => {
                    info!("TunnelManager received shutdown command");
                    self.disconnect_all().await;
                    let _ = reply.send(());
                    return; // Exit the run loop
                }
            }
        }

        info!("TunnelManager shutting down, disconnecting all tunnels");
        self.disconnect_all().await;
    }

    fn handle_respond_ki(&mut self, id: &str, responses: Vec<String>) -> Result<(), TunnelError> {
        let tunnel = self.tunnels.get_mut(id)
            .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

        if let Some(ki_slot) = &tunnel.ki_slot {
            if let Some(tx) = ki_slot.lock().unwrap().take() {
                let _ = tx.send(responses);
                Ok(())
            } else {
                Err(TunnelError::SshError("No pending keyboard-interactive prompt".to_string()))
            }
        } else {
            Err(TunnelError::SshError("No keyboard-interactive session active".to_string()))
        }
    }

    fn handle_cancel_ki(&mut self, id: &str) -> Result<(), TunnelError> {
        let tunnel = self.tunnels.get_mut(id)
            .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

        // Drop the ki_slot, which will cause the oneshot receiver to get a RecvError
        tunnel.ki_slot.take();
        Ok(())
    }

    async fn handle_connect(&mut self, id: &str) -> Result<(), TunnelError> {
        // Check tunnel exists and is not already connected
        {
            let tunnel = self
                .tunnels
                .get_mut(id)
                .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

            if tunnel.status == TunnelStatus::Connected
                || tunnel.status == TunnelStatus::Connecting
            {
                debug!("Tunnel {} already connected/connecting", id);
                return Ok(());
            }

            tunnel.status = TunnelStatus::Connecting;
            tunnel.error_message = None;
        }
        self.emit_status(id, &TunnelStatus::Connecting);

        // Validate jump chain before attempting connection
        if let Err(e) = self.validate_jump_chain(id) {
            let tunnel = self.tunnels.get_mut(id).unwrap();
            tunnel.status = TunnelStatus::Disconnected;
            tunnel.error_message = Some(e.to_string());
            self.emit_status(id, &TunnelStatus::Disconnected);
            return Err(e);
        }

        let timeout_secs = self.settings.connection_timeout_secs;
        let keepalive_interval = self.settings.keepalive_interval_secs;
        let keepalive_timeout = self.settings.keepalive_timeout_secs;

        // Extract what we need for the connection
        let (local_port, remote_host, remote_port, jump_host) = {
            let tunnel = self.tunnels.get(id).unwrap();
            (
                tunnel.config.local_port,
                tunnel.config.remote_host.clone(),
                tunnel.config.remote_port,
                tunnel.config.jump_host.clone(),
            )
        };

        // Connect: with or without jump host
        let ssh = if let Some(jump_id) = jump_host {
            match self.connect_via_jump(id, &jump_id).await {
                Ok(conn) => conn,
                Err(e) => {
                    let error_msg = e.to_string();
                    let tunnel = self.tunnels.get_mut(id).unwrap();
                    tunnel.status = TunnelStatus::Disconnected;
                    tunnel.error_message = Some(error_msg);
                    tunnel.jump_connection = None;
                    tunnel.ki_slot = None;
                    self.emit_status(id, &TunnelStatus::Disconnected);
                    return Err(e);
                }
            }
        } else {
            // Direct connection
            let credentials = match self.build_credentials(id) {
                Ok(c) => c,
                Err(e) => {
                    let error_msg = e.to_string();
                    let tunnel = self.tunnels.get_mut(id).unwrap();
                    tunnel.status = TunnelStatus::Disconnected;
                    tunnel.error_message = Some(error_msg);
                    self.emit_status(id, &TunnelStatus::Disconnected);
                    return Err(e);
                }
            };

            let (host, port, user) = {
                let tunnel = self.tunnels.get(id).unwrap();
                (tunnel.config.host.clone(), tunnel.config.port, tunnel.config.user.clone())
            };

            match SshConnection::connect(&host, port, &user, credentials, timeout_secs).await {
                Ok(conn) => Arc::new(conn),
                Err(e) => {
                    let error_msg = e.to_string();
                    let tunnel = self.tunnels.get_mut(id).unwrap();
                    tunnel.status = TunnelStatus::Disconnected;
                    tunnel.error_message = Some(error_msg);
                    tunnel.ki_slot = None;
                    self.emit_status(id, &TunnelStatus::Disconnected);
                    return Err(e);
                }
            }
        };

        // Increment generation so stale TunnelDied messages are ignored
        let generation = {
            let tunnel = self.tunnels.get_mut(id).unwrap();
            tunnel.generation += 1;
            tunnel.generation
        };

        // Create death channel for health/forwarder to report tunnel death
        let (death_tx, mut death_rx) = mpsc::channel::<String>(1);
        let manager_tx = self.manager_tx.clone();
        let tunnel_id = id.to_string();

        // Spawn death listener that forwards to manager with generation tag
        let death_handle = tokio::spawn(async move {
            if let Some(error) = death_rx.recv().await {
                let _ = manager_tx
                    .send(ManagerCommand::TunnelDied {
                        id: tunnel_id,
                        error,
                        generation,
                    })
                    .await;
            }
        });

        // Spawn port forwarder
        let fwd_ssh = ssh.clone();
        let fwd_death_tx = death_tx.clone();
        let fwd_remote_host = remote_host.clone();
        let fwd_tunnel_id = id.to_string();
        let forwarder_handle = tokio::spawn(async move {
            if let Err(e) = PortForwarder::start(
                fwd_ssh,
                local_port,
                fwd_remote_host,
                remote_port,
                fwd_death_tx.clone(),
                fwd_tunnel_id,
            )
            .await
            {
                warn!("Port forwarder exited with error: {}", e);
                let _ = fwd_death_tx.send(format!("Port forwarder failed: {}", e)).await;
            }
        });

        // Spawn health monitor
        let health_ssh = ssh.clone();
        let health_tunnel_id = id.to_string();
        let health_death_tx = death_tx;
        let health_handle = tokio::spawn(async move {
            HealthMonitor::run(
                health_ssh,
                health_tunnel_id,
                keepalive_interval,
                keepalive_timeout,
                health_death_tx,
            )
            .await;
        });

        // Store state — abort all previous tasks and track new ones
        {
            let tunnel = self.tunnels.get_mut(id).unwrap();
            // Abort any leftover tasks from a previous connection
            for handle in tunnel.abort_handles.drain(..) {
                handle.abort();
            }
            tunnel.abort_handles.push(forwarder_handle.abort_handle());
            tunnel.abort_handles.push(health_handle.abort_handle());
            tunnel.abort_handles.push(death_handle.abort_handle());
            tunnel.ssh_connection = Some(ssh);
            tunnel.status = TunnelStatus::Connected;
        }
        self.emit_status(id, &TunnelStatus::Connected);

        info!("Tunnel {} connected", id);
        Ok(())
    }

    async fn handle_disconnect(&mut self, id: &str) -> Result<(), TunnelError> {
        {
            let tunnel = self
                .tunnels
                .get_mut(id)
                .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

            if tunnel.status == TunnelStatus::Disconnected {
                return Ok(());
            }

            tunnel.status = TunnelStatus::Disconnecting;
        }
        self.emit_status(id, &TunnelStatus::Disconnecting);

        // Abort all background tasks and clean up SSH
        if let Some(tunnel) = self.tunnels.get_mut(id) {
            for handle in tunnel.abort_handles.drain(..) {
                handle.abort();
            }
            if let Some(ssh) = tunnel.ssh_connection.take() {
                ssh.disconnect().await;
            }
            if let Some(jump_ssh) = tunnel.jump_connection.take() {
                jump_ssh.disconnect().await;
            }
            tunnel.ki_slot = None;
            tunnel.status = TunnelStatus::Disconnected;
            tunnel.error_message = None;
        }
        self.emit_status(id, &TunnelStatus::Disconnected);

        info!("Tunnel {} disconnected", id);
        Ok(())
    }

    async fn handle_tunnel_died(&mut self, id: &str, error: &str, generation: u64) {
        let tunnel = match self.tunnels.get(id) {
            Some(t) => t,
            None => return,
        };

        // Ignore stale death messages from a previous connection generation
        if tunnel.generation != generation {
            debug!(
                "Ignoring stale TunnelDied for {} (gen {} != current {})",
                id, generation, tunnel.generation
            );
            return;
        }

        warn!("Tunnel {} died: {}", id, error);
        {
            let tunnel = self.tunnels.get_mut(id).unwrap();
            tunnel.status = TunnelStatus::Error;
            tunnel.error_message = Some(error.to_string());
        }
        self.emit_status(id, &TunnelStatus::Error);
        self.emit_error(id, error, "connection_lost");

        // Clean up
        {
            let tunnel = self.tunnels.get_mut(id).unwrap();
            for handle in tunnel.abort_handles.drain(..) {
                handle.abort();
            }
            if let Some(ssh) = tunnel.ssh_connection.take() {
                ssh.disconnect().await;
            }
            if let Some(jump_ssh) = tunnel.jump_connection.take() {
                jump_ssh.disconnect().await;
            }
            tunnel.ki_slot = None;
            tunnel.status = TunnelStatus::Disconnected;
        }
        self.emit_status(id, &TunnelStatus::Disconnected);
    }

    fn handle_add_tunnel(&mut self, config: TunnelConfig) -> Result<TunnelInfo, TunnelError> {
        if self.tunnels.contains_key(&config.id) {
            return Err(TunnelError::ConfigInvalid(
                format!("Tunnel '{}' already exists", config.id),
            ));
        }
        let state = TunnelState::new(config);
        let info = self.tunnel_to_info(&state);
        self.tunnels.insert(info.id.clone(), state);
        info!("Added tunnel '{}'", info.id);
        Ok(info)
    }

    async fn handle_update_tunnel(&mut self, config: TunnelConfig) -> Result<TunnelInfo, TunnelError> {
        let id = config.id.clone();

        // Disconnect if currently connected
        if let Some(tunnel) = self.tunnels.get(&id) {
            if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting {
                info!("Disconnecting tunnel '{}' before update", id);
                self.handle_disconnect(&id).await.ok();
            }
        } else {
            return Err(TunnelError::TunnelNotFound(id));
        }

        // Replace with new config, reset state
        let state = TunnelState::new(config);
        let info = self.tunnel_to_info(&state);
        self.tunnels.insert(id.clone(), state);
        self.emit_status(&id, &TunnelStatus::Disconnected);
        info!("Updated tunnel '{}'", id);
        Ok(info)
    }

    async fn handle_remove_tunnel(&mut self, id: &str) -> Result<(), TunnelError> {
        // Disconnect if currently connected
        if let Some(tunnel) = self.tunnels.get(id) {
            if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting {
                info!("Disconnecting tunnel '{}' before removal", id);
                self.handle_disconnect(id).await.ok();
            }
        }

        // Find tunnels that use this one as a jump host and disconnect them
        let dependent_ids: Vec<String> = self.tunnels.iter()
            .filter(|(tid, t)| {
                *tid != id && t.config.jump_host.as_deref() == Some(id)
            })
            .map(|(tid, _)| tid.clone())
            .collect();

        for dep_id in &dependent_ids {
            if let Some(t) = self.tunnels.get(dep_id) {
                if t.status == TunnelStatus::Connected || t.status == TunnelStatus::Connecting {
                    info!("Disconnecting dependent tunnel '{}' before removing jump host '{}'", dep_id, id);
                    self.handle_disconnect(dep_id).await.ok();
                }
            }
        }

        // Clear dangling jump_host references
        for dep_id in &dependent_ids {
            if let Some(t) = self.tunnels.get_mut(dep_id) {
                t.config.jump_host = None;
            }
        }

        // Delete password from keyring
        keychain::delete_password(id);

        match self.tunnels.remove(id) {
            Some(_) => {
                info!("Removed tunnel '{}'", id);
                Ok(())
            }
            None => Err(TunnelError::TunnelNotFound(id.to_string())),
        }
    }

    fn handle_reload(&mut self, config: AppConfig) -> Result<(), TunnelError> {
        // Update settings
        self.settings = config.settings;

        let new_ids: std::collections::HashSet<String> =
            config.tunnels.iter().map(|tc| tc.id.clone()).collect();

        // Remove tunnels that are no longer in config (disconnect first)
        let removed_ids: Vec<String> = self
            .tunnels
            .keys()
            .filter(|id| !new_ids.contains(*id))
            .cloned()
            .collect();
        for id in &removed_ids {
            if let Some(tunnel) = self.tunnels.get(id) {
                if tunnel.status == TunnelStatus::Connected
                    || tunnel.status == TunnelStatus::Connecting
                {
                    // Abort background tasks and clean up
                    if let Some(tunnel) = self.tunnels.get_mut(id) {
                        for handle in tunnel.abort_handles.drain(..) {
                            handle.abort();
                        }
                        // Can't await disconnect here since handle_reload is sync,
                        // but aborting tasks is sufficient for cleanup
                    }
                }
            }
            self.tunnels.remove(id);
            info!("Removed tunnel '{}' during reload", id);
        }

        // Add new tunnels, update existing ones (but don't touch connected tunnels' state)
        for tc in config.tunnels {
            if let Some(existing) = self.tunnels.get_mut(&tc.id) {
                existing.config = tc;
            } else {
                self.tunnels.insert(tc.id.clone(), TunnelState::new(tc));
            }
        }

        info!("Config reloaded, {} tunnels configured", self.tunnels.len());
        Ok(())
    }

    async fn disconnect_all(&mut self) {
        let ids: Vec<String> = self.tunnels.keys().cloned().collect();
        for id in ids {
            let _ = self.handle_disconnect(&id).await;
        }
    }

    fn emit_status(&self, id: &str, status: &TunnelStatus) {
        if let Some(tx) = &self.event_tx {
            let event = TunnelStatusEvent {
                id: id.to_string(),
                status: status.clone(),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            };
            let _ = tx.send(event);
        }
    }

    fn emit_error(&self, id: &str, message: &str, code: &str) {
        if let Some(tx) = &self.error_tx {
            let event = crate::types::TunnelErrorEvent {
                id: id.to_string(),
                message: message.to_string(),
                code: code.to_string(),
            };
            let _ = tx.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AuthMethod, Settings, TunnelType};

    fn test_config() -> AppConfig {
        AppConfig {
            version: 1,
            tunnels: vec![
                TunnelConfig {
                    id: "db".into(),
                    name: "Database".into(),
                    host: "example.com".into(),
                    port: 22,
                    user: "user".into(),
                    auth_method: AuthMethod::Key,
                    key_path: "~/.ssh/id_rsa".into(),
                    tunnel_type: TunnelType::Local,
                    local_port: 5432,
                    remote_host: "db.internal".into(),
                    remote_port: 5432,
                    auto_connect: false,
                    jump_host: None,
                },
                TunnelConfig {
                    id: "redis".into(),
                    name: "Redis".into(),
                    host: "example.com".into(),
                    port: 22,
                    user: "user".into(),
                    auth_method: AuthMethod::Key,
                    key_path: "~/.ssh/id_rsa".into(),
                    tunnel_type: TunnelType::Local,
                    local_port: 6379,
                    remote_host: "redis.internal".into(),
                    remote_port: 6379,
                    auto_connect: false,
                    jump_host: None,
                },
            ],
            settings: Settings::default(),
        }
    }

    #[tokio::test]
    async fn list_tunnels_returns_all() {
        let handle = spawn_manager(test_config(), None, None, None);
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ListTunnels { reply: reply_tx })
            .await
            .unwrap();

        let tunnels = reply_rx.await.unwrap();
        assert_eq!(tunnels.len(), 2);
        assert!(tunnels.iter().all(|t| t.status == TunnelStatus::Disconnected));
    }

    #[tokio::test]
    async fn connect_unknown_tunnel_returns_error() {
        let handle = spawn_manager(test_config(), None, None, None);

        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "nonexistent".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        let result = reply_rx.await.unwrap();
        assert!(matches!(result, Err(TunnelError::TunnelNotFound(_))));
    }

    #[tokio::test]
    async fn reload_config_adds_new_tunnels() {
        let handle = spawn_manager(test_config(), None, None, None);

        let mut new_config = test_config();
        new_config.tunnels.push(TunnelConfig {
            id: "new".into(),
            name: "New Tunnel".into(),
            host: "example.com".into(),
            port: 22,
            user: "user".into(),
            auth_method: AuthMethod::Key,
            key_path: "~/.ssh/id_rsa".into(),
            tunnel_type: TunnelType::Local,
            local_port: 8080,
            remote_host: "web.internal".into(),
            remote_port: 80,
            auto_connect: false,
            jump_host: None,
        });

        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ReloadConfig {
                config: new_config,
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Verify 3 tunnels now
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ListTunnels { reply: reply_tx })
            .await
            .unwrap();
        let tunnels = reply_rx.await.unwrap();
        assert_eq!(tunnels.len(), 3);
    }

    #[tokio::test]
    async fn connect_to_unreachable_host_returns_error() {
        let mut config = test_config();
        // Use an unreachable host with a short timeout
        config.tunnels[0].host = "192.0.2.1".into(); // RFC 5737 TEST-NET, should be unreachable
        config.settings.connection_timeout_secs = 2;

        let handle = spawn_manager(config, None, None, None);

        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        let result = reply_rx.await.unwrap();
        assert!(result.is_err(), "Expected error connecting to unreachable host");

        // Verify tunnel is back to Disconnected with an error message
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ListTunnels { reply: reply_tx })
            .await
            .unwrap();
        let tunnels = reply_rx.await.unwrap();
        let db = tunnels.iter().find(|t| t.id == "db").unwrap();
        assert_eq!(db.status, TunnelStatus::Disconnected);
        assert!(db.error_message.is_some(), "Expected error_message to be set");
    }
}
