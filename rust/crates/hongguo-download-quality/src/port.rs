//! The Rust port of `src/native/download-quality.js`.
//!
//! The JavaScript semantics live in the `hongguo-js` crate, so `undefined`, the default
//! parameter and the numeric coercion read the same way in every port. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff`
//! checks behavior against what the source recorded.

#![forbid(unsafe_code)]

use hongguo_js::*;
use serde_json::Value;

/// The error `normalizeDownloadQuality` raises for a value that is not a known quality.
const INVALID_QUALITY: &str = "下载清晰度无效，请选择自动、1080p 或 720p";

/// The error the parameter destructuring raises when the first argument is `null`.
///
/// The source destructures `{ width, height } = {}` in the parameter list, so `null` fails
/// while the argument is bound, before the body runs. V8 names the bound parameter with
/// placeholders, because the default value hides the original source text.
const NULL_DESTRUCTURE: &str = "Cannot destructure property 'width' of '(intermediate value)(intermediate value)(intermediate value)' as it is null.";

/// The known download qualities, in the order the source lists them.
const DOWNLOAD_QUALITIES: [&str; 3] = ["auto", "1080p", "720p"];

/// Port of `normalizeDownloadQuality` (`src/native/download-quality.js:3`).
pub fn normalize_download_quality(value: Value) -> Result<Value, String> {
    // The default parameter fires for `undefined` only; `null` stays `null` and fails.
    let value = if is_undefined(&value) {
        Value::String("auto".into())
    } else {
        value
    };
    let text = value.as_str().unwrap_or_default();
    if DOWNLOAD_QUALITIES.contains(&text) {
        Ok(value)
    } else {
        Err(INVALID_QUALITY.to_string())
    }
}

/// Port of `meetsDownloadQuality` (`src/native/download-quality.js:8`).
pub fn meets_download_quality(dimensions: Value, quality: Value) -> Result<Value, String> {
    // The parameter destructuring runs before the body, so `null` fails here even when the
    // quality is invalid too.
    let source = if is_undefined(&dimensions) {
        Value::Object(serde_json::Map::new())
    } else if dimensions.is_null() {
        return Err(NULL_DESTRUCTURE.to_string());
    } else {
        dimensions
    };
    let width = js_index_get(&source, "width")?;
    let height = js_index_get(&source, "height")?;

    let quality = normalize_download_quality(quality)?;
    let quality_text = quality.as_str().unwrap_or_default();
    if quality_text == "auto" {
        return Ok(Value::Bool(true));
    }

    let measured = [to_number(width), to_number(height)];
    let enough = measured
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
        && js_min_all(measured) >= parse_int(quality_text, 10);
    Ok(Value::Bool(enough))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quality(value: &str) -> Value {
        Value::String(value.into())
    }

    #[test]
    fn keeps_the_known_qualities() {
        for known in ["auto", "1080p", "720p"] {
            assert_eq!(
                normalize_download_quality(quality(known)).unwrap(),
                quality(known)
            );
        }
        assert!(normalize_download_quality(undefined()).is_ok());
        assert!(normalize_download_quality(Value::Null).is_err());
        assert!(normalize_download_quality(quality("720P")).is_err());
    }

    #[test]
    fn measures_the_smaller_side() {
        let wide = serde_json::json!({ "width": 1920, "height": 1080 });
        let short = serde_json::json!({ "width": 1920, "height": 720 });
        let meets = |size: &Value, wanted: &str| {
            meets_download_quality(size.clone(), quality(wanted)).unwrap()
        };
        assert_eq!(meets(&wide, "1080p"), Value::Bool(true));
        assert_eq!(meets(&wide, "720p"), Value::Bool(true));
        assert_eq!(meets(&short, "1080p"), Value::Bool(false));
        assert_eq!(
            meets_download_quality(undefined(), undefined()).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn refuses_a_null_first_argument() {
        assert!(meets_download_quality(Value::Null, quality("auto")).is_err());
    }
}
