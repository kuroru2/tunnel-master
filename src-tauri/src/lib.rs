mod config;
mod commands;
mod errors;
mod keychain;
mod tunnel;
pub mod types;

use commands::AppState;
use config::store::ConfigStore;
use tunnel::manager::spawn_manager;
use types::TunnelStatusEvent;

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

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<TunnelStatusEvent>();
            let (error_tx, mut error_rx) = tokio::sync::mpsc::unbounded_channel::<types::TunnelErrorEvent>();
            let manager = spawn_manager(config, Some(event_tx), Some(error_tx));

            let app_handle = app.handle().clone();
            tokio::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    let _ = app_handle.emit("tunnel-status-changed", &event);
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
