//! Pure danmaku helpers shared by the desktop and web shells.
//!
//! Ported from `src/native/danmaku.js`. The network fetch, cache and device identity stay in the
//! shells; only the value normalization and page parsing live here.

use hongguo_js::{as_text, js_trim, parse_int, string_to_number};
use serde_json::{Map, Value, json};

const MAX_TIME_SECONDS: f64 = 604800.0;
const PAGE_COUNT: i64 = 90;

fn is_number(value: &Value) -> bool {
    value.is_number() || value.get("$num").is_some()
}

fn finite_number(value: &Value) -> Option<f64> {
    let n = if let Some(n) = value.as_f64() {
        n
    } else {
        let s = value.get("$num").and_then(Value::as_str)?;
        match s {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            "-0" => -0.0,
            _ => return None,
        }
    };
    n.is_finite().then_some(n)
}

fn color_from_number(value: f64) -> String {
    if value < 0.0 {
        return "#FFFFFF".into();
    }
    // JS `&` first converts to int32 via ToInt32 (truncate toward zero, wrap mod 2^32).
    let int = value as i32;
    format!("#{:06X}", int & 0x00FF_FFFF)
}

/// `normalizeColor`: fold numeric and hex colors into a `#RRGGBB` string.
pub fn normalize_color(value: &Value) -> String {
    if is_number(value) {
        return match finite_number(value) {
            Some(n) => color_from_number(n),
            None => "#FFFFFF".into(),
        };
    }
    if let Value::String(text) = value {
        let text = js_trim(text);
        if text.is_empty() {
            return "#FFFFFF".into();
        }
        let lower = text.to_ascii_lowercase();
        if lower.len() == 7
            && lower.starts_with('#')
            && lower[1..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return text.to_ascii_uppercase();
        }
        if lower.len() == 4
            && lower.starts_with('#')
            && lower[1..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            let expanded: String = lower[1..].chars().map(|c| format!("{c}{c}")).collect();
            return format!("#{}", expanded.to_ascii_uppercase());
        }
        // `^(?:0x[0-9a-f]+|[0-9]+)$`: a `0x` prefix allows hex digits, bare text only decimals.
        let (digits, radix_ok) = if let Some(rest) = lower.strip_prefix("0x") {
            (
                rest,
                !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_hexdigit()),
            )
        } else {
            (
                lower.as_str(),
                !lower.is_empty() && lower.bytes().all(|b| b.is_ascii_digit()),
            )
        };
        if radix_ok {
            let parsed = if lower.starts_with("0x") {
                i64::from_str_radix(digits, 16)
                    .map(|n| n as f64)
                    .unwrap_or(f64::NAN)
            } else {
                string_to_number(digits)
            };
            if parsed.is_finite() && parsed >= 0.0 {
                return color_from_number(parsed);
            }
        }
    }
    "#FFFFFF".into()
}

fn is_object(value: &Value) -> bool {
    matches!(value, Value::Object(_)) && !hongguo_js::is_undefined(value)
}

/// `normalizeMode`: find an explicit position token in any holder, else `scroll`.
pub fn normalize_mode(holders: &[Value]) -> String {
    for holder in holders {
        if !is_object(holder) || holder.is_null() {
            continue;
        }
        for key in [
            "mode",
            "danmaku_type",
            "position",
            "fix_type",
            "danmaku_position",
        ] {
            let value = hongguo_js::prop(holder, key);
            if value.is_null() || hongguo_js::is_undefined(value) {
                continue;
            }
            let token = js_trim(&as_text(value)).to_lowercase();
            if ["top", "2", "fix_top", "顶部"].contains(&token.as_str()) {
                return "top".into();
            }
            if ["bottom", "3", "fix_bottom", "底部"].contains(&token.as_str()) {
                return "bottom".into();
            }
        }
    }
    "scroll".into()
}

/// `cleanControl`: replace control characters and line separators with spaces, then trim.
pub fn clean_control(text: &Value) -> String {
    let raw = hongguo_js::js_string(text);
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        let code = ch as u32;
        if code < 0x20 || code == 0x7f || ch == '\u{2028}' || ch == '\u{2029}' {
            out.push(' ');
        } else {
            out.push(ch);
        }
    }
    js_trim(&out)
}

/// `toSeconds`: milliseconds to seconds, rejecting out-of-range and non-numeric values.
pub fn to_seconds(value: &Value) -> Option<f64> {
    let num = if is_number(value) {
        finite_number(value)?
    } else if let Value::String(text) = value {
        let digits = text.split_once('.').map_or(text.as_str(), |(a, _)| a);
        let decimals = text.split_once('.').map(|(_, b)| b);
        if digits.is_empty()
            || !digits.bytes().all(|b| b.is_ascii_digit())
            || decimals.is_some_and(|b| b.is_empty() || !b.bytes().all(|c| c.is_ascii_digit()))
        {
            return None;
        }
        match string_to_number(text) {
            n if n.is_finite() => n,
            _ => return None,
        }
    } else {
        return None;
    };
    let seconds = num / 1000.0;
    if seconds < 0.0 || seconds > MAX_TIME_SECONDS {
        return None;
    }
    Some(seconds)
}

