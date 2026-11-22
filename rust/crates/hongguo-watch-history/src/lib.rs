//! JS bindings for `src/native/watch-history.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `listHistory`.
#[napi(js_name = "listHistory")]
pub fn list_history(
    series: String,
    playback: String,
    show_sequels: String,
) -> napi::Result<String> {
    run(move || {
        let series = crate::js::from_json(&series)?;
        let playback = crate::js::from_json(&playback)?;
        let show_sequels = crate::js::from_json(&show_sequels)?;
        let out = port::list_history(series, playback, show_sequels)?;
        crate::js::to_json(&out)
    })
}

/// `recordPlayback`.
#[napi(js_name = "recordPlayback")]
pub fn record_playback(
    previous: String,
    series: String,
    index: String,
    time: String,
    now: String,
    metadata: String,
) -> napi::Result<String> {
    run(move || {
        let previous = crate::js::from_json(&previous)?;
        let series = crate::js::from_json(&series)?;
        let index = crate::js::from_json(&index)?;
        let time = crate::js::from_json(&time)?;
        let now = crate::js::from_json(&now)?;
        let metadata = crate::js::from_json(&metadata)?;
        let out = port::record_playback(previous, series, index, time, now, metadata)?;
        crate::js::to_json(&out)
    })
}

/// `applyEpisodeUpdate`.
#[napi(js_name = "applyEpisodeUpdate")]
pub fn apply_episode_update(
    record: String,
    old_series: String,
    fresh_series: String,
    now: String,
) -> napi::Result<String> {
    run(move || {
        let record = crate::js::from_json(&record)?;
        let old_series = crate::js::from_json(&old_series)?;
        let fresh_series = crate::js::from_json(&fresh_series)?;
        let now = crate::js::from_json(&now)?;
        let out = port::apply_episode_update(record, old_series, fresh_series, now)?;
        crate::js::to_json(&out)
    })
}

/// `seasonTitle`.
#[napi(js_name = "seasonTitle")]
pub fn season_title(title: String) -> napi::Result<String> {
    run(move || {
        let title = crate::js::from_json(&title)?;
        let out = port::season_title(title)?;
        crate::js::to_json(&out)
    })
}

/// `sourceOf`.
#[napi(js_name = "sourceOf")]
pub fn source_of(id: String) -> napi::Result<String> {
    run(move || {
        let id = crate::js::from_json(&id)?;
        let out = port::source_of(id)?;
        crate::js::to_json(&out)
    })
}

/// `familyKey`.
#[napi(js_name = "familyKey")]
pub fn family_key(series: String) -> napi::Result<String> {
    run(move || {
        let series = crate::js::from_json(&series)?;
        let out = port::family_key(series)?;
        crate::js::to_json(&out)
    })
}

/// `seasonOf`.
#[napi(js_name = "seasonOf")]
pub fn season_of(series: String) -> napi::Result<String> {
    run(move || {
        let series = crate::js::from_json(&series)?;
        let out = port::season_of(series)?;
        crate::js::to_json(&out)
    })
}

/// `findSequels`.
#[napi(js_name = "findSequels")]
pub fn find_sequels(
    series: String,
    candidates: String,
    after_season: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let series = crate::js::from_json(&series)?;
        let candidates = crate::js::from_json(&candidates)?;
        let out = port::find_sequels(series, candidates, after_season)?;
        crate::js::to_json(&out)
    })
}

/// `sequelKeyword`.
#[napi(js_name = "sequelKeyword")]
pub fn sequel_keyword(series: String) -> napi::Result<String> {
    run(move || {
        let series = crate::js::from_json(&series)?;
        let out = port::sequel_keyword(series)?;
        crate::js::to_json(&out)
    })
}

/// `findSimilarSeries`.
#[napi(js_name = "findSimilarSeries")]
pub fn find_similar_series(series: String, candidates: String) -> napi::Result<String> {
    run(move || {
        let series = crate::js::from_json(&series)?;
        let candidates = crate::js::from_json(&candidates)?;
        let out = port::find_similar_series(series, candidates)?;
        crate::js::to_json(&out)
    })
}

