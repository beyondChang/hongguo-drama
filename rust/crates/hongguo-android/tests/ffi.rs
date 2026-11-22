//! Host-side verification of the JNI/C export contract (ADR-011). The production `.so` is built
//! with cargo-ndk for Android ABIs; here we exercise the same exported functions on the host.

use std::ffi::{CStr, CString};

fn call(input: &str, f: unsafe extern "C" fn(*const i8) -> *mut i8) -> serde_json::Value {
    let c_input = CString::new(input).unwrap();
    let out = unsafe { f(c_input.as_ptr()) };
    assert!(!out.is_null());
    let text = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_string();
    hongguo_android::hg_free_string(out);
    serde_json::from_str(&text).unwrap()
}

#[test]
fn window_scale_round_trip() {
    let result = call(r#"{"value":1.7}"#, hongguo_android::hg_window_scale);
    assert_eq!(result["ok"], true);
    assert_eq!(result["scale"], 1.7);
    let clamped = call(r#"{"value":9}"#, hongguo_android::hg_window_scale);
    assert_eq!(clamped["scale"], 2.0);
    let fallback = call(r#"{"value":"x"}"#, hongguo_android::hg_window_scale);
    assert_eq!(fallback["scale"], 1.0);
}

#[test]
fn release_round_trip() {
    let newer = call(
        r#"{"data":{"tag_name":"v8.0.0"},"currentVersion":"7.9"}"#,
        hongguo_android::hg_available_release,
    );
    assert_eq!(newer["ok"], true);
    assert_eq!(newer["release"]["version"], "8.0");
    let old = call(
        r#"{"data":{"tag_name":"v7.8"},"currentVersion":"7.9"}"#,
        hongguo_android::hg_available_release,
    );
    assert!(old["release"].is_null());
}

#[test]
fn null_input_is_safe() {
    let out = hongguo_android::hg_window_scale(std::ptr::null());
    assert!(!out.is_null());
    let text = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_string();
    hongguo_android::hg_free_string(out);
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["scale"], 1.0);
}
