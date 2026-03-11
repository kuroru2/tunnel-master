use tauri::State;
use tokio::sync::oneshot;

use crate::config::store::ConfigStore;
use crate::tunnel::manager::{ManagerCommand, ManagerHandle};
use crate::types::TunnelInfo;

pub struct AppState {
    pub manager: ManagerHandle,
    pub config_store: ConfigStore,
}

#[tauri::command]
pub async fn list_tunnels(state: State<'_, AppState>) -> Result<Vec<TunnelInfo>, String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::ListTunnels { reply: reply_tx })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))
}

#[tauri::command]
pub async fn connect_tunnel(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::Connect {
            id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn disconnect_tunnel(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::Disconnect {
            id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reload_config(state: State<'_, AppState>) -> Result<(), String> {
    let config = state
        .config_store
        .load()
        .map_err(|e| e.to_string())?;

    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::ReloadConfig {
            config,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())
}
