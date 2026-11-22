//! Tauri 2 desktop shell command layer (ADR-002/ADR-003).
//!
//! Each function here mirrors one `window.*` contract from `preload.js`: the renderer keeps its
//! React code unchanged, and this layer translates the IPC payload into the same Rust unit the
//! other two shells call. When the `tauri-runtime` feature is enabled, `bin/desktop.rs` registers
//! these as `#[tauri::command]` handlers; the command bodies stay shared and unit-testable.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// `window.getMediaSources()` -> `get-media-sources`.
pub fn get_media_sources(settings: Value) -> Result<Value, String> {
    hongguo_media_sources::port::get_media_sources(settings)
}

/// `window.setBuiltinSourceEnabled(id, enabled)` -> `set-builtin-source-enabled`.
pub fn set_builtin_source_enabled(
    settings: Value,
    id: Value,
    enabled: Value,
) -> Result<Value, String> {
    hongguo_media_sources::port::set_builtin_source_enabled(settings, id, enabled)
}

/// `window.changeWindowZoom(direction)` -> `change-window-zoom`. The Node main process stores the
/// new zoom; the pure step (clamp) is the same `getWindowScale` the unit crate exposes.
pub fn change_window_zoom(current: Value, direction: Value) -> Result<Value, String> {
    let current = hongguo_window_layout::port::get_window_scale(current)?;
    let value = match direction.as_str().unwrap_or("") {
        "in" => current.as_f64().unwrap_or(1.0) + 0.1,
        "out" => current.as_f64().unwrap_or(1.0) - 0.1,
        "reset" => 1.0,
        _ => return Err("Invalid zoom direction".into()),
    };
    // `Math.round((value) * 10) / 10` then clamp, matching `saveWindowZoom`.
    let rounded = (value * 10.0).round() / 10.0;
    hongguo_window_layout::port::get_window_scale(json!(rounded))
}

/// `window.normalizeDownloadQuality(quality)` -> the shared quality normalizer.
pub fn normalize_download_quality(quality: Value) -> Result<Value, String> {
    hongguo_download_quality::port::normalize_download_quality(quality)
}

/// `window.checkRelease(data, currentVersion)` -> the shared release check.
pub fn check_release(data: Value, current_version: Value) -> Result<Value, String> {
    hongguo_app_update::port::available_release(data, current_version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn zoom_commands_clamp_and_reject() {
        assert_eq!(
            change_window_zoom(json!(1), json!("in")).unwrap(),
            json!(1.1)
        );
        assert_eq!(
            change_window_zoom(json!(1.96), json!("in")).unwrap(),
            json!(2)
        );
        assert_eq!(
            change_window_zoom(json!(1.5), json!("reset")).unwrap(),
            json!(1)
        );
        assert!(change_window_zoom(json!(1), json!("sideways")).is_err());
    }

    #[test]
    fn source_commands_preserve_fixed_hongguo() {
        let settings = json!({ "sources": {} });
        let updated = set_builtin_source_enabled(settings, json!("dramabox"), json!(true)).unwrap();
        let listed = get_media_sources(updated).unwrap();
        let ids: Vec<&str> = listed
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap())
            .collect();
        // hongguo is always present; dramabox was just enabled; reelshort stays off.
        assert_eq!(ids, vec!["hongguo", "xifan", "hema", "dramabox"]);
        assert!(set_builtin_source_enabled(json!({}), json!("hongguo"), json!(false)).is_err());
    }

    #[test]
    fn quality_and_release_commands_delegate() {
        let quality = normalize_download_quality(json!("1080p")).unwrap();
        assert_eq!(quality, json!("1080p"));
        let release = check_release(json!({ "tag_name": "v8.0.0" }), json!("7.9")).unwrap();
        assert_eq!(release["version"], "8.0");
    }
}
