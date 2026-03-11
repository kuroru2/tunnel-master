use tauri::State;
use tokio::sync::oneshot;

use crate::config::store::{generate_id, validate_tunnel_input, ConfigStore};
use crate::keychain;
use crate::tunnel::manager::{ManagerCommand, ManagerHandle};
use crate::types::{TunnelConfig, TunnelInfo, TunnelInput};

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
pub async fn store_passphrase_for_tunnel(
    id: String,
    passphrase: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state
        .manager
        .send(ManagerCommand::GetKeyPath {
            id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| format!("Manager unavailable: {}", e))?;

    let key_path = reply_rx
        .await
        .map_err(|e| format!("Manager response error: {}", e))?
        .map_err(|e| e.to_string())?;

    let expanded = ConfigStore::expand_tilde(&key_path);
    keychain::set_passphrase(expanded.to_string_lossy().as_ref(), &passphrase)
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

#[tauri::command]
pub async fn add_tunnel(
    input: TunnelInput,
    state: State<'_, AppState>,
) -> Result<TunnelInfo, String> {
    // Collect existing IDs and ports for validation
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::ListTunnels { reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    let existing = reply_rx.await.map_err(|e| format!("Manager error: {}", e))?;

    let existing_ids: Vec<String> = existing.iter().map(|t| t.id.clone()).collect();
    let existing_ports: Vec<(String, u16)> = existing.iter().map(|t| (t.id.clone(), t.local_port)).collect();

    // Validate
    validate_tunnel_input(&input, &existing_ports, None).map_err(|e| e.to_string())?;

    // Generate ID
    let id = generate_id(&input.name, &existing_ids);
    let config = input.to_config(id);

    // Save to disk
    {
        let mut app_config = state.config_store.load().map_err(|e| e.to_string())?;
        app_config.tunnels.push(config.clone());
        state.config_store.save(&app_config).map_err(|e| e.to_string())?;
    }

    // Add to manager
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::AddTunnel { config, reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_tunnel(
    id: String,
    input: TunnelInput,
    state: State<'_, AppState>,
) -> Result<TunnelInfo, String> {
    // Collect existing ports for validation (exclude self)
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::ListTunnels { reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    let existing = reply_rx.await.map_err(|e| format!("Manager error: {}", e))?;
    let existing_ports: Vec<(String, u16)> = existing.iter().map(|t| (t.id.clone(), t.local_port)).collect();

    // Validate (exclude self from port conflict check)
    validate_tunnel_input(&input, &existing_ports, Some(&id)).map_err(|e| e.to_string())?;

    // Preserve original ID
    let config = input.to_config(id.clone());

    // Save to disk
    {
        let mut app_config = state.config_store.load().map_err(|e| e.to_string())?;
        if let Some(pos) = app_config.tunnels.iter().position(|t| t.id == id) {
            app_config.tunnels[pos] = config.clone();
        } else {
            return Err(format!("Tunnel '{}' not found in config", id));
        }
        state.config_store.save(&app_config).map_err(|e| e.to_string())?;
    }

    // Update in manager
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::UpdateTunnel { config, reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_tunnel(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Remove from manager (disconnects if connected)
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::RemoveTunnel { id: id.clone(), reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())?;

    // Remove from config on disk
    let mut app_config = state.config_store.load().map_err(|e| e.to_string())?;
    app_config.tunnels.retain(|t| t.id != id);
    state.config_store.save(&app_config).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_tunnel_config(
    id: String,
    state: State<'_, AppState>,
) -> Result<TunnelConfig, String> {
    let (reply_tx, reply_rx) = oneshot::channel();
    state.manager.send(ManagerCommand::GetTunnelConfig { id, reply: reply_tx })
        .await.map_err(|e| format!("Manager unavailable: {}", e))?;
    reply_rx.await.map_err(|e| format!("Manager error: {}", e))?.map_err(|e| e.to_string())
}