/// `acknowledge`.
#[napi(js_name = "acknowledge")]
pub fn acknowledge(record: String, snapshot: String) -> napi::Result<String> {
    run(move || {
        let record = crate::js::from_json(&record)?;
        let snapshot = crate::js::from_json(&snapshot)?;
        let out = port::acknowledge(record, snapshot)?;
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

static S_LIST_HISTORY_0: wire::Schema = wire::Schema::Dyn;
static S_LIST_HISTORY_1: wire::Schema = wire::Schema::Dyn;
static S_LIST_HISTORY_2: wire::Schema = wire::Schema::Dyn;
static S_LIST_HISTORY_OUT: wire::Schema = wire::Schema::Dyn;

/// `listHistory` over the binary wire.
#[napi(js_name = "listHistoryWire")]
pub fn list_history_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let series: Value = wire::read_value(r, &S_LIST_HISTORY_0)?;
        let playback: Value = wire::read_value(r, &S_LIST_HISTORY_1)?;
        let show_sequels: Value = wire::read_value(r, &S_LIST_HISTORY_2)?;
        let out = &port::list_history(series, playback, show_sequels)?;
        wire::write_value(w, out, &S_LIST_HISTORY_OUT)?;
        Ok(())
    })
}

static S_RECORD_PLAYBACK_0: wire::Schema = wire::Schema::Dyn;
static S_RECORD_PLAYBACK_1: wire::Schema = wire::Schema::Dyn;
static S_RECORD_PLAYBACK_2: wire::Schema = wire::Schema::Dyn;
static S_RECORD_PLAYBACK_3: wire::Schema = wire::Schema::Dyn;
static S_RECORD_PLAYBACK_4: wire::Schema = wire::Schema::Dyn;
static S_RECORD_PLAYBACK_5: wire::Schema = wire::Schema::Dyn;
static S_RECORD_PLAYBACK_OUT: wire::Schema = wire::Schema::Dyn;

/// `recordPlayback` over the binary wire.
#[napi(js_name = "recordPlaybackWire")]
pub fn record_playback_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let previous: Value = wire::read_value(r, &S_RECORD_PLAYBACK_0)?;
        let series: Value = wire::read_value(r, &S_RECORD_PLAYBACK_1)?;
        let index: Value = wire::read_value(r, &S_RECORD_PLAYBACK_2)?;
        let time: Value = wire::read_value(r, &S_RECORD_PLAYBACK_3)?;
        let now: Value = wire::read_value(r, &S_RECORD_PLAYBACK_4)?;
        let metadata: Value = wire::read_value(r, &S_RECORD_PLAYBACK_5)?;
        let out = &port::record_playback(previous, series, index, time, now, metadata)?;
        wire::write_value(w, out, &S_RECORD_PLAYBACK_OUT)?;
        Ok(())
    })
}

static S_APPLY_EPISODE_UPDATE_0: wire::Schema = wire::Schema::Dyn;
static S_APPLY_EPISODE_UPDATE_1: wire::Schema = wire::Schema::Dyn;
static S_APPLY_EPISODE_UPDATE_2: wire::Schema = wire::Schema::Dyn;
static S_APPLY_EPISODE_UPDATE_3: wire::Schema = wire::Schema::Dyn;
static S_APPLY_EPISODE_UPDATE_OUT: wire::Schema = wire::Schema::Dyn;

/// `applyEpisodeUpdate` over the binary wire.
#[napi(js_name = "applyEpisodeUpdateWire")]
pub fn apply_episode_update_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let record: Value = wire::read_value(r, &S_APPLY_EPISODE_UPDATE_0)?;
        let old_series: Value = wire::read_value(r, &S_APPLY_EPISODE_UPDATE_1)?;
        let fresh_series: Value = wire::read_value(r, &S_APPLY_EPISODE_UPDATE_2)?;
        let now: Value = wire::read_value(r, &S_APPLY_EPISODE_UPDATE_3)?;
        let out = &port::apply_episode_update(record, old_series, fresh_series, now)?;
        wire::write_value(w, out, &S_APPLY_EPISODE_UPDATE_OUT)?;
        Ok(())
    })
}

static S_SEASON_TITLE_0: wire::Schema = wire::Schema::Dyn;
static S_SEASON_TITLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `seasonTitle` over the binary wire.
#[napi(js_name = "seasonTitleWire")]
pub fn season_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let title: Value = wire::read_value(r, &S_SEASON_TITLE_0)?;
        let out = &port::season_title(title)?;
        wire::write_value(w, out, &S_SEASON_TITLE_OUT)?;
        Ok(())
    })
}

