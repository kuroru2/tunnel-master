mod config;
mod commands;
mod errors;
mod keychain;
mod tunnel;
pub mod types;

use commands::AppState;
use config::store::ConfigStore;
use tunnel::manager::{spawn_manager, ManagerCommand};
use types::{TunnelStatus, TunnelStatusEvent};

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter,
    Manager,
};

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter("tunnel_master=debug")
        .init();

    let config_store = ConfigStore::new(ConfigStore::default_path());
    let config = config_store.load().unwrap_or_else(|e| {
        tracing::warn!("Could not load config: {}. Starting with empty config.", e);
        types::AppConfig {
            version: 1,
            tunnels: vec![],
            settings: types::Settings::default(),
        }
    });

    tauri::Builder::default()
        .setup(move |app| {
            let quit = MenuItem::with_id(app, "quit", "Quit Tunnel Master", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let _tray = TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Tunnel Master — no active tunnels")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app: &tauri::AppHandle, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray: &tauri::tray::TrayIcon, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<TunnelStatusEvent>();
            let (error_tx, mut error_rx) = tokio::sync::mpsc::unbounded_channel::<types::TunnelErrorEvent>();
            let manager = spawn_manager(config, Some(event_tx), Some(error_tx));

            let app_handle = app.handle().clone();
            let manager_for_events = manager.clone();
            tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    let _ = app_handle.emit("tunnel-status-changed", &event);

                    if let Some(tray) = app_handle.tray_by_id("main") {
                        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                        if manager_for_events
                            .send(ManagerCommand::ListTunnels { reply: reply_tx })
                            .await
                            .is_ok()
                        {
                            if let Ok(tunnels) = reply_rx.await {
                                let connected = tunnels
                                    .iter()
                                    .filter(|t| t.status == TunnelStatus::Connected)
                                    .count();
                                let total = tunnels.len();
                                let tooltip = if connected == 0 {
                                    "Tunnel Master — no active tunnels".to_string()
                                } else if connected == total {
                                    format!("Tunnel Master — all {} tunnels active", total)
                                } else {
                                    format!("Tunnel Master — {}/{} tunnels active", connected, total)
                                };
                                let _ = tray.set_tooltip(Some(&tooltip));
                            }
                        }
                    }
                }
            });

            let app_handle2 = app.handle().clone();
            tokio::spawn(async move {
                while let Some(event) = error_rx.recv().await {
                    let _ = app_handle2.emit("tunnel-error", &event);
                }
            });

            app.manage(AppState {
                manager,
                config_store,
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_tunnels,
            commands::connect_tunnel,
            commands::disconnect_tunnel,
            commands::reload_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
