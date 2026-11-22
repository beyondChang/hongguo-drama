//! The Rust port of the pure helpers in `src/native/window-layout.js`, reached through a
//! bridge-only named-export wrapper. JavaScript semantics live in `hongguo-js`.

#![forbid(unsafe_code)]

use hongguo_js::{
    as_number, is_undefined, js_max_all, js_min, math_round, number, prop, to_number, truthy,
};
use serde_json::{Map, Value};

fn num(value: &Value) -> f64 {
    to_number(value)
}

/// JS `value || fallback` for the optional `previous?.width`/`height` fields.
fn or_default(value: &Value, fallback: f64) -> f64 {
    if truthy(value) {
        to_number(value)
    } else {
        fallback
    }
}

/// Port of `fitWindowBounds` (`src/native/window-layout.js:2`).
pub fn fit_window_bounds(
    mode: Value,
    current: Value,
    work_area: Value,
    previous: Value,
) -> Result<Value, String> {
    let mode_text = mode.as_str().unwrap_or("");
    if mode_text != "portrait" && mode_text != "landscape" {
        return Err("Invalid window layout".into());
    }
    let portrait = mode_text == "portrait";
    let has_previous = !previous.is_null() && !is_undefined(&previous);
    let previous_height = if has_previous {
        or_default(prop(&previous, "height"), 750.0)
    } else {
        750.0
    };
    let height = js_min(
        if portrait { 940.0 } else { previous_height },
        num(prop(&work_area, "height")),
    );
    let previous_width = if has_previous {
        or_default(prop(&previous, "width"), 1100.0)
    } else {
        1100.0
    };
    let width = js_min(
        if portrait {
            js_min(620.0, math_round(height * 0.74))
        } else {
            previous_width
        },
        num(prop(&work_area, "width")),
    );
    let origin = if !portrait && has_previous {
        previous.clone()
    } else {
        let mut origin = Map::new();
        origin.insert(
            "x".into(),
            number(math_round(
                num(prop(&current, "x")) + (num(prop(&current, "width")) - width) / 2.0,
            )),
        );
        origin.insert(
            "y".into(),
            number(math_round(
                num(prop(&current, "y")) + (num(prop(&current, "height")) - height) / 2.0,
            )),
        );
        Value::Object(origin)
    };
    let area_x = num(prop(&work_area, "x"));
    let area_y = num(prop(&work_area, "y"));
    let mut out = Map::new();
    out.insert(
        "x".into(),
        number(js_max_all([
            area_x,
            js_min(
                num(prop(&origin, "x")),
                area_x + num(prop(&work_area, "width")) - width,
            ),
        ])),
    );
    out.insert(
        "y".into(),
        number(js_max_all([
            area_y,
            js_min(
                num(prop(&origin, "y")),
                area_y + num(prop(&work_area, "height")) - height,
            ),
        ])),
    );
    out.insert("width".into(), number(width));
    out.insert("height".into(), number(height));
    Ok(Value::Object(out))
}

/// Port of `getWindowScale` (`src/native/window-layout.js:17`).
pub fn get_window_scale(value: Value) -> Result<Value, String> {
    let scale = match as_number(&value) {
        Some(n) if n.is_finite() => js_min(2.0, hongguo_js::js_max(0.5, n)),
        _ => 1.0,
    };
    Ok(number(scale))
}
