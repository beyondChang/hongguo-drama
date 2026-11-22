//! JS bindings for `src/native/_wrapper-danmaku-pure.js`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
pub mod utf16;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `normalizeColor`.
#[napi(js_name = "normalizeColor")]
pub fn normalize_color(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_color(value)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeMode`.
#[napi(js_name = "normalizeMode")]
pub fn normalize_mode(holders: String) -> napi::Result<String> {
    run(move || {
        let holders = crate::js::from_json(&holders)?;
        let out = port::normalize_mode(holders)?;
        crate::js::to_json(&out)
    })
}

/// `cleanControl`.
#[napi(js_name = "cleanControl")]
pub fn clean_control(text: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::clean_control(text)?;
        crate::js::to_json(&out)
    })
}

/// `toSeconds`.
#[napi(js_name = "toSeconds")]
pub fn to_seconds(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::to_seconds(value)?;
        crate::js::to_json(&out)
    })
}

/// `parsePage`.
#[napi(js_name = "parsePage")]
pub fn parse_page(payload: String, vid: String) -> napi::Result<String> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let vid = crate::js::from_json(&vid)?;
        let out = port::parse_page(payload, vid)?;
        crate::js::to_json(&out)
    })
}

/// `buildBody`.
#[napi(js_name = "buildBody")]
pub fn build_body(
    vid: String,
    book_id: String,
    start: String,
    duration: String,
) -> napi::Result<String> {
    run(move || {
        let vid = crate::js::from_json(&vid)?;
        let book_id = crate::js::from_json(&book_id)?;
        let start = crate::js::from_json(&start)?;
        let duration = crate::js::from_json(&duration)?;
        let out = port::build_body(vid, book_id, start, duration)?;
        crate::js::to_json(&out)
    })
}

// ---- the binary wire ------------------------------------------------------------------------

type WireBuffers = (
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint32Array,
);

thread_local! {
    static WIRE: std::cell::RefCell<Option<WireBuffers>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static SCRATCH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers the argument, result and argument-length buffers `index.cjs` uses.
#[napi(js_name = "rustAnythingWireInit")]
pub fn rust_anything_wire_init(
    input: napi::bindgen_prelude::Uint8Array,
    output: napi::bindgen_prelude::Uint8Array,
    len: napi::bindgen_prelude::Uint32Array,
) {
    WIRE.with(|w| *w.borrow_mut() = Some((input, output, len)));
}

/// Copies a result that did not fit into the (now larger) result buffer.
#[napi(js_name = "rustAnythingWireFlush")]
pub fn rust_anything_wire_flush() -> f64 {
    let bytes = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    WIRE.with(|w| {
        let mut w = w.borrow_mut();
        let Some((_, output, _)) = w.as_mut() else {
            return -1.0;
        };
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        n as f64
    })
}

/// Reads the argument bytes (`wLen[0]` of them), runs `f`, and writes its result; a negative
/// return is the size of a result that did not fit, kept for `rustAnythingWireFlush`.
fn wire_io(
    f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>,
) -> napi::Result<f64> {
    WIRE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some((input, output, len)) = guard.as_mut() else {
            return Err(napi::Error::from_reason(
                "rust-anything: the wire buffers are not registered",
            ));
        };
        let len = (len.first().copied().unwrap_or(0) as usize).min(input.len());
        let mut w = wire::Writer(SCRATCH.with(|s| std::mem::take(&mut *s.borrow_mut())));
        w.0.clear();
        let result = run(|| f(&mut wire::Reader::new(&input[..len]), &mut w));
        let keep = |w: wire::Writer| SCRATCH.with(|s| *s.borrow_mut() = w.0);
        match result {
            // A result of another shape than the TS type: JS runs the call through JSON.
            Err(e) if e.reason.starts_with(wire::MISMATCH) => {
                keep(w);
                return Ok(f64::NAN);
            }
            Err(e) => {
                keep(w);
                let mut m = wire::Writer::default();
                m.string16(&e.reason);
                // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
                let out = unsafe { output.as_mut() };
                if m.0.len() > out.len() {
                    return Err(e);
                }
                out[..m.0.len()].copy_from_slice(&m.0);
                return Ok(f64::INFINITY);
            }
            Ok(()) => {}
        }
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = w.0.len();
        if n > out.len() {
            PENDING.with(|p| *p.borrow_mut() = w.0);
            return Ok(-(n as f64));
        }
        out[..n].copy_from_slice(&w.0);
        keep(w);
        Ok(n as f64)
    })
}

static S_NORMALIZE_COLOR_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_COLOR_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeColor` over the binary wire.
#[napi(js_name = "normalizeColorWire")]
pub fn normalize_color_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_COLOR_0)?;
        let out = &port::normalize_color(value)?;
        wire::write_value(w, out, &S_NORMALIZE_COLOR_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_MODE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_MODE_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeMode` over the binary wire.
#[napi(js_name = "normalizeModeWire")]
pub fn normalize_mode_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let holders: Value = wire::read_value(r, &S_NORMALIZE_MODE_0)?;
        let out = &port::normalize_mode(holders)?;
        wire::write_value(w, out, &S_NORMALIZE_MODE_OUT)?;
        Ok(())
    })
}

static S_CLEAN_CONTROL_0: wire::Schema = wire::Schema::Dyn;
static S_CLEAN_CONTROL_OUT: wire::Schema = wire::Schema::Dyn;

/// `cleanControl` over the binary wire.
#[napi(js_name = "cleanControlWire")]
pub fn clean_control_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_CLEAN_CONTROL_0)?;
        let out = &port::clean_control(text)?;
        wire::write_value(w, out, &S_CLEAN_CONTROL_OUT)?;
        Ok(())
    })
}

static S_TO_SECONDS_0: wire::Schema = wire::Schema::Dyn;
static S_TO_SECONDS_OUT: wire::Schema = wire::Schema::Dyn;

/// `toSeconds` over the binary wire.
#[napi(js_name = "toSecondsWire")]
pub fn to_seconds_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_TO_SECONDS_0)?;
        let out = &port::to_seconds(value)?;
        wire::write_value(w, out, &S_TO_SECONDS_OUT)?;
        Ok(())
    })
}

static S_PARSE_PAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_PAGE_1: wire::Schema = wire::Schema::Dyn;
static S_PARSE_PAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parsePage` over the binary wire.
#[napi(js_name = "parsePageWire")]
pub fn parse_page_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: Value = wire::read_value(r, &S_PARSE_PAGE_0)?;
        let vid: Value = wire::read_value(r, &S_PARSE_PAGE_1)?;
        let out = &port::parse_page(payload, vid)?;
        wire::write_value(w, out, &S_PARSE_PAGE_OUT)?;
        Ok(())
    })
}

static S_BUILD_BODY_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_BODY_1: wire::Schema = wire::Schema::Dyn;
static S_BUILD_BODY_2: wire::Schema = wire::Schema::Dyn;
static S_BUILD_BODY_3: wire::Schema = wire::Schema::Dyn;
static S_BUILD_BODY_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildBody` over the binary wire.
#[napi(js_name = "buildBodyWire")]
pub fn build_body_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let vid: Value = wire::read_value(r, &S_BUILD_BODY_0)?;
        let book_id: Value = wire::read_value(r, &S_BUILD_BODY_1)?;
        let start: Value = wire::read_value(r, &S_BUILD_BODY_2)?;
        let duration: Value = wire::read_value(r, &S_BUILD_BODY_3)?;
        let out = &port::build_body(vid, book_id, start, duration)?;
        wire::write_value(w, out, &S_BUILD_BODY_OUT)?;
        Ok(())
    })
}
