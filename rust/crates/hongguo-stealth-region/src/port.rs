//! The Rust port of `src/native/stealth-region.js`.
//!
//! JavaScript semantics live in `hongguo-js`. After changing a signature, run
//! `rust-anything refresh` to regenerate the bindings.

#![forbid(unsafe_code)]

use hongguo_js::{number, number_is_finite};
use serde_json::{Map, Value, json};

const KEYS: [&str; 4] = ["x", "y", "width", "height"];
const ERROR: &str = "小窗唤醒区域无效：区域须在窗口内，宽高至少为 5%";

fn check(value: &Value) -> Result<Value, String> {
    let Some(object) = value.as_object() else {
        return Err(ERROR.into());
    };
    if object.len() != KEYS.len() {
        return Err(ERROR.into());
    }
    let mut numbers = [0.0; 4];
    for (index, key) in KEYS.iter().enumerate() {
        let Some(part) = object.get(*key) else {
            return Err(ERROR.into());
        };
        if !number_is_finite(part) {
            return Err(ERROR.into());
        }
        numbers[index] = part.as_f64().unwrap_or(f64::NAN);
    }
    if numbers[0] < 0.0
        || numbers[1] < 0.0
        || numbers[2] < 0.05
        || numbers[3] < 0.05
        || numbers[0] + numbers[2] > 1.0
        || numbers[1] + numbers[3] > 1.0
    {
        return Err(ERROR.into());
    }
    let mut copy = Map::new();
    for (key, value) in KEYS.iter().zip(numbers) {
        copy.insert((*key).to_string(), number(value));
    }
    Ok(Value::Object(copy))
}

/// Port of `validateStealthWakeRegion` (`src/native/stealth-region.js:3`).
pub fn validate_stealth_wake_region(value: Value) -> Result<Value, String> {
    check(&value)
}

/// Port of `normalizeStealthWakeRegion` (`src/native/stealth-region.js:12`).
pub fn normalize_stealth_wake_region(value: Value) -> Result<Value, String> {
    check(&value).or_else(|_| {
        Ok(json!({
            "x": 0,
            "y": 0,
            "width": 1,
            "height": 1,
        }))
    })
}
