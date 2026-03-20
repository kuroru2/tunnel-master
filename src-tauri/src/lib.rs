mod commands;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use commands::AppState;
use tunnel_core::config::store::ConfigStore;
use tunnel_core::events::KiPromptEntry;
use tunnel_core::tunnel::manager::{spawn_manager, ManagerCommand};
use tunnel_core::types::{AppConfig, Settings, TrafficSample, TunnelStatus};
use tunnel_core::TunnelEventHandler;

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder},
    Emitter,
    Manager,
};

// ── Tauri-specific event types ───────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct TunnelStatusEvent {
    id: String,
    status: TunnelStatus,
    timestamp: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
struct TunnelErrorEvent {
    id: String,
    message: String,
}

// ── TauriEventHandler — bridges tunnel_core events to Tauri ─────────

struct TauriEventHandler {
    app_handle: tauri::AppHandle,
}

impl TauriEventHandler {
    fn new(app_handle: tauri::AppHandle) -> Self {
        Self { app_handle }
    }
}

impl TunnelEventHandler for TauriEventHandler {
    fn on_tunnel_state_changed(&self, id: String, status: TunnelStatus, _error_message: Option<String>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let event = TunnelStatusEvent { id, status, timestamp };
        let _ = self.app_handle.emit("tunnel-status-changed", &event);
    }

    fn on_passphrase_requested(&self, id: String, key_path: String) {
        let _ = self.app_handle.emit("passphrase-requested", serde_json::json!({
            "id": id,
            "keyPath": key_path,
        }));
    }

    fn on_password_requested(&self, id: String) {
        let _ = self.app_handle.emit("password-requested", serde_json::json!({
            "id": id,
        }));
    }

    fn on_host_key_verification(&self, id: String, host: String, port: u16, key_type: String, fingerprint: String) {
        let _ = self.app_handle.emit("host-key-verification", serde_json::json!({
            "id": id,
            "host": host,
            "port": port,
            "keyType": key_type,
            "fingerprint": fingerprint,
        }));
    }

    fn on_keyboard_interactive(
        &self,
        id: String,
        name: String,
        instructions: String,
        prompts: Vec<KiPromptEntry>,
    ) {
        let prompts_json: Vec<serde_json::Value> = prompts.iter().map(|p| serde_json::json!({
            "text": p.text,
            "echo": p.echo,
        })).collect();
        let _ = self.app_handle.emit("keyboard-interactive", serde_json::json!({
            "id": id,
            "name": name,
            "instructions": instructions,
            "prompts": prompts_json,
        }));
    }

    fn on_traffic_update(&self, id: String, sample: TrafficSample) {
        let _ = self.app_handle.emit("traffic-update", serde_json::json!({
            "id": id,
            "bytesIn": sample.bytes_in,
            "bytesOut": sample.bytes_out,
            "timestamp": sample.timestamp,
        }));
    }

    fn on_error(&self, id: String, message: String) {
        let event = TunnelErrorEvent { id, message };
        let _ = self.app_handle.emit("tunnel-error", &event);
    }
}

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
            // Un-highlight the tray icon and release menu bar when panel loses focus
            if let Some(tray) = app_handle_for_handler.tray_by_id("main") {
                set_tray_highlighted(&tray, false);
            }
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
/// Set the tray icon's highlighted state via NSStatusBarButton.
fn set_tray_highlighted(tray: &tauri::tray::TrayIcon, highlighted: bool) {
    let _ = tray.with_inner_tray_icon(move |inner| {
        if let Some(status_item) = inner.ns_status_item() {
            unsafe {
                let mtm = tauri_nspanel::objc2::MainThreadMarker::new_unchecked();
                if let Some(button) = status_item.button(mtm) {
                    let _: () = tauri_nspanel::objc2::msg_send![&*button, setHighlighted: highlighted];
                }
            }
        }
    });
}


fn handle_tray_click(tray: &tauri::tray::TrayIcon, rect: tauri::Rect) {
    let app = tray.app_handle();
    if let Ok(panel) = app.get_webview_panel("main") {
        if panel.is_visible() {
            set_tray_highlighted(tray, false);
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
            set_tray_highlighted(tray, true);
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

// ── Shutdown signal handling ─────────────────────────────────────────

#[cfg(unix)]
async fn wait_for_shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM");
    tokio::select! {
        _ = sigterm.recv() => {}
        _ = tokio::signal::ctrl_c() => {}
    }
}

#[cfg(not(unix))]
async fn wait_for_shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
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
        AppConfig {
            version: 1,
            tunnels: vec![],
            settings: Settings::default(),
        }
    });

    let builder = tauri::Builder::default();

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            setup_macos_panel(app)?;

            let quit = MenuItem::with_id(app, "quit", "Quit Tunnel Master", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray-icon.png"))
                .expect("failed to load tray icon");

            let _tray = TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .icon_as_template(true)
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

            let event_handler = Arc::new(TauriEventHandler::new(app.handle().clone()));
            let manager = spawn_manager(config, event_handler);

            // Periodically update tray tooltip based on tunnel states
            let tray_app_handle = app.handle().clone();
            let tray_manager = manager.clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
                loop {
                    interval.tick().await;
                    if let Some(tray) = tray_app_handle.tray_by_id("main") {
                        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                        if tray_manager
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

            // Graceful shutdown on signals: send Shutdown command to manager,
            // wait for all tunnels to disconnect, then exit.
            let shutdown_manager = manager.clone();
            let shutdown_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                wait_for_shutdown_signal().await;
                tracing::info!("Shutdown signal received, disconnecting tunnels");

                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                if shutdown_manager
                    .send(ManagerCommand::Shutdown { reply: reply_tx })
                    .await
                    .is_ok()
                {
                    // Wait for manager to finish disconnecting (with timeout)
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        reply_rx,
                    ).await;
                }
                shutdown_app.exit(0);
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
            commands::reorder_tunnels,
            commands::quit_app,
            commands::accept_host_key,
            commands::pick_key_file,
            commands::store_password_for_tunnel,
            commands::clear_password_for_tunnel,
            commands::respond_keyboard_interactive,
            commands::cancel_keyboard_interactive,
            commands::get_traffic_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
