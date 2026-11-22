//! JavaScript semantics for the Rust ports of the hongguo modules.
//!
//! The ports keep the original's JavaScript behaviour: `undefined` is not `null`, a default
//! parameter fires only for `undefined`, a missing property is `undefined`, and `Set` membership
//! compares by value. JSON cannot express `undefined`, `NaN`, `-0` or the difference between a
//! missing property and `null`, so the wire protocol carries the markers this module reads:
//! `{"$undefined": true}` for `undefined` and `{"$num": "NaN"}` for a number JSON has no room
//! for. Every unit port reads those markers through here, so all ports agree on them.

use serde_json::{Map, Value};
use std::sync::OnceLock;

pub use regex::Regex as JsRegex;

/// Builds a lazily compiled regex that lives for the life of the program.
#[macro_export]
macro_rules! js_regex {
    ($name:ident, $pattern:expr) => {
        fn $name() -> &'static $crate::JsRegex {
            static RE: ::std::sync::OnceLock<$crate::JsRegex> = ::std::sync::OnceLock::new();
            RE.get_or_init(|| $crate::JsRegex::new($pattern).expect("valid regex"))
        }
    };
}

// ---------------------------------------------------------------------------
// Values JSON cannot express
// ---------------------------------------------------------------------------

/// JS `undefined` inside a structured value: `{ a: undefined }` is not `{ a: null }`.
pub fn undefined() -> Value {
    serde_json::json!({ "$undefined": true })
}

