use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use crate::config::store::ConfigStore;
use crate::events::TunnelEventHandler;
use crate::keychain;
use crate::tunnel::connection::accept_pending_host_key;
use crate::tunnel::manager::{ManagerCommand, ManagerHandle, spawn_manager};
use crate::types::{AppConfig, TrafficSample, TunnelConfig, TunnelInfo};

#[derive(uniffi::Object)]
pub struct TunnelCore {
    manager: ManagerHandle,
    config_store: Mutex<ConfigStore>,
    runtime: tokio::runtime::Handle,
}

#[uniffi::export]
impl TunnelCore {
    #[uniffi::constructor]
    pub fn new(event_handler: Arc<dyn TunnelEventHandler>) -> Self {
        let runtime = tokio::runtime::Handle::try_current()
            .unwrap_or_else(|_| {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
                let handle = rt.handle().clone();
                std::thread::spawn(move || rt.block_on(std::future::pending::<()>()));
                handle
            });

        let config_store = ConfigStore::new(ConfigStore::default_path());
        let config = config_store.load().unwrap_or_else(|_| AppConfig {
            version: 1,
            tunnels: vec![],
            settings: crate::types::Settings::default(),
        });

        let manager = runtime.block_on(async {
            spawn_manager(config, event_handler)
        });

        Self {
            manager,
            config_store: Mutex::new(config_store),
            runtime,
        }
    }

    pub fn list_tunnels(&self) -> Vec<TunnelInfo> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::ListTunnels { reply: tx }).await
        });
        self.runtime.block_on(async { rx.await.unwrap_or_default() })
    }

    pub fn connect(&self, id: String) {
        let manager = self.manager.clone();
        self.runtime.spawn(async move {
            let (tx, rx) = oneshot::channel();
            let _ = manager.send(ManagerCommand::Connect { id, reply: tx }).await;
            let _ = rx.await;
        });
    }

    pub fn disconnect(&self, id: String) {
        let manager = self.manager.clone();
        self.runtime.spawn(async move {
            let (tx, rx) = oneshot::channel();
            let _ = manager.send(ManagerCommand::Disconnect { id, reply: tx }).await;
            let _ = rx.await;
        });
    }

    pub fn get_tunnel_config(&self, id: String) -> Option<TunnelConfig> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::GetTunnelConfig { id, reply: tx }).await
        });
        self.runtime.block_on(async { rx.await.ok().and_then(|r| r.ok()) })
    }

    pub fn add_tunnel(&self, config: TunnelConfig) {
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                app_config.tunnels.push(config.clone());
                let _ = store.save(&app_config);
            }
        }
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::AddTunnel { config, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn update_tunnel(&self, id: String, config: TunnelConfig) {
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                if let Some(pos) = app_config.tunnels.iter().position(|t| t.id == id) {
                    app_config.tunnels[pos] = config.clone();
                    let _ = store.save(&app_config);
                }
            }
        }
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::UpdateTunnel { config, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn delete_tunnel(&self, id: String) {
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                app_config.tunnels.retain(|t| t.id != id);
                let _ = store.save(&app_config);
            }
        }
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::RemoveTunnel { id, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn reorder_tunnels(&self, ids: Vec<String>) {
        {
            let store = self.config_store.lock().unwrap();
            if let Ok(mut app_config) = store.load() {
                let mut reordered = Vec::new();
                for id in &ids {
                    if let Some(tc) = app_config.tunnels.iter().find(|t| &t.id == id) {
                        reordered.push(tc.clone());
                    }
                }
                app_config.tunnels = reordered;
                let _ = store.save(&app_config);
            }
        }
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::ReorderTunnels { ids, reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn reload_config(&self) {
        let store = self.config_store.lock().unwrap();
        if let Ok(config) = store.load() {
            let manager = self.manager.clone();
            let (tx, rx) = oneshot::channel();
            let _ = self.runtime.block_on(async {
                manager.send(ManagerCommand::ReloadConfig { config, reply: tx }).await
            });
            let _ = self.runtime.block_on(async { rx.await });
        }
    }

    pub fn get_traffic_history(&self, id: String) -> Vec<TrafficSample> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::GetTrafficHistory { id, reply: tx }).await
        });
        self.runtime.block_on(async {
            rx.await.ok().and_then(|r| r.ok()).unwrap_or_default()
        })
    }

    pub fn accept_host_key(&self, host: String, port: u16) {
        let _ = accept_pending_host_key(&host, port);
    }

    pub fn submit_passphrase(&self, id: String, passphrase: String) {
        let key_path = self.get_key_path(&id);
        if let Some(kp) = key_path {
            let _ = keychain::set_passphrase(&kp, &passphrase);
        }
        self.connect(id);
    }

    pub fn submit_password(&self, id: String, password: String) {
        let _ = keychain::store_password(&id, &password);
        self.connect(id);
    }

    pub fn respond_keyboard_interactive(&self, id: String, responses: Vec<String>) {
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::RespondKeyboardInteractive {
                id, responses, reply: tx,
            }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn cancel_auth(&self, id: String) {
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::CancelKeyboardInteractive {
                id, reply: tx,
            }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }

    pub fn store_passphrase(&self, id: String, passphrase: String) {
        let key_path = self.get_key_path(&id);
        if let Some(kp) = key_path {
            let _ = keychain::set_passphrase(&kp, &passphrase);
        }
    }

    pub fn store_password(&self, id: String, password: String) {
        let _ = keychain::store_password(&id, &password);
    }

    pub fn clear_credential(&self, id: String) {
        keychain::delete_password(&id);
    }

    pub fn shutdown(&self) {
        let manager = self.manager.clone();
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            manager.send(ManagerCommand::Shutdown { reply: tx }).await
        });
        let _ = self.runtime.block_on(async { rx.await });
    }
}

impl TunnelCore {
    fn get_key_path(&self, id: &str) -> Option<String> {
        let (tx, rx) = oneshot::channel();
        let _ = self.runtime.block_on(async {
            self.manager.send(ManagerCommand::GetKeyPath {
                id: id.to_string(),
                reply: tx,
            }).await
        });
        self.runtime.block_on(async {
            rx.await.ok().and_then(|r| r.ok())
        })
    }
}
