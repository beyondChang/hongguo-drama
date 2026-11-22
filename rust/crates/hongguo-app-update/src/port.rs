//! The Rust port of `src/native/app-update.js`.
//!
//! The release-tag parsing and comparison live in `hongguo-core::app_update`; this crate is the
//! bridge to the original module. After changing a signature, run `rust-anything refresh`.

#![forbid(unsafe_code)]

/// Port of `availableRelease` (`src/native/app-update.js:10`).
pub fn available_release(
    data: serde_json::Value,
    current_version: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(hongguo_core::app_update::available_release(
        &data,
        &current_version,
    ))
}
