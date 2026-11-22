//! The Rust port of `src/native/error-text.js`.
//!
//! JavaScript semantics (ToString, ToIntegerOrInfinity) live in `hongguo-js`. `sanitizeLogValue`
//! checks `instanceof Error`, so the tool keeps that thin wrapper in JS. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings.

#![forbid(unsafe_code)]

use hongguo_js::{is_nullish, is_undefined, js_string, to_integer, to_number};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use url::Url;

/// JavaScript's `\s`, which also includes U+FEFF (Rust's `\s` does not).
const JS_SPACE: &str = r"\t\n\x0B\f\r \u{00A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}";

const ERROR_FIELDS: &str =
    "^(?:error|apiError|warning|reason|lastCheckError|lastEpisodeCheckError|lastSequelCheckError)$";

static CREDENTIALS: OnceLock<Regex> = OnceLock::new();
static SENSITIVE: OnceLock<Regex> = OnceLock::new();
static HTTP_URL: OnceLock<Regex> = OnceLock::new();
static ERROR_FIELD: OnceLock<Regex> = OnceLock::new();

fn credentials() -> &'static Regex {
    CREDENTIALS.get_or_init(|| {
        Regex::new(&format!(r"([a-z][a-z0-9+.-]*://)[^{JS_SPACE}/@]*@")).expect("valid regex")
    })
}

fn sensitive() -> &'static Regex {
    SENSITIVE.get_or_init(|| {
        Regex::new(&format!(
            r#"(?-u:\b)(Bearer|decryption_key)[{JS_SPACE}]+[^{JS_SPACE}]+"#
        ))
        .expect("valid regex")
    })
}

fn http_url() -> &'static Regex {
    HTTP_URL.get_or_init(|| {
        Regex::new(&format!(r#"(?-u:\b)https?://[^{JS_SPACE}<>"'）]+"#)).expect("valid regex")
    })
}

fn error_field() -> &'static Regex {
    ERROR_FIELD.get_or_init(|| Regex::new(&format!("(?i){ERROR_FIELDS}")).expect("valid regex"))
}

/// `new URL(raw)`: the origin JS would print, or `None` when the URL is invalid.
fn origin_of(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let url = Url::parse(raw).ok()?;
    let scheme = url.scheme();
    if scheme != "http" && scheme != "https" {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

/// `safeErrorText(value, maxLength = 600)`.
pub fn safe_error_text(value: Value, max_length: Value) -> Result<Value, String> {
    let value = if is_nullish(&value) {
        Value::String("操作失败".into())
    } else {
        value
    };
    let mut text = js_string(&value);
    text = credentials()
        .replace_all(&text, "$1[认证已隐藏]@")
        .into_owned();
    text = sensitive().replace_all(&text, "$1 [已隐藏]").into_owned();
    text = http_url()
        .replace_all(&text, |found: &regex::Captures| {
            let raw = found.get(0).map_or("", |m| m.as_str());
            let trailing_len = raw
                .chars()
                .rev()
                .take_while(|c| matches!(c, ')' | ',' | '.' | ';' | '!' | '?'))
                .count();
            let clean_end = raw.len() - trailing_len;
            let (clean, trailing) = raw.split_at(clean_end);
            match origin_of(clean) {
                Some(origin) => format!("{origin}/...{trailing}"),
                None => format!("[来源地址已隐藏]{trailing}"),
            }
        })
        .into_owned();

    let max_length = if is_undefined(&max_length) {
        Value::from(600)
    } else {
        max_length
    };
    let mut end = to_integer(&max_length);
    if end < 0.0 {
        end = (text.chars().count() as f64 + end).max(0.0);
    }
    let end = end.min(text.chars().count() as f64) as usize;
    Ok(Value::String(text.chars().take(end).collect()))
}

/// A wire marker object is not a plain object in JS: it revives to a Date, Map, Set, bigint,
/// etc., so `Object.getPrototypeOf` is not `Object.prototype` and the source returns it as-is.
fn is_opaque(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.len() != 1 {
        return false;
    }
    let key = object.keys().next().map_or("", String::as_str);
    matches!(
        key,
        "$date" | "$map" | "$set" | "$bytes" | "$buffer" | "$bigint" | "$utf16" | "$num"
    )
}

/// The wire cannot carry a WeakMap: `undefined` means the default one. A `Map` or `Set`
/// marker revives to an object that also has `.has`, so those are accepted too. Any other
/// explicit value raises the same errors V8 does when the source calls `seen.has(value)`.
fn check_seen(seen: &Value) -> Result<(), String> {
    if is_undefined(seen) {
        return Ok(());
    }
    if seen.is_null() {
        return Err("Cannot read properties of null (reading 'has')".into());
    }
    if seen.as_object().is_some_and(|object| {
        object.len() == 1
            && object
                .keys()
                .next()
                .is_some_and(|key| key == "$map" || key == "$set")
    }) {
        return Ok(());
    }
    Err("seen.has is not a function".into())
}

/// `sanitizeIpcValue(value, depth = 0, seen = new WeakMap())`.
///
/// The wire has no object identity, so a valid `seen` cannot carry entries. Every value that
/// arrives is a fresh tree, which makes the WeakMap a no-op here.
pub fn sanitize_ipc_value(value: Value, depth: Value, seen: Value) -> Result<Value, String> {
    let depth = to_number(&depth);
    if is_nullish(&value) || matches!(value, Value::Bool(_) | Value::Number(_) | Value::String(_)) {
        return Ok(value);
    }
    if depth > 8.0 {
        return Ok(value);
    }
    check_seen(&seen)?;
    let next_depth = Value::from(depth + 1.0);
    if let Some(items) = value.as_array() {
        let copy = items
            .iter()
            .map(|item| sanitize_ipc_value(item.clone(), next_depth.clone(), seen.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(Value::Array(copy));
    }
    if is_opaque(&value) {
        return Ok(value);
    }
    let Some(object) = value.as_object() else {
        // A function or symbol cannot cross the wire; it arrives as `undefined`.
        return Ok(value);
    };
    let mut copy = Map::new();
    for (key, item) in object {
        if item.is_string() && error_field().is_match(key) {
            let safe = safe_error_text(item.clone(), Value::from(600))?;
            copy.insert(key.clone(), safe);
        } else {
            let nested = sanitize_ipc_value(item.clone(), next_depth.clone(), seen.clone())?;
            copy.insert(key.clone(), nested);
        }
    }
    Ok(Value::Object(copy))
}
