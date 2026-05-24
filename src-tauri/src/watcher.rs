use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::sync::mpsc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

use crate::bridge::{read_bridge, BridgeData};
use crate::color_rules::color_for_percent;
use crate::icon;

#[derive(Clone, Serialize)]
pub struct TrayUpdatePayload {
    pub percent:     f64,
    pub used_tokens: u64,
    pub max_tokens:  u64,
    pub model:       Option<String>,
    pub session_id:  String,
}

impl From<&BridgeData> for TrayUpdatePayload {
    fn from(b: &BridgeData) -> Self {
        Self {
            percent:     b.context.percent,
            used_tokens: b.context.used_tokens,
            max_tokens:  b.context.max_tokens,
            model:       b.model.clone(),
            session_id:  b.session_id.clone(),
        }
    }
}

fn apply(app: &AppHandle, data: &BridgeData) {
    let app_state = app.state::<crate::AppState>();

    // Update last bridge snapshot.
    if let Ok(mut guard) = app_state.last_bridge.lock() {
        *guard = Some(data.clone());
    }

    // Read color rules from user config (not hardcoded defaults).
    let rules = app_state.config.lock().unwrap().color_rules.clone();
    let color   = color_for_percent(data.context.percent, &rules);
    let img     = icon::render(data.context.percent, color);
    let tooltip = format!(
        "MyTurn — {:.1}% ({} / {} tokens)",
        data.context.percent,
        data.context.used_tokens,
        data.context.max_tokens
    );
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_icon(Some(img));
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

/// Spawns a background thread that watches ~/.claude/myturn-bridge.json.
/// On every change, updates the tray icon color/fill and emits
/// "myturn://tray-update" to the frontend.
pub fn start(app: AppHandle) {
    let home        = dirs::home_dir().expect("home dir not found");
    let bridge_path = home.join(".claude").join("myturn-bridge.json");
    let watch_dir   = home.join(".claude");

    std::thread::spawn(move || {
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

        let mut watcher = RecommendedWatcher::new(
            tx,
            Config::default().with_poll_interval(Duration::from_millis(500)),
        )
        .expect("failed to create file watcher");

        // Watch the parent directory — atomic renames create a new inode and
        // would silently drop a direct file watch on the bridge file itself.
        watcher
            .watch(&watch_dir, RecursiveMode::NonRecursive)
            .expect("failed to watch ~/.claude/");

        // Apply immediately if bridge file already exists at startup.
        if let Some(data) = read_bridge(&bridge_path) {
            apply(&app, &data);
            let _ = app.emit("myturn://tray-update", TrayUpdatePayload::from(&data));
        }

        for result in rx {
            let event = match result {
                Ok(e)  => e,
                Err(_) => continue,
            };

            let is_bridge = event.paths.iter().any(|p| p == &bridge_path);
            if !is_bridge { continue; }

            let is_write = matches!(
                event.kind,
                EventKind::Modify(_) | EventKind::Create(_)
            );
            if !is_write { continue; }

            if let Some(data) = read_bridge(&bridge_path) {
                apply(&app, &data);
                let _ = app.emit("myturn://tray-update", TrayUpdatePayload::from(&data));
            }
        }
    });
}
