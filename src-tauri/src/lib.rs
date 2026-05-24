pub mod bridge;
pub mod color_rules;
pub mod config;
pub mod icon;
pub mod watcher;

use std::path::PathBuf;
use std::sync::Mutex;
use bridge::BridgeData;
use config::AppConfig;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_positioner::{Position, WindowExt};

// ── Shared app state ─────────────────────────────────────────────────────────

pub struct AppState {
    pub last_bridge: Mutex<Option<BridgeData>>,
    pub config:      Mutex<AppConfig>,
    pub config_path: PathBuf,
}

// ── Config helpers ───────────────────────────────────────────────────────────

fn resolve_config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".myturn"))
        .join("config.json")
}

/// Load config from `path`. If the file doesn't exist, write defaults and
/// return them. Errors fall back to defaults silently.
fn load_or_create_config(path: &PathBuf) -> AppConfig {
    if path.exists() {
        return config::load(path);
    }
    let defaults = AppConfig::default();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&defaults) {
        let _ = std::fs::write(path, json);
    }
    defaults
}

fn save_config(path: &PathBuf, cfg: &AppConfig) {
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(path, json);
    }
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
fn current_state(state: tauri::State<AppState>) -> Option<BridgeData> {
    state.last_bridge.lock().unwrap().clone()
}

#[tauri::command]
fn get_config(state: tauri::State<AppState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
fn set_autostart(
    enabled: bool,
    state: tauri::State<AppState>,
    app: AppHandle,
) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    let mut cfg = state.config.lock().unwrap();
    cfg.auto_start = enabled;
    save_config(&state.config_path, &cfg);
    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            current_state,
            get_config,
            set_autostart
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Load (or create) config, then store as managed state.
            let config_path = resolve_config_path(app.handle());
            let app_config  = load_or_create_config(&config_path);

            // Apply autostart preference from config.
            if app_config.auto_start {
                let _ = app.autolaunch().enable();
            } else {
                let _ = app.autolaunch().disable();
            }

            app.manage(AppState {
                last_bridge: Mutex::new(None),
                config:      Mutex::new(app_config),
                config_path,
            });

            // Create flyout window — hidden until tray click.
            WebviewWindowBuilder::new(app, "flyout", WebviewUrl::App("index.html".into()))
                .title("MyTurn")
                .decorations(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .inner_size(320.0, 190.0)
                .resizable(false)
                .visible(false)
                .build()?;

            // Create system tray — left click toggles the flyout.
            TrayIconBuilder::with_id("main")
                .icon(icon::render(0.0, "#2ECC71"))
                .tooltip("MyTurn — waiting for first turn...")
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);

                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("flyout") {
                            if win.is_visible().unwrap_or(false) {
                                let _ = win.hide();
                            } else {
                                let _ = win.move_window(Position::TrayBottomRight);
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;

            watcher::start(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