/// `buildBody`: the POST payload for one danmaku time window.
pub fn build_body(vid: &Value, book_id: &Value, start: &Value, duration: &Value) -> Value {
    let business = json!({
        "book_id": book_id,
        "start_offset_time": start,
        "playlet_item_duration": duration,
        "need_danmaku_guide_type": [1, 3, 4, 2],
    });
    json!({
        "comment_source": 601,
        "server_channel": 1000,
        "group_id": vid,
        "group_type": 30,
        "comment_type": 20,
        "sort": 1,
        "count": PAGE_COUNT,
        "cursor": "",
        "aid": 8662,
        "compliance_status": 0,
        "business_param": business,
    })
}

/// `parsePage`: one response page into `{ items, nextMs, hasMore }`.
pub fn parse_page(payload: &Value, vid: &Value) -> Value {
    let empty = || json!({ "items": [], "nextMs": null, "hasMore": null });
    if !is_object(payload) {
        return empty();
    }
    let data = hongguo_js::prop(payload, "data");
    if !is_object(data) {
        return empty();
    }
    let empty_rows: Vec<Value> = Vec::new();
    let rows = match hongguo_js::prop(data, "data_list") {
        Value::Array(items) => items,
        _ => &empty_rows,
    };

    let mut next_ms: Option<f64> = None;
    let mut has_more: Option<bool> = None;

    let extra = hongguo_js::prop(data, "extra");
    if is_object(extra) {
        let raw = js_trim(&as_text(hongguo_js::prop(
            extra,
            "next_query_danmaku_list_time",
        )));
        if !raw.is_empty() {
            let parsed = parse_int(&raw, 10);
            if parsed.is_finite() {
                next_ms = Some(parsed);
            }
        }
    }

    let info = hongguo_js::prop(data, "common_list_info");
    if is_object(info) {
        let flag = hongguo_js::prop(info, "has_more");
        if let Value::Bool(b) = flag {
            has_more = Some(*b);
        } else if let Value::String(s) = flag {
            let token = js_trim(s).to_lowercase();
            has_more = Some(["1", "true", "yes"].contains(&token.as_str()));
        } else if !as_text(flag).is_empty() {
            let token = js_trim(&as_text(flag)).to_lowercase();
            has_more = Some(!["0", "false", "no"].contains(&token.as_str()));
        }

        let mut cursor = hongguo_js::prop(info, "cursor").clone();
        if let Value::String(s) = &cursor
            && js_trim(s).starts_with('{')
        {
            cursor = serde_json::from_str(s).unwrap_or(Value::Null);
        }
        if is_object(&cursor) {
            for key in ["end_offset_time", "next_offset_time", "offset"] {
                let candidate = parse_int(&as_text(hongguo_js::prop(&cursor, key)), 10);
                if !candidate.is_finite() {
                    continue;
                }
                if next_ms.is_none() || candidate > next_ms.unwrap_or(f64::NEG_INFINITY) {
                    next_ms = Some(candidate);
                }
                break;
            }
        }
    }

    if has_more == Some(false) {
        next_ms = None;
    }

    let vid_text = as_text(vid);
    let mut items = Vec::new();
    for raw in rows {
        if !is_object(raw) {
            continue;
        }
        let comment = hongguo_js::prop(raw, "comment");
        if !is_object(comment) {
            continue;
        }
        let common = hongguo_js::prop(comment, "common");
        let common = if is_object(common) {
            common
        } else {
            &Value::Null
        };

        let group_id = as_text(hongguo_js::prop(common, "group_id"));
        if !group_id.is_empty() && group_id != vid_text {
            continue;
        }
        if as_text(hongguo_js::prop(common, "status")) != "1" {
            continue;
        }

        let content = hongguo_js::prop(common, "content");
        let text = if is_object(content) {
            as_text(hongguo_js::prop(content, "text"))
        } else {
            String::new()
        };

        let expand = hongguo_js::prop(comment, "expand");
        let expand = if is_object(expand) {
            expand
        } else {
            &Value::Null
        };
        let mut time_raw = hongguo_js::prop(expand, "offset_time").clone();
        if time_raw.is_null() || hongguo_js::is_undefined(&time_raw) {
            time_raw = hongguo_js::prop(comment, "offset_time").clone();
        }

        let mut color_raw: Option<Value> = None;
        for holder in [content, common, expand, comment] {
            if !is_object(holder) {
                continue;
            }
            for key in ["color", "font_color", "text_color", "col"] {
                let v = hongguo_js::prop(holder, key);
                if !v.is_null() && !hongguo_js::is_undefined(v) {
                    color_raw = Some(v.clone());
                    break;
                }
            }
            if color_raw.is_some() {
                break;
            }
        }

        let mut item = Map::new();
        item.insert("text".into(), Value::String(text));
        item.insert("timeRaw".into(), time_raw);
        item.insert(
            "color".into(),
            Value::String(normalize_color(color_raw.as_ref().unwrap_or(&Value::Null))),
        );
        item.insert(
            "mode".into(),
            Value::String(normalize_mode(&[raw.clone(), comment.clone()])),
        );
        items.push(Value::Object(item));
    }

    let next_ms_value = match next_ms {
        Some(n) if n > 0.0 => number_value(n),
        _ => Value::Null,
    };
    json!({
        "items": items,
        "nextMs": next_ms_value,
        "hasMore": has_more.map(Value::Bool).unwrap_or(Value::Null),
    })
}

fn number_value(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        json!(n as i64)
    } else {
        json!(n)
    }
}
