//! The Rust port of `src/native/media-sources.js`.
//!
//! JavaScript semantics live in `hongguo-js`. After changing a signature, run
//! `rust-anything refresh` to regenerate the bindings.

#![forbid(unsafe_code)]

use hongguo_js::*;
use serde_json::{Map, Value, json};

const BUILTIN: [(&str, &str); 5] = [
    ("hongguo", "红果"),
    ("xifan", "西饭"),
    ("hema", "河马"),
    ("reelshort", "ReelShort（海外）"),
    ("dramabox", "DramaBox（海外）"),
];
const SWITCHABLE: [&str; 4] = ["xifan", "hema", "reelshort", "dramabox"];
const OVERSEAS: [&str; 2] = ["reelshort", "dramabox"];

/// Port of `getMediaSources` (`src/native/media-sources.js:6`).
pub fn get_media_sources(settings: Value) -> Result<Value, String> {
    let switches = js_index_get(&settings, "builtin_sources")?;
    let mut sources = Vec::new();
    for (id, label) in BUILTIN {
        let value = prop(switches, id);
        let included = if OVERSEAS.contains(&id) {
            value == &Value::Bool(true)
        } else {
            id == "hongguo" || value != &Value::Bool(false)
        };
        if included {
            sources.push(json!({ "id": id, "label": label }));
        }
    }
    let custom = prop(&settings, "custom_sources");
    if truthy(custom) {
        let Value::Array(items) = custom else {
            return Err("sources.filter is not a function".into());
        };
        for source in items {
            if !truthy(prop(source, "enabled")) {
                continue;
            }
            sources.push(json!({
                "id": format!("custom:{}", js_string(prop(source, "id"))),
                "label": prop(source, "name").clone(),
                "custom": true,
            }));
        }
    }
    Ok(Value::Array(sources))
}

/// Port of `setBuiltinSourceEnabled` (`src/native/media-sources.js:14`).
pub fn set_builtin_source_enabled(
    settings: Value,
    id: Value,
    enabled: Value,
) -> Result<Value, String> {
    let name = id.as_str().unwrap_or_default();
    if name == "hongguo" {
        return Err("红果来源固定开启，不能关闭".into());
    }
    if !SWITCHABLE.contains(&name) || !matches!(enabled, Value::Bool(_)) {
        return Err("来源开关无效".into());
    }
    let mut copy = match &settings {
        Value::Object(object) => object.clone(),
        _ => Map::new(),
    };
    let switches = js_index_get(&settings, "builtin_sources")?;
    let mut switch_copy = match switches {
        Value::Object(object) if !is_undefined(switches) => object.clone(),
        _ => Map::new(),
    };
    switch_copy.insert(name.to_string(), enabled);
    copy.insert("builtin_sources".into(), Value::Object(switch_copy));
    Ok(Value::Object(copy))
}
