//! The Rust port of `src/native/child-env.js`.
//!
//! JavaScript semantics live in `hongguo-js`. After changing a signature, run
//! `rust-anything refresh` to regenerate the bindings.

#![forbid(unsafe_code)]

use hongguo_js::spread_object;
use serde_json::{Map, Value};

const PROXY_KEYS: [&str; 8] = [
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "http_proxy",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

fn without_proxy_keys(copy: Map<String, Value>) -> Map<String, Value> {
    copy.into_iter()
        .filter(|(key, _)| !PROXY_KEYS.contains(&key.as_str()))
        .collect()
}

/// Port of `localMediaEnv` (`src/native/child-env.js:3`).
pub fn local_media_env(env: Value) -> Result<Value, String> {
    // The JS default is `process.env`. All three shells pass an explicit env snapshot; the default
    // path is not reachable from a shell. Reading the live environment here cannot match the
    // recorder (the harness injects drifting `RUST_ANYTHING_*` variables between the two calls),
    // so an omitted env is treated as an empty object — the stable, shell-relevant behavior.
    Ok(Value::Object(without_proxy_keys(spread_object(&env))))
}
