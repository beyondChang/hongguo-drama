//! Tauri 2 window entry point. Built only with `--features tauri-runtime`, which needs the platform
//! WebView stack (WebView2 on Windows, WKWebView on macOS, webkitgtk on Linux). The command bodies
//! live in the library so they stay testable without a GUI; this file only registers them.

use serde_json::Value;

#[tauri::command]
fn get_media_sources(settings: Value) -> Result<Value, String> {
    hongguo_desktop::get_media_sources(settings)
}

#[tauri::command]
fn set_builtin_source_enabled(settings: Value, id: Value, enabled: Value) -> Result<Value, String> {
    hongguo_desktop::set_builtin_source_enabled(settings, id, enabled)
}

#[tauri::command]
fn change_window_zoom(current: Value, direction: Value) -> Result<Value, String> {
    hongguo_desktop::change_window_zoom(current, direction)
}

#[tauri::command]
fn normalize_download_quality(quality: Value) -> Result<Value, String> {
    hongguo_desktop::normalize_download_quality(quality)
}

#[tauri::command]
fn check_release(data: Value, current_version: Value) -> Result<Value, String> {
    hongguo_desktop::check_release(data, current_version)
}

fn main() {
    // The renderer keeps the existing React/preload contract. A later wave adds the real window,
    // asset pipeline and the `window.*` -> `invoke` adapter (ADR-003). This skeleton only proves
    // the command surface registers against the same Rust units as the server and Android shells.
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_media_sources,
            set_builtin_source_enabled,
            change_window_zoom,
            normalize_download_quality,
            check_release,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run hongguo-desktop");
}
