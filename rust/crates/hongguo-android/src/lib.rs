//! Rust core for the Android/TV WebView shell (ADR-005/ADR-011).
//!
//! The production bridge is JNI: `MainActivity` loads this cdylib as `.so` and calls
//! `Java_*` wrappers that pass UTF-8 strings through `JNIEnv`. Those wrappers are a thin UTF-8 /
//! `jstring` conversion over the C ABI entry points here, so the same exported functions can be
//! exercised on the host with plain C strings. Large payloads (CENC decrypt/merge) will use direct
//! byte buffers in a later wave; these first entries carry JSON so they can be tested verbatim.

// FFI boundary: the JNI/C shims need `unsafe` to read caller-provided C strings. The pure logic
// they call lives in `forbid(unsafe_code)` crates; only the pointer conversion is unsafe here.
#![allow(unsafe_op_in_unsafe_fn)]
// Exported extern "C" functions must be safe fns; the pointer validity is guaranteed by the JNI/C
// caller contract (documented with SAFETY on each function), so clippy's warning does not apply.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use std::ffi::{CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};

use serde_json::Value;

/// Every entry returns a freshly allocated NUL-terminated JSON string. The caller frees it with
/// `hg_free_string`. On a panic the result is a JSON error object rather than unwinding across FFI.
fn json_result<F>(work: F) -> *mut c_char
where
    F: FnOnce() -> Result<Value, String>,
{
    let value = match catch_unwind(AssertUnwindSafe(work)) {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => serde_json::json!({ "ok": false, "error": error }),
        Err(_) => serde_json::json!({ "ok": false, "error": "Rust panic" }),
    };
    let text = value.to_string();
    CString::new(text)
        .unwrap_or_else(|_| CString::new("{\"ok\":false,\"error\":\"nul in result\"}").unwrap())
        .into_raw()
}

/// Read a NUL-terminated UTF-8 input; a null pointer is treated as JSON `null`.
/// SAFETY: `input` must be a valid NUL-terminated C string or null (guaranteed by the JNI/C caller).
unsafe fn input_value(input: *const c_char) -> Value {
    if input.is_null() {
        return Value::Null;
    }
    // SAFETY: caller guarantees a NUL-terminated string.
    let text = unsafe { std::ffi::CStr::from_ptr(input) }.to_string_lossy();
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

/// Free a string returned by an `hg_*` entry point.
///
/// SAFETY: `value` must be a pointer previously returned by this library's `hg_*` functions.
#[unsafe(no_mangle)]
pub extern "C" fn hg_free_string(value: *mut c_char) {
    if !value.is_null() {
        // SAFETY: ownership came from `CString::into_raw` in `json_result`.
        drop(unsafe { CString::from_raw(value) });
    }
}

/// `Native.windowScale(value)` -> `{ ok, scale }`, backed by `hongguo-window-layout`.
///
/// SAFETY: `input` must be a valid NUL-terminated JSON C string or null.
#[unsafe(no_mangle)]
pub extern "C" fn hg_window_scale(input: *const c_char) -> *mut c_char {
    json_result(|| {
        // SAFETY: see function contract.
        let value = unsafe { input_value(input) };
        let args = value.get("value").cloned().unwrap_or(value);
        let scale = hongguo_window_layout::port::get_window_scale(args)?;
        Ok(serde_json::json!({ "ok": true, "scale": scale }))
    })
}

/// `Native.availableRelease(data, currentVersion)` -> `{ ok, release }`, backed by
/// `hongguo-app-update`.
///
/// SAFETY: `input` must be a valid NUL-terminated JSON C string or null.
#[unsafe(no_mangle)]
pub extern "C" fn hg_available_release(input: *const c_char) -> *mut c_char {
    json_result(|| {
        // SAFETY: see function contract.
        let value = unsafe { input_value(input) };
        let data = value.get("data").cloned().unwrap_or(Value::Null);
        let current = value.get("currentVersion").cloned().unwrap_or(Value::Null);
        let release = hongguo_app_update::port::available_release(data, current)?;
        Ok(serde_json::json!({ "ok": true, "release": release }))
    })
}

/// A version marker so the Java side can verify the loaded `.so` matches the shipped core.
#[unsafe(no_mangle)]
pub extern "C" fn hg_core_version() -> *const c_char {
    c"rust-core-0.1.0".as_ptr()
}

// Keep the JNI environment pointer type nameable for the production wrappers (unused on host).
#[allow(dead_code)]
type JniEnv = *mut c_void;
