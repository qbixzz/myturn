pub mod bridge;
pub mod color_rules;
pub mod config;
pub mod icon;
pub mod watcher;

use std::path::PathBuf;
use std::sync::Mutex;
use bridge::BridgeData;
use config::AppConfig;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_dialog::DialogExt;

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
        .plugin(tauri_plugin_single_instance::init(|_app, _argv, _cwd| {
            // Second instance launched — silently ignore; first instance stays.
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
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

            let config_path = resolve_config_path(app.handle());
            let app_config  = load_or_create_config(&config_path);

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

            // ── Tray context menu ─────────────────────────────────────────
            // Settings is greyed out (Phase 2 placeholder).
            let settings = MenuItem::with_id(app, "settings", "Settings…",    false, None::<&str>)?;
            let about    = MenuItem::with_id(app, "about",    "About MyTurn", true,  None::<&str>)?;
            let sep      = PredefinedMenuItem::separator(app)?;
            let quit     = MenuItem::with_id(app, "quit",     "Exit",         true,  None::<&str>)?;
            let menu     = Menu::with_items(app, &[&settings, &about, &sep, &quit])?;

            // ── System tray ───────────────────────────────────────────────
            // Gray icon = "waiting for first turn" — replaced on first bridge read.
            TrayIconBuilder::with_id("main")
                .icon(icon::render(0.0, "#444444"))
                .tooltip("MyTurn — waiting for first turn…")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "about" => {
                        let v = app.package_info().version.to_string();
                        app.dialog()
                            .message(format!(
                                "MyTurn v{v}\n\nContext window usage tracker for Claude Code."
                            ))
                            .title("About MyTurn")
                            .show(|_| {});
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            watcher::start(app.handle().clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
