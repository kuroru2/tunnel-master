use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info, warn};

use crate::errors::TunnelError;
use crate::types::{AppConfig, TunnelConfig, TunnelInfo, TunnelStatus, TunnelStatusEvent};

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
    TunnelDied {
        id: String,
        error: String,
    },
}

/// Runtime state for a single tunnel
#[derive(Debug)]
struct TunnelState {
    config: TunnelConfig,
    status: TunnelStatus,
    error_message: Option<String>,
    /// Handle to abort the tunnel's tokio tasks on disconnect
    abort_handle: Option<tokio::task::AbortHandle>,
}

impl TunnelState {
    fn new(config: TunnelConfig) -> Self {
        Self {
            config,
            status: TunnelStatus::Disconnected,
            error_message: None,
            abort_handle: None,
        }
    }

    fn to_info(&self) -> TunnelInfo {
        TunnelInfo {
            id: self.config.id.clone(),
            name: self.config.name.clone(),
            status: self.status.clone(),
            local_port: self.config.local_port,
            remote_host: self.config.remote_host.clone(),
            remote_port: self.config.remote_port,
            error_message: self.error_message.clone(),
        }
    }
}

pub type ManagerHandle = mpsc::Sender<ManagerCommand>;

/// Spawn the TunnelManager actor. Returns a sender to communicate with it.
pub fn spawn_manager(
    config: AppConfig,
    event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
    error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
) -> ManagerHandle {
    let (tx, rx) = mpsc::channel(32);
    let manager_tx = tx.clone();

    tokio::spawn(async move {
        let mut manager = TunnelManagerActor::new(config, event_tx, error_tx, manager_tx);
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
}

impl TunnelManagerActor {
    fn new(
        config: AppConfig,
        event_tx: Option<mpsc::UnboundedSender<TunnelStatusEvent>>,
        error_tx: Option<mpsc::UnboundedSender<crate::types::TunnelErrorEvent>>,
        manager_tx: mpsc::Sender<ManagerCommand>,
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
        }
    }

    async fn run(&mut self, mut rx: mpsc::Receiver<ManagerCommand>) {
        info!("TunnelManager started with {} tunnels", self.tunnels.len());

        while let Some(cmd) = rx.recv().await {
            match cmd {
                ManagerCommand::ListTunnels { reply } => {
                    let infos: Vec<TunnelInfo> =
                        self.tunnels.values().map(|t| t.to_info()).collect();
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

                ManagerCommand::TunnelDied { id, error } => {
                    self.handle_tunnel_died(&id, &error).await;
                }
            }
        }

        info!("TunnelManager shutting down, disconnecting all tunnels");
        self.disconnect_all().await;
    }

    async fn handle_connect(&mut self, id: &str) -> Result<(), TunnelError> {
        {
            let tunnel = self
                .tunnels
                .get_mut(id)
                .ok_or_else(|| TunnelError::TunnelNotFound(id.to_string()))?;

            if tunnel.status == TunnelStatus::Connected || tunnel.status == TunnelStatus::Connecting
            {
                debug!("Tunnel {} already connected/connecting", id);
                return Ok(());
            }

            tunnel.status = TunnelStatus::Connecting;
            tunnel.error_message = None;
        }
        self.emit_status(id, &TunnelStatus::Connecting);

        // TODO: Task 6 will add real SSH connection logic here.
        // For now, we just transition to Connected to validate the state machine.
        self.tunnels.get_mut(id).unwrap().status = TunnelStatus::Connected;
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

        // Abort the tunnel's background tasks if any
        if let Some(tunnel) = self.tunnels.get_mut(id) {
            if let Some(handle) = tunnel.abort_handle.take() {
                handle.abort();
            }
            tunnel.status = TunnelStatus::Disconnected;
            tunnel.error_message = None;
        }
        self.emit_status(id, &TunnelStatus::Disconnected);

        info!("Tunnel {} disconnected", id);
        Ok(())
    }

    async fn handle_tunnel_died(&mut self, id: &str, error: &str) {
        if !self.tunnels.contains_key(id) {
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
            if let Some(handle) = tunnel.abort_handle.take() {
                handle.abort();
            }
            tunnel.status = TunnelStatus::Disconnected;
        }
        self.emit_status(id, &TunnelStatus::Disconnected);
    }

    fn handle_reload(&mut self, config: AppConfig) -> Result<(), TunnelError> {
        // Update settings
        self.settings = config.settings;

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
    use crate::types::{Settings, TunnelType};

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
                    key_path: "~/.ssh/id_rsa".into(),
                    tunnel_type: TunnelType::Local,
                    local_port: 5432,
                    remote_host: "db.internal".into(),
                    remote_port: 5432,
                    auto_connect: false,
                },
                TunnelConfig {
                    id: "redis".into(),
                    name: "Redis".into(),
                    host: "example.com".into(),
                    port: 22,
                    user: "user".into(),
                    key_path: "~/.ssh/id_rsa".into(),
                    tunnel_type: TunnelType::Local,
                    local_port: 6379,
                    remote_host: "redis.internal".into(),
                    remote_port: 6379,
                    auto_connect: false,
                },
            ],
            settings: Settings::default(),
        }
    }

    #[tokio::test]
    async fn list_tunnels_returns_all() {
        let handle = spawn_manager(test_config(), None, None);
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
    async fn connect_transitions_to_connected() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let handle = spawn_manager(test_config(), Some(event_tx), None);

        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        assert!(reply_rx.await.unwrap().is_ok());

        // Should have received Connecting then Connected events
        let evt1 = event_rx.recv().await.unwrap();
        assert_eq!(evt1.status, TunnelStatus::Connecting);
        let evt2 = event_rx.recv().await.unwrap();
        assert_eq!(evt2.status, TunnelStatus::Connected);
    }

    #[tokio::test]
    async fn disconnect_transitions_to_disconnected() {
        let handle = spawn_manager(test_config(), None, None);

        // Connect first
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Disconnect
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Disconnect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Verify status
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::ListTunnels { reply: reply_tx })
            .await
            .unwrap();
        let tunnels = reply_rx.await.unwrap();
        let db = tunnels.iter().find(|t| t.id == "db").unwrap();
        assert_eq!(db.status, TunnelStatus::Disconnected);
    }

    #[tokio::test]
    async fn connect_unknown_tunnel_returns_error() {
        let handle = spawn_manager(test_config(), None, None);

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
    async fn tunnel_died_transitions_through_error() {
        let (event_tx, mut event_rx) = mpsc::unbounded_channel();
        let handle = spawn_manager(test_config(), Some(event_tx), None);

        // Connect first
        let (reply_tx, reply_rx) = oneshot::channel();
        handle
            .send(ManagerCommand::Connect {
                id: "db".into(),
                reply: reply_tx,
            })
            .await
            .unwrap();
        reply_rx.await.unwrap().unwrap();

        // Drain connect events
        let _ = event_rx.recv().await; // Connecting
        let _ = event_rx.recv().await; // Connected

        // Simulate tunnel death
        handle
            .send(ManagerCommand::TunnelDied {
                id: "db".into(),
                error: "Connection lost".into(),
            })
            .await
            .unwrap();

        // Should get Error then Disconnected
        let evt = event_rx.recv().await.unwrap();
        assert_eq!(evt.status, TunnelStatus::Error);
        let evt = event_rx.recv().await.unwrap();
        assert_eq!(evt.status, TunnelStatus::Disconnected);
    }

    #[tokio::test]
    async fn reload_config_adds_new_tunnels() {
        let handle = spawn_manager(test_config(), None, None);

        let mut new_config = test_config();
        new_config.tunnels.push(TunnelConfig {
            id: "new".into(),
            name: "New Tunnel".into(),
            host: "example.com".into(),
            port: 22,
            user: "user".into(),
            key_path: "~/.ssh/id_rsa".into(),
            tunnel_type: TunnelType::Local,
            local_port: 8080,
            remote_host: "web.internal".into(),
            remote_port: 80,
            auto_connect: false,
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
}
