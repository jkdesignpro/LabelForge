//! App settings, saved as JSON in the user's config folder.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct Settings {
    /// Folder for global counters. Empty = the app's own data folder.
    /// Point several computers at one network folder to share counters.
    pub counter_dir: String,
    /// Printers set up for direct printing (see printing.rs).
    pub printers: Vec<serde_json::Value>,
    /// Odoo connections (passwords/API keys are kept in the OS keychain, not here).
    pub odoo: Vec<serde_json::Value>,
    /// Folder for the print history log. Empty = app data folder.
    pub history_dir: String,
}

fn path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d.join("settings.json"))
}

pub fn load(app: &tauri::AppHandle) -> Settings {
    path(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(app: &tauri::AppHandle, s: &Settings) -> Result<(), String> {
    let p = path(app)?;
    std::fs::write(p, serde_json::to_string_pretty(s).unwrap()).map_err(|e| e.to_string())
}

pub fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

pub fn counter_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let s = load(app);
    if s.counter_dir.trim().is_empty() {
        Ok(data_dir(app)?.join("counters"))
    } else {
        Ok(PathBuf::from(s.counter_dir))
    }
}
