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
    tray::{MouseButton, MouseButtonState, TrayIconBuilder},
    Emitter,
    Manager,
};

// ── macOS: NSPanel imports and setup ────────────────────────────────

#[cfg(target_os = "macos")]
use tauri_nspanel::ManagerExt as PanelManagerExt;
#[cfg(target_os = "macos")]
use tauri_nspanel::WebviewWindowExt as PanelWindowExt;

#[cfg(target_os = "macos")]
tauri_nspanel::tauri_panel! {
    panel!(TunnelPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })

    panel_event!(TunnelPanelHandler {
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

#[cfg(target_os = "macos")]
fn setup_macos_panel(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    if let Some(window) = app.get_webview_window("main") {
        let panel = window.to_panel::<TunnelPanel>()?;

        panel.set_style_mask(
            tauri_nspanel::StyleMask::empty().nonactivating_panel().into(),
        );

        panel.set_collection_behavior(
            tauri_nspanel::CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary()
                .into(),
        );

        panel.set_level(tauri_nspanel::PanelLevel::PopUpMenu.value());
        panel.set_hides_on_deactivate(false);

        let handler = TunnelPanelHandler::new();
        let app_handle_for_handler = app.handle().clone();
        handler.window_did_resign_key(move |_notification| {
            tracing::debug!("Panel resigned key window — hiding");
            if let Ok(p) = app_handle_for_handler.get_webview_panel("main") {
                p.hide();
            }
        });
        panel.set_event_handler(Some(handler.as_ref()));

        tracing::info!("NSPanel configured: is_floating={}, can_become_key={}",
            panel.is_floating_panel(), panel.can_become_key_window());
    } else {
        tracing::error!("Could not find 'main' webview window!");
    }

    Ok(())
}

// ── Tray icon click handler (macOS — uses NSPanel) ──────────────────

#[cfg(target_os = "macos")]
fn handle_tray_click(tray: &tauri::tray::TrayIcon, rect: tauri::Rect) {
    let app = tray.app_handle();
    if let Ok(panel) = app.get_webview_panel("main") {
        if panel.is_visible() {
            panel.hide();
        } else {
            if let Some(window) = app.get_webview_window("main") {
                let scale = window.scale_factor().unwrap_or(1.0);

                let icon_x = match rect.position {
                    tauri::Position::Physical(p) => p.x as f64,
                    tauri::Position::Logical(l) => l.x * scale,
                };
                let icon_y = match rect.position {
                    tauri::Position::Physical(p) => p.y as f64,
                    tauri::Position::Logical(l) => l.y * scale,
                };
                let icon_h = match rect.size {
                    tauri::Size::Physical(s) => s.height as f64,
                    tauri::Size::Logical(l) => l.height * scale,
                };

                let x = icon_x;
                let y = icon_y + icon_h;
                let _ = window.set_position(tauri::PhysicalPosition::new(x as i32, y as i32));
            }
            panel.order_front_regardless();
            panel.show_and_make_key();
        }
    }
}

// ── Tray icon click handler (Linux/Windows — uses regular window) ───

#[cfg(not(target_os = "macos"))]
fn handle_tray_click(tray: &tauri::tray::TrayIcon, _rect: tauri::Rect) {
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

// ── Main run function ───────────────────────────────────────────────

pub fn run() {
    #[cfg(target_os = "macos")]
    {
        use tracing_subscriber::prelude::*;
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new("tunnel_master=debug"))
            .with(tracing_oslog::OsLogger::new(
                "com.tunnelmaster.app",
                "default",
            ))
            .init();
    }
    #[cfg(not(target_os = "macos"))]
    {
        tracing_subscriber::fmt()
            .with_env_filter("tunnel_master=debug")
            .init();
    }

    let config_store = ConfigStore::new(ConfigStore::default_path());
    let config = config_store.load().unwrap_or_else(|e| {
        tracing::warn!("Could not load config: {}. Starting with empty config.", e);
        types::AppConfig {
            version: 1,
            tunnels: vec![],
            settings: types::Settings::default(),
        }
    });

    let mut builder = tauri::Builder::default();

    #[cfg(target_os = "macos")]
    {
        builder = builder.plugin(tauri_nspanel::init());
    }

    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            setup_macos_panel(app)?;

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
                    if let tauri::tray::TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        rect,
                        ..
                    } = event
                    {
                        handle_tray_click(tray, rect);
                    }
                })
                .build(app)?;

            let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<TunnelStatusEvent>();
            let (error_tx, mut error_rx) = tokio::sync::mpsc::unbounded_channel::<types::TunnelErrorEvent>();
            let manager = spawn_manager(config, Some(event_tx), Some(error_tx));

            let app_handle = app.handle().clone();
            let manager_for_events = manager.clone();
            tauri::async_runtime::spawn(async move {
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
            tauri::async_runtime::spawn(async move {
                while let Some(event) = error_rx.recv().await {
                    let _ = app_handle2.emit("tunnel-error", &event);
                }
            });

            app.manage(AppState {
                manager,
                config_store: std::sync::Mutex::new(config_store),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_tunnels,
            commands::connect_tunnel,
            commands::disconnect_tunnel,
            commands::store_passphrase_for_tunnel,
            commands::reload_config,
            commands::add_tunnel,
            commands::update_tunnel,
            commands::delete_tunnel,
            commands::get_tunnel_config,
            commands::accept_host_key,
            commands::pick_key_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