/// Whether a structured value received from JS was `undefined`.
pub fn is_undefined(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(|o| o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

/// A JS number inside a structured value. JSON has no NaN, Infinity or -0: those cross as
/// `{"$num": "NaN"}` and friends, which `index.cjs` turns back into numbers.
pub fn number(n: f64) -> Value {
    if n.is_nan() {
        serde_json::json!({ "$num": "NaN" })
    } else if n.is_infinite() {
        serde_json::json!({ "$num": if n > 0.0 { "Infinity" } else { "-Infinity" } })
    } else if n == 0.0 && n.is_sign_negative() {
        serde_json::json!({ "$num": "-0" })
    } else if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

/// A number received from JS inside a structured value, plain or marked.
pub fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) if o.len() == 1 => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

/// JS `Number.prototype.toString()` (what `${n}` and `String(n)` print): `1e+21`, `5e-324`,
/// `0` for `-0`, `NaN`, `Infinity`, and the shortest digits that read back as the same number.
pub fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.fract() == 0.0 && x.abs() < 9_007_199_254_740_992.0 {
        return (x as i64).to_string();
    }
    let json = serde_json::to_string(&x).unwrap_or_default();
    let (sign, json) = match json.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", json.as_str()),
    };
    let (mantissa, exponent) = match json.find(['e', 'E']) {
        Some(i) => (&json[..i], json[i + 1..].parse::<i64>().unwrap_or(0)),
        None => (json, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all = format!("{int}{frac}");
    let lead = all.len() - all.trim_start_matches('0').len();
    let digits = all[lead..].trim_end_matches('0');
    let k = digits.len() as i64;
    let n = int.len() as i64 - lead as i64 + exponent;
    let mut out = String::with_capacity(digits.len() + 8);
    out.push_str(sign);
    if k <= n && n <= 21 {
        out.push_str(digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(digits);
    } else {
        let e = n - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.abs().to_string());
    }
    out
}

/// JS `Math.round`: the nearest integer, halves toward positive infinity (`-2.5` gives `-2`,
/// where Rust's `round` gives `-3`), and `-0` for negatives that round to zero.
pub fn math_round(x: f64) -> f64 {
    if !x.is_finite() || x == x.trunc() {
        return x;
    }
    let floor = x.floor();
    let r = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if r == 0.0 && x < 0.0 { -0.0 } else { r }
}

/// JS `Array.prototype.sort(compare)`, as V8 runs it.
///
/// With a consistent comparator any stable sort agrees with JS. With one that returns NaN or
/// contradicts itself, the order depends on the algorithm: under 64 elements this reproduces
/// V8's TimSort exactly (one run, then binary insertion); longer arrays use Rust's stable sort.
pub fn sort<T>(items: &mut [T], mut compare: impl FnMut(&T, &T) -> f64) {
    let n = items.len();
    if n < 2 {
        return;
    }
    if n >= 64 {
        items.sort_by(|a, b| {
            compare(a, b)
                .partial_cmp(&0.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        return;
    }
    // CountAndMakeRun: the leading ascending run, or strictly descending run (reversed).
    let mut run = 2;
    let descending = compare(&items[1], &items[0]) < 0.0;
    while run < n {
        let order = compare(&items[run], &items[run - 1]);
        if (descending && order >= 0.0) || (!descending && order < 0.0) {
            break;
        }
        run += 1;
    }
    if descending {
        items[..run].reverse();
    }
    // BinaryInsertionSort of the rest.
    for start in run..n {
        let (mut left, mut right) = (0, start);
        while left < right {
            let mid = left + ((right - left) >> 1);
            if compare(&items[start], &items[mid]) < 0.0 {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        items[left..=start].rotate_right(1);
    }
}

// ---------------------------------------------------------------------------
// Truthiness and coercion
// ---------------------------------------------------------------------------

/// `value == null` in JS: `null` and `undefined` are both nullish.
pub fn is_nullish(value: &Value) -> bool {
    value.is_null() || is_undefined(value)
}

/// JS truthiness: `undefined`, `null`, `false`, `0`, `-0`, `NaN` and `""` are falsy.
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(f64::NAN);
            f != 0.0 && !f.is_nan()
        }
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !is_undefined(value),
    }
}

/// JS `Number(value)`.
pub fn to_number(value: &Value) -> f64 {
    // The wire carries NaN, Infinity and -0 as `{"$num": ...}` markers; those revive to
    // numbers on the JS side, so they must convert like numbers here too.
    if let Some(number) = as_number(value) {
        return number;
    }
    match value {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => string_to_number(s),
        // `Number([])` is 0 and `Number([5])` is 5: an array turns into its joined text.
        Value::Array(items) => {
            let joined = items
                .iter()
                .map(|item| match item {
                    Value::Null => String::new(),
                    other => js_string(other),
                })
                .collect::<Vec<_>>()
                .join(",");
            string_to_number(&joined)
        }
        Value::Object(_) => f64::NAN,
    }
}

/// JS `Number(string)`: the empty string is 0, a numeric prefix is parsed, anything else is NaN.
pub fn string_to_number(text: &str) -> f64 {
    let trimmed = text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if trimmed.is_empty() {
        return 0.0;
    }
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if trimmed == "Infinity" || trimmed == "+Infinity" {
        return f64::INFINITY;
    }
    if trimmed == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    // Rust's parser accepts forms JS does not ("inf", "1_0", "1."), so reject them first.
    if trimmed
        .chars()
        .any(|c| !(c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E')))
    {
        return f64::NAN;
    }
    trimmed.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number(value) || 0`: JS truthiness collapses 0, -0 and NaN to 0.
pub fn number_or_zero(value: &Value) -> f64 {
    let n = to_number(value);
    if n == 0.0 || n.is_nan() { 0.0 } else { n }
}

/// JS `parseInt(text, radix)`: the leading digits in the radix, or `NaN`.
///
/// The sign comes first, then the digits the radix allows; the rest of the text is ignored.
pub fn parse_int(text: &str, radix: u32) -> f64 {
    let trimmed = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    let mut chars = trimmed.chars().peekable();
    let negative = match chars.peek() {
        Some('+') => {
            chars.next();
            false
        }
        Some('-') => {
            chars.next();
            true
        }
        _ => false,
    };
    let digits: String = chars.take_while(|c| c.is_digit(radix)).collect();
    if digits.is_empty() {
        return f64::NAN;
    }
    match u64::from_str_radix(&digits, radix) {
        Ok(value) if negative => -(value as f64),
        Ok(value) => value as f64,
        Err(_) => f64::NAN,
    }
}

/// JS `a + b`: string concatenation if either primitive is a string, numeric addition otherwise.
pub fn add(left: &Value, right: &Value) -> Value {
    fn primitive(value: &Value) -> Value {
        match value {
            Value::Object(_) if is_undefined(value) => Value::Null,
            Value::Array(_) => Value::String(js_string(value)),
            Value::Object(_) => Value::String("[object Object]".into()),
            other => other.clone(),
        }
    }
    let left = primitive(left);
    let right = primitive(right);
    if left.is_string() || right.is_string() {
        Value::String(format!("{}{}", js_string(&left), js_string(&right)))
    } else {
        number(to_number(&left) + to_number(&right))
    }
}

/// JS `ToIntegerOrInfinity(value)`: `NaN` and zero both become positive zero.
pub fn to_integer(value: &Value) -> f64 {
    let number = to_number(value);
    if number.is_nan() || number == 0.0 {
        0.0
    } else {
        number.trunc()
    }
}

/// `Number.isFinite(value)`.
pub fn number_is_finite(value: &Value) -> bool {
    matches!(value, Value::Number(n) if n.as_f64().is_some_and(f64::is_finite))
}

/// `Number.isInteger(value)`: a finite number with no fractional part.
pub fn number_is_integer(value: &Value) -> bool {
    match value {
        Value::Number(n) => n
            .as_f64()
            .is_some_and(|f| f.is_finite() && f.fract() == 0.0),
        _ => false,
    }
}

/// `Number.isSafeInteger(value)`.
pub fn number_is_safe_integer(value: f64) -> bool {
    value.is_finite() && value.fract() == 0.0 && value.abs() <= 9_007_199_254_740_991.0
}

/// JS `String(value)`.
pub fn js_string(value: &Value) -> String {
    // Marker objects revive to primitives on the JS side, so `String` must print the primitive.
    if let Some(number) = as_number(value) {
        return number_to_string(number);
    }
    if let Some(object) = value.as_object()
        && object.len() == 1
    {
        if let Some(text) = object.get("$utf16")
            && let Some(units) = text.as_array()
        {
            let units = units
                .iter()
                .filter_map(|u| u.as_u64().and_then(|u| u16::try_from(u).ok()))
                .collect::<Vec<_>>();
            return String::from_utf16_lossy(&units);
        }
        if let Some(digits) = object.get("$bigint").and_then(Value::as_str) {
            return digits.to_string();
        }
    }
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                other => js_string(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(value) {
                "undefined".into()
            } else {
                "[object Object]".into()
            }
        }
    }
}

/// `String(value || '')`: JS falsy values become the empty string.
pub fn string_or_empty(value: &Value) -> String {
    if truthy(value) {
        js_string(value)
    } else {
        String::new()
    }
}

/// `asText`: `null`/`undefined` -> `''`, booleans -> `'1'`/`'0'`, everything else `String(value)`.
/// The danmaku helpers use this instead of raw `String` so flags do not print `"true"`.
pub fn as_text(value: &Value) -> String {
    if is_nullish(value) {
        String::new()
    } else if let Value::Bool(b) = value {
        if *b { "1".into() } else { "0".into() }
    } else {
        js_string(value)
    }
}

/// JS `String.prototype.trim()`: strips whitespace and line terminators as V8 does, which
/// includes U+FEFF and the Unicode line/paragraph separators that Rust's `trim` also strips but
/// includes a few code points Rust does not. The set is the ES `WhiteSpace` + `LineTerminator`.
pub fn js_trim(text: &str) -> String {
    fn js_space(c: char) -> bool {
        matches!(
            c,
            '\u{0009}'
                | '\u{000B}'
                | '\u{000C}'
                | '\u{0020}'
                | '\u{00A0}'
                | '\u{FEFF}'
                | '\u{1680}'
                | '\u{2000}'
                ..='\u{200A}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202F}'
                    | '\u{205F}'
                    | '\u{3000}'
                    | '\u{000A}'
                    | '\u{000D}'
        )
    }
    text.trim_matches(js_space).to_string()
}

/// `value || fallback`: the raw value when it is truthy, the fallback otherwise.
pub fn or_else(value: &Value, fallback: Value) -> Value {
    if truthy(value) {
        value.clone()
    } else {
        fallback
    }
}

/// `value || 0`.
pub fn or_zero(value: &Value) -> Value {
    or_else(value, number(0.0))
}

/// `value || ''`.
pub fn or_empty_string(value: &Value) -> Value {
    or_else(value, Value::String(String::new()))
}

// ---------------------------------------------------------------------------
// Members
// ---------------------------------------------------------------------------

/// `base[key]`: the property, or `undefined` when it is not there.
pub fn prop<'a>(value: &'a Value, key: &str) -> &'a Value {
    match value {
        Value::Object(map) => map.get(key).unwrap_or_else(|| undefined_ref()),
        _ => undefined_ref(),
    }
}

/// The shared `undefined` marker: the same value every time, so `prop` can hand out a
/// reference to it.
fn undefined_ref() -> &'static Value {
    static UNDEFINED: OnceLock<Value> = OnceLock::new();
    UNDEFINED.get_or_init(undefined)
}

/// `base[key] || fallback`.
pub fn prop_or(value: &Value, key: &str, fallback: Value) -> Value {
    or_else(prop(value, key), fallback)
}

/// `value?.property`: `None` for a nullish base, the property otherwise.
pub fn optional_prop<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    if is_nullish(value) {
        None
    } else {
        Some(prop(value, key))
    }
}

/// `value?.a?.b`: the chain stops at the first nullish step.
pub fn optional_chain<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in keys {
        if is_nullish(current) {
            return None;
        }
        current = prop(current, key);
    }
    if is_nullish(current) {
        None
    } else {
        Some(current)
    }
}

/// `base[key]`: the JS member access, with the error a nullish base raises.
///
/// The key is the string form of the property: `undefined` becomes `"undefined"`, which is
/// what the JS error message names.
pub fn js_index_get<'a>(base: &'a Value, key: &str) -> Result<&'a Value, String> {
    if is_nullish(base) {
        return Err(null_property_error(key, base));
    }
    Ok(match base {
        Value::Object(map) => map.get(key).unwrap_or_else(|| undefined_ref()),
        Value::Array(items) => match key.parse::<usize>() {
            Ok(index) => items.get(index).unwrap_or_else(|| undefined_ref()),
            Err(_) => undefined_ref(),
        },
        // A string index gives a character in JS. The wire carries no characters on their own,
        // so the lookup reads `undefined` instead.
        Value::String(_) => undefined_ref(),
        _ => undefined_ref(),
    })
}

/// The error a property read raises: `null` and `undefined` say so.
pub fn null_property_error(property: &str, base: &Value) -> String {
    let kind = if is_undefined(base) {
        "undefined"
    } else {
        "null"
    };
    format!("Cannot read properties of {kind} (reading '{property}')")
}

/// The error `receiver.method` raises when the receiver has no such method.
pub fn not_a_function_error(receiver: &str, method: &str) -> String {
    format!("{receiver}.{method} is not a function")
}

/// `arr.filter(fn)`: the receiver must be an array.
pub fn filter_array(value: &Value, receiver: &str, method: &str) -> Result<Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items.clone()),
        Value::Null => Err(null_property_error(method, value)),
        Value::Object(_) if is_undefined(value) => Err(null_property_error(method, value)),
        _ => Err(not_a_function_error(receiver, method)),
    }
}

// ---------------------------------------------------------------------------
// Arrays and numbers
// ---------------------------------------------------------------------------

/// `Array.isArray(value)`.
pub fn is_array(value: &Value) -> bool {
    value.is_array()
}

/// `Array.isArray(value) ? value : []`.
pub fn array_or_empty(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items.clone(),
        _ => Vec::new(),
    }
}

/// `value.length` for an array or a string, `undefined` otherwise.
pub fn js_length(value: &Value) -> Option<f64> {
    match value {
        Value::Array(items) => Some(items.len() as f64),
        Value::String(s) => Some(s.encode_utf16().count() as f64),
        _ => None,
    }
}

/// `Math.max(a, b)` with JS's NaN propagation.
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `Math.min(a, b)` with JS's NaN propagation.
pub fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// `Math.max(...)` over a list.
pub fn js_max_all(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = values.into_iter();
    let mut max = match it.next() {
        Some(v) => v,
        None => return f64::NEG_INFINITY,
    };
    for v in it {
        max = js_max(max, v);
    }
    max
}

/// `Math.min(...)` over a list.
pub fn js_min_all(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut it = values.into_iter();
    let mut min = match it.next() {
        Some(v) => v,
        None => return f64::INFINITY,
    };
    for v in it {
        min = js_min(min, v);
    }
    min
}

/// The key `obj[item.series_id]` uses: the string form, with `undefined` as `"undefined"`.
pub fn index_key(value: &Value) -> String {
    js_string(value)
}

// ---------------------------------------------------------------------------
// Equality and spreading
// ---------------------------------------------------------------------------

/// `SameValueZero`, the comparison `Set.prototype.has` uses.
///
/// Numbers compare by value, so `1` and `1.0` are the same entry; `NaN` matches `NaN`. Objects
/// and arrays compare by their contents here, where JS compares by reference: the wire protocol
/// cannot carry references, and the ids a `Set` holds are strings and numbers in practice.
pub fn same_value_zero(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => {
            let (a, b) = (
                a.as_f64().unwrap_or(f64::NAN),
                b.as_f64().unwrap_or(f64::NAN),
            );
            a == b || (a.is_nan() && b.is_nan())
        }
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(_), Value::Array(_)) | (Value::Object(_), Value::Object(_)) => {
            if is_undefined(left) || is_undefined(right) {
                is_undefined(left) && is_undefined(right)
            } else {
                left == right
            }
        }
        _ => false,
    }
}

/// `{...value}`: the key/value pairs the JS object spread produces.
///
/// An object spreads its own enumerable keys; a string spreads its indices (`{0: 'a'}`); an
/// array spreads its indices; anything else spreads nothing.
pub fn spread_pairs(value: &Value) -> Vec<(String, Value)> {
    match value {
        Value::Object(map) => map
            .iter()
            .filter(|(key, _)| !is_undefined(value) || key.as_str() != "$undefined")
            .map(|(key, item)| (key.clone(), item.clone()))
            .collect(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item.clone()))
            .collect(),
        Value::String(text) => text
            .chars()
            .enumerate()
            .map(|(index, ch)| (index.to_string(), Value::String(ch.to_string())))
            .collect(),
        _ => Vec::new(),
    }
}

/// `{...value}` as a new map: the spread's own keys first, then the ones set afterwards.
pub fn spread_object(value: &Value) -> Map<String, Value> {
    let mut map = Map::new();
    for (key, item) in spread_pairs(value) {
        map.insert(key, item);
    }
    map
}

/// JS `encodeURIComponent`: unreserved ASCII plus `!~*'()` stay as-is; every other byte of the
/// UTF-8 encoding is percent-encoded with uppercase hex.
pub fn encode_uri_component(text: &str) -> String {
    fn keep(byte: u8) -> bool {
        byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            )
    }
    let mut out = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        if keep(byte) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", byte));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn tells_undefined_from_null() {
        assert!(is_undefined(&undefined()));
        assert!(!is_undefined(&Value::Null));
        assert!(is_nullish(&undefined()));
        assert!(is_nullish(&Value::Null));
        assert!(!is_nullish(&Value::Bool(false)));
    }

    #[test]
    fn encodes_uri_components() {
        assert_eq!(encode_uri_component("a b/c?d"), "a%20b%2Fc%3Fd");
        assert_eq!(encode_uri_component("v7.9.1"), "v7.9.1");
        assert_eq!(encode_uri_component("!~*'()"), "!~*'()");
        assert_eq!(encode_uri_component("中文"), "%E4%B8%AD%E6%96%87");
    }

    #[test]
    fn follows_js_truthiness() {
        assert!(!truthy(&undefined()));
        assert!(!truthy(&Value::Null));
        assert!(!truthy(&json!(0)));
        assert!(!truthy(&json!("")));
        assert!(!truthy(&json!(false)));
        assert!(truthy(&json!("a")));
        assert!(truthy(&json!([])));
        assert!(truthy(&json!({})));
    }

    #[test]
    fn coerces_like_js() {
        assert_eq!(to_number(&json!("12")), 12.0);
        assert_eq!(to_number(&json!("")), 0.0);
        assert!(to_number(&json!("abc")).is_nan());
        assert_eq!(to_number(&json!([5])), 5.0);
        assert_eq!(to_number(&json!([])), 0.0);
        assert!(to_number(&undefined()).is_nan());
        assert_eq!(js_string(&undefined()), "undefined");
        assert_eq!(js_string(&json!(null)), "null");
        assert_eq!(string_or_empty(&undefined()), "");
    }

    #[test]
    fn missing_property_is_undefined() {
        let value = json!({ "a": 1 });
        assert!(is_undefined(prop(&value, "b")));
        assert_eq!(prop(&value, "a"), &json!(1));
    }

    #[test]
    fn set_membership_compares_by_value() {
        assert!(same_value_zero(&json!(1), &json!(1.0)));
        assert!(!same_value_zero(&json!("1"), &json!(1)));
        assert!(same_value_zero(&json!(null), &Value::Null));
    }

    #[test]
    fn spread_keeps_the_key_order() {
        let mut map = spread_object(&json!({ "a": 1, "b": 2 }));
        map.insert("a".into(), json!(3));
        let keys: Vec<&String> = map.keys().collect();
        assert_eq!(keys, vec!["a", "b"]);
    }
}