static S_SOURCE_OF_0: wire::Schema = wire::Schema::Dyn;
static S_SOURCE_OF_OUT: wire::Schema = wire::Schema::Dyn;

/// `sourceOf` over the binary wire.
#[napi(js_name = "sourceOfWire")]
pub fn source_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: Value = wire::read_value(r, &S_SOURCE_OF_0)?;
        let out = &port::source_of(id)?;
        wire::write_value(w, out, &S_SOURCE_OF_OUT)?;
        Ok(())
    })
}

static S_FAMILY_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_FAMILY_KEY_OUT: wire::Schema = wire::Schema::Dyn;

/// `familyKey` over the binary wire.
#[napi(js_name = "familyKeyWire")]
pub fn family_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let series: Value = wire::read_value(r, &S_FAMILY_KEY_0)?;
        let out = &port::family_key(series)?;
        wire::write_value(w, out, &S_FAMILY_KEY_OUT)?;
        Ok(())
    })
}

static S_SEASON_OF_0: wire::Schema = wire::Schema::Dyn;
static S_SEASON_OF_OUT: wire::Schema = wire::Schema::Dyn;

/// `seasonOf` over the binary wire.
#[napi(js_name = "seasonOfWire")]
pub fn season_of_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let series: Value = wire::read_value(r, &S_SEASON_OF_0)?;
        let out = &port::season_of(series)?;
        wire::write_value(w, out, &S_SEASON_OF_OUT)?;
        Ok(())
    })
}

static S_FIND_SEQUELS_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_SEQUELS_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_SEQUELS_OUT: wire::Schema = wire::Schema::Dyn;

/// `findSequels` over the binary wire.
#[napi(js_name = "findSequelsWire")]
pub fn find_sequels_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let series: Value = wire::read_value(r, &S_FIND_SEQUELS_0)?;
        let candidates: Value = wire::read_value(r, &S_FIND_SEQUELS_1)?;
        let after_season: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::find_sequels(series, candidates, after_season)?;
        wire::write_value(w, out, &S_FIND_SEQUELS_OUT)?;
        Ok(())
    })
}

static S_SEQUEL_KEYWORD_0: wire::Schema = wire::Schema::Dyn;
static S_SEQUEL_KEYWORD_OUT: wire::Schema = wire::Schema::Dyn;

/// `sequelKeyword` over the binary wire.
#[napi(js_name = "sequelKeywordWire")]
pub fn sequel_keyword_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let series: Value = wire::read_value(r, &S_SEQUEL_KEYWORD_0)?;
        let out = &port::sequel_keyword(series)?;
        wire::write_value(w, out, &S_SEQUEL_KEYWORD_OUT)?;
        Ok(())
    })
}

static S_FIND_SIMILAR_SERIES_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_SIMILAR_SERIES_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_SIMILAR_SERIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `findSimilarSeries` over the binary wire.
#[napi(js_name = "findSimilarSeriesWire")]
pub fn find_similar_series_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let series: Value = wire::read_value(r, &S_FIND_SIMILAR_SERIES_0)?;
        let candidates: Value = wire::read_value(r, &S_FIND_SIMILAR_SERIES_1)?;
        let out = &port::find_similar_series(series, candidates)?;
        wire::write_value(w, out, &S_FIND_SIMILAR_SERIES_OUT)?;
        Ok(())
    })
}

static S_ACKNOWLEDGE_0: wire::Schema = wire::Schema::Dyn;
static S_ACKNOWLEDGE_1: wire::Schema = wire::Schema::Dyn;
static S_ACKNOWLEDGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `acknowledge` over the binary wire.
#[napi(js_name = "acknowledgeWire")]
pub fn acknowledge_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let record: Value = wire::read_value(r, &S_ACKNOWLEDGE_0)?;
        let snapshot: Value = wire::read_value(r, &S_ACKNOWLEDGE_1)?;
        let out = &port::acknowledge(record, snapshot)?;
        wire::write_value(w, out, &S_ACKNOWLEDGE_OUT)?;
        Ok(())
    })
}
