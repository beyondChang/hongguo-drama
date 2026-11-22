//! The Rust port of `src/native/ui-language.mjs`.
//!
//! JavaScript semantics live in `hongguo-js`. After changing a signature, run
//! `rust-anything refresh` to regenerate the bindings.

#![forbid(unsafe_code)]

use hongguo_js::{
    is_undefined, js_index_get, js_trim, null_property_error, string_or_empty, truthy,
};
use serde_json::{Value, json};

hongguo_js::js_regex!(zh_locale, r"(?i)^zh(?:[-_.@]|$)");

const OPTIONS: [&str; 3] = ["system", "zh-CN", "en"];
const UNSUPPORTED: &str = "Unsupported language preference";

fn is_option(value: &Value) -> bool {
    value.as_str().is_some_and(|text| OPTIONS.contains(&text))
}

/// The error V8 raises when calling `storage.<method>(...)` on a value that crossed the wire.
/// Functions cannot cross JSON, so a real method never survives: only the receiver shape
/// decides which error the original raises.
fn method_error(storage: &Value, method: &str) -> String {
    if storage.is_null() || is_undefined(storage) {
        null_property_error(method, storage)
    } else {
        format!("storage.{method} is not a function")
    }
}

/// Port of `resolveLanguage` (`src/native/ui-language.mjs:3`).
pub fn resolve_language(preference: Value, locale: Value) -> Result<Value, String> {
    if let Some(text) = preference.as_str()
        && (text == "zh-CN" || text == "en")
    {
        return Ok(preference);
    }
    let locale = js_trim(&string_or_empty(&locale));
    Ok(Value::String(
        if zh_locale().is_match(&locale) {
            "zh-CN"
        } else {
            "en"
        }
        .into(),
    ))
}

/// Port of `systemLocale` (`src/native/ui-language.mjs:7`).
pub fn system_locale(env: Value, locale: Value, platform: Value) -> Result<Value, String> {
    let env = if is_undefined(&env) { json!({}) } else { env };
    let locale = if is_undefined(&locale) {
        Value::String(String::new())
    } else {
        locale
    };
    let platform = if is_undefined(&platform) {
        Value::String(String::new())
    } else {
        platform
    };
    if platform != Value::String("linux".into()) {
        return Ok(locale);
    }
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let value = js_index_get(&env, key)?;
        if truthy(value) {
            return Ok(value.clone());
        }
    }
    Ok(locale)
}

/// Port of `readPreference` (`src/native/ui-language.mjs:10`).
pub fn read_preference(storage: Value) -> Result<Value, String> {
    // `storage.getItem(...)` always throws for a JSON value (no function survives the wire),
    // and the `catch` returns `"system"`.
    let _ = method_error(&storage, "getItem");
    Ok(Value::String("system".into()))
}

/// Port of `savePreference` (`src/native/ui-language.mjs:14`).
pub fn save_preference(preference: Value, storage: Value) -> Result<Value, String> {
    if !is_option(&preference) {
        return Err(UNSUPPORTED.into());
    }
    Err(method_error(&storage, "setItem"))
}
