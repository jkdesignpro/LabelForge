// LabelForge desktop app. Free software under the GNU AGPL v3 or later.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use labelforge_core::api;
use labelforge_core::Label;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn render(req: serde_json::Value) -> Result<serde_json::Value, String> {
    let req: api::RenderRequest = serde_json::from_value(req).map_err(|e| e.to_string())?;
    serde_json::to_value(api::render(req)).map_err(|e| e.to_string())
}

#[tauri::command]
fn render_run(req: serde_json::Value) -> Result<serde_json::Value, String> {
    let req: api::RunRequest = serde_json::from_value(req).map_err(|e| e.to_string())?;
    serde_json::to_value(api::render_run(req)?).map_err(|e| e.to_string())
}

#[tauri::command]
fn symbologies() -> serde_json::Value {
    api::symbologies()
}

#[tauri::command]
fn example_label() -> String {
    include_str!("../../examples/part-tag.lfl").to_string()
}

/// Show an Open dialog and load a label. Returns `null` if cancelled.
#[tauri::command]
async fn open_label(app: tauri::AppHandle) -> Result<Option<(String, Label)>, String> {
    let Some(path) = app.dialog().file().add_filter("LabelForge label", &["lfl"]).blocking_pick_file() else {
        return Ok(None);
    };
    let path = path.into_path().map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(&path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    Ok(Some((path.display().to_string(), Label::from_json(&text)?)))
}

/// Save a label. With no path (or `save_as`), shows a Save dialog first.
/// Returns the path written, or `null` if cancelled.
#[tauri::command]
async fn save_label(app: tauri::AppHandle, label: Label, path: Option<String>) -> Result<Option<String>, String> {
    let path = match path {
        Some(p) if !p.is_empty() => std::path::PathBuf::from(p),
        _ => {
            let Some(p) = app
                .dialog()
                .file()
                .add_filter("LabelForge label", &["lfl"])
                .set_file_name(format!("{}.lfl", label.name))
                .blocking_save_file()
            else {
                return Ok(None);
            };
            p.into_path().map_err(|e| e.to_string())?
        }
    };
    std::fs::write(&path, label.to_json()).map_err(|e| format!("Cannot write {}: {e}", path.display()))?;
    Ok(Some(path.display().to_string()))
}

/// Save an exported file (SVG) chosen by the user.
#[tauri::command]
async fn export_file(app: tauri::AppHandle, name: String, ext: String, contents: String) -> Result<Option<String>, String> {
    let Some(p) = app.dialog().file().add_filter(&ext.to_uppercase(), &[ext.as_str()]).set_file_name(name).blocking_save_file() else {
        return Ok(None);
    };
    let p = p.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&p, contents).map_err(|e| e.to_string())?;
    Ok(Some(p.display().to_string()))
}

/// Open the system print dialog for the current page (print view).
#[tauri::command]
fn print_page(window: tauri::WebviewWindow) -> Result<(), String> {
    window.print().map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![render, render_run, symbologies, example_label, open_label, save_label, export_file, print_page])
        .run(tauri::generate_context!())
        .expect("error while running LabelForge");
}
