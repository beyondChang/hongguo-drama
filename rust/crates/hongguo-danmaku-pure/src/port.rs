//! The Rust port of the pure helpers in `src/native/danmaku.js`, reached through a bridge-only
//! wrapper. The logic lives in `hongguo-core::danmaku`; network/cache stay in the shells.

#![forbid(unsafe_code)]

use hongguo_js::{is_undefined, truthy};
use serde_json::Value;

pub fn normalize_color(value: Value) -> Result<Value, String> {
    Ok(Value::String(hongguo_core::danmaku::normalize_color(
        &value,
    )))
}

/// The wrapper calls `d.normalizeMode(...(holders || []))`. Reproduce that here so any falsy or
/// scalar first argument degrades the same way the original does.
pub fn normalize_mode(holders: Value) -> Result<Value, String> {
    let values: Vec<Value> = if !truthy(&holders) || is_undefined(&holders) {
        Vec::new()
    } else {
        match &holders {
            Value::Array(items) => items.clone(),
            Value::String(text) => text.chars().map(|c| Value::String(c.to_string())).collect(),
            // `...value` on a truthy non-iterable throws in JS.
            _ => {
                return Err(
                    "Spread syntax requires ...iterable[Symbol.iterator] to be a function".into(),
                );
            }
        }
    };
    Ok(Value::String(hongguo_core::danmaku::normalize_mode(
        &values,
    )))
}

pub fn clean_control(text: Value) -> Result<Value, String> {
    Ok(Value::String(hongguo_core::danmaku::clean_control(&text)))
}

pub fn to_seconds(value: Value) -> Result<Value, String> {
    Ok(hongguo_core::danmaku::to_seconds(&value)
        .map(hongguo_js::number)
        .unwrap_or(Value::Null))
}

pub fn parse_page(payload: Value, vid: Value) -> Result<Value, String> {
    Ok(hongguo_core::danmaku::parse_page(&payload, &vid))
}

pub fn build_body(
    vid: Value,
    book_id: Value,
    start: Value,
    duration: Value,
) -> Result<Value, String> {
    Ok(hongguo_core::danmaku::build_body(
        &vid, &book_id, &start, &duration,
    ))
}
