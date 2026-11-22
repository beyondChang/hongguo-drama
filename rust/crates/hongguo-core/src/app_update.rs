//! Release-tag version comparison shared by the update check.
//!
//! `app-update.js` compares a GitHub release tag against the running version. The parsing and
//! comparison are domain logic (the same tag rules appear in update manifests), so they live here
//! rather than in the bridge crate.

use hongguo_js::{encode_uri_component, js_string, number_is_safe_integer, prop, truthy};
use serde_json::{Map, Value};

const RELEASES_URL: &str = "https://github.com/jackotom/hongguo-drama/releases";

/// `Number(text)` for the digit-only capture groups of the version regex.
fn capture_number(text: &str) -> f64 {
    text.parse::<f64>().unwrap_or(f64::NAN)
}

/// Port of the internal `versionParts` (`src/native/app-update.js:3`).
fn version_parts(value: &Value) -> Option<[f64; 3]> {
    let text = if truthy(value) {
        js_string(value)
    } else {
        String::new()
    };
    let body = text.strip_prefix('v').unwrap_or(&text);
    let mut dots = body.split('.');
    let major = dots.next()?;
    let minor = dots.next()?;
    let patch = dots.next();
    if dots.next().is_some() {
        return None;
    }
    // `\d+` requires at least one ASCII digit and nothing else in the segment.
    fn digits(s: &str) -> bool {
        !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
    }
    if !digits(major) || !digits(minor) {
        return None;
    }
    let patch = match patch {
        Some(p) if digits(p) => p,
        Some(_) => return None,
        None => "0",
    };
    let parts = [
        capture_number(major),
        capture_number(minor),
        capture_number(patch),
    ];
    parts
        .into_iter()
        .all(number_is_safe_integer)
        .then_some(parts)
}

/// `availableRelease(data, currentVersion)`: the newest public release, or `null` when the tag is
/// not newer, is a draft/pre-release, or either version cannot be parsed.
pub fn available_release(data: &Value, current_version: &Value) -> Value {
    if !truthy(data) || truthy(prop(data, "draft")) || truthy(prop(data, "prerelease")) {
        return Value::Null;
    }
    let tag = prop(data, "tag_name");
    let (Some(next), Some(current)) = (version_parts(tag), version_parts(current_version)) else {
        return Value::Null;
    };
    let Some(index) = next.iter().zip(current.iter()).position(|(a, b)| a != b) else {
        return Value::Null;
    };
    if next[index] < current[index] {
        return Value::Null;
    }
    let tag_text = js_string(tag);
    let version = match tag_text.strip_prefix('v') {
        Some(rest) => rest.to_string(),
        None => tag_text.clone(),
    };
    // `^(\d+\.\d+)\.0$` -> `$1`: drop a trailing `.0` after a `major.minor`.
    let version = if let Some(head) = version.strip_suffix(".0") {
        let mut parts = head.split('.');
        let a = parts.next().unwrap_or("");
        let b = parts.next();
        if !a.is_empty()
            && a.bytes().all(|c| c.is_ascii_digit())
            && b.is_some_and(|b| !b.is_empty() && b.bytes().all(|c| c.is_ascii_digit()))
            && parts.next().is_none()
        {
            head.to_string()
        } else {
            version
        }
    } else {
        version
    };
    let url = format!("{}/tag/{}", RELEASES_URL, encode_uri_component(&tag_text));
    let mut result = Map::new();
    result.insert("version".into(), Value::String(version));
    result.insert("url".into(), Value::String(url));
    Value::Object(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_a_newer_release() {
        let release = available_release(&json!({ "tag_name": "v7.9.1" }), &json!("7.9"));
        assert_eq!(release["version"], "7.9.1");
        assert_eq!(
            release["url"],
            "https://github.com/jackotom/hongguo-drama/releases/tag/v7.9.1"
        );
    }

    #[test]
    fn rejects_old_draft_and_unparsable() {
        assert_eq!(
            available_release(&json!({ "tag_name": "v7.8" }), &json!("7.9")),
            Value::Null
        );
        assert_eq!(
            available_release(&json!({ "tag_name": "v7.9", "draft": true }), &json!("7.8")),
            Value::Null
        );
        assert_eq!(
            available_release(&json!({ "tag_name": "junk" }), &json!("7.8")),
            Value::Null
        );
    }
}
