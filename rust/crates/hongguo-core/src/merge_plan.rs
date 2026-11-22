//! Merge-plan domain logic shared by the desktop download pipeline and the three shells.

use hongguo_js::*;
use serde_json::{Map, Value, json};

/// The fields that make two video streams incompatible for stream copying.
const VIDEO_FIELDS: [&str; 15] = [
    "codec_name",
    "codec_tag_string",
    "profile",
    "level",
    "width",
    "height",
    "pix_fmt",
    "sample_aspect_ratio",
    "r_frame_rate",
    "extradata_hash",
    "color_range",
    "color_space",
    "color_transfer",
    "color_primaries",
    "field_order",
];
const AUDIO_FIELDS: [&str; 6] = [
    "codec_name",
    "profile",
    "sample_rate",
    "channels",
    "channel_layout",
    "extradata_hash",
];

/// `JSON.stringify` writes `null` for `undefined` inside an array.
pub fn in_array(value: &Value) -> Value {
    if is_undefined(value) {
        Value::Null
    } else {
        value.clone()
    }
}

/// `signature(stream, fields)`: the decoder configuration, not per-file statistics.
pub fn signature(stream: Option<&Value>, fields: &[&str]) -> String {
    let Some(stream) = stream else {
        return String::new();
    };
    let parts = fields
        .iter()
        .map(|key| match stream.get(*key) {
            Some(value) if !is_undefined(value) && !value.is_null() => value.clone(),
            _ => Value::Null,
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&Value::Array(parts)).unwrap_or_default()
}

pub fn as_array<'a>(value: &'a Value, receiver: &str) -> Result<&'a Vec<Value>, String> {
    match value {
        Value::Array(items) => Ok(items),
        Value::Null | Value::Object(_) if is_nullish(value) => {
            Err(null_property_error("find", value))
        }
        _ => Err(not_a_function_error(receiver, "find")),
    }
}

/// Port of `readMergeInfo` (`src/native/merge-plan.js:5`).
pub fn read_merge_info(data: Value) -> Result<Value, String> {
    let streams_value = js_index_get(&data, "streams")?;
    let streams = if is_nullish(streams_value) {
        Vec::new()
    } else {
        as_array(streams_value, "data.streams")?.clone()
    };
    let is_type = |stream: &Value, kind: &str| -> Result<bool, String> {
        Ok(js_index_get(stream, "codec_type")? == &Value::String(kind.into()))
    };
    let video = streams
        .iter()
        .find(|s| is_type(s, "video").unwrap_or(false));
    let Some(video) = video else {
        return Ok(Value::Null);
    };
    let audio = streams
        .iter()
        .find(|s| is_type(s, "audio").unwrap_or(false));

    let duration =
        number_or_zero(optional_chain(&data, &["format", "duration"]).unwrap_or(&Value::Null));
    let format_bitrate = {
        let declared =
            to_number(optional_chain(&data, &["format", "bit_rate"]).unwrap_or(&Value::Null));
        if declared != 0.0 && !declared.is_nan() {
            declared
        } else {
            let size =
                to_number(optional_chain(&data, &["format", "size"]).unwrap_or(&Value::Null));
            let from_size = size * 8.0 / duration;
            if from_size == 0.0 || from_size.is_nan() {
                0.0
            } else {
                from_size
            }
        }
    };
    let video_signature = signature(Some(video), &VIDEO_FIELDS);
    let audio_signature = signature(audio, &AUDIO_FIELDS);
    let combined = json!([
        in_array(&Value::String(video_signature.clone())),
        in_array(js_index_get(video, "time_base")?),
        in_array(&Value::String(audio_signature.clone())),
        in_array(audio.map_or(&Value::Null, |a| {
            a.get("time_base").unwrap_or(&Value::Null)
        })),
    ]);
    let video_bitrate = {
        let declared = to_number(js_index_get(video, "bit_rate")?);
        if declared != 0.0 && !declared.is_nan() {
            declared
        } else {
            let audio_rate = audio
                .map(|a| to_number(a.get("bit_rate").unwrap_or(&Value::Null)))
                .unwrap_or(f64::NAN);
            let audio_rate = if audio_rate == 0.0 || audio_rate.is_nan() {
                if audio.is_some() { 96_000.0 } else { 0.0 }
            } else {
                audio_rate
            };
            js_max(0.0, format_bitrate - audio_rate)
        }
    };
    let entry = |key: &str| -> Value { video.get(key).cloned().unwrap_or_else(undefined) };
    Ok(json!({
        "codec": entry("codec_name"),
        "w": entry("width"),
        "h": entry("height"),
        "audio": audio.is_some(),
        "duration": number(duration),
        "video": video.clone(),
        "audioStream": audio.cloned().unwrap_or(Value::Null),
        "videoSignature": video_signature,
        "audioSignature": audio_signature,
        "signature": serde_json::to_string(&combined).unwrap_or_default(),
        "videoBitrate": number(video_bitrate),
    }))
}

pub fn audio_stream(info: &Value) -> &Value {
    prop(info, "audioStream")
}

/// Port of `buildMergePlan` (`src/native/merge-plan.js:17`).
pub fn build_merge_plan(infos: Value, options: Value) -> Result<Value, String> {
    let options = if is_undefined(&options) {
        Value::Object(Map::new())
    } else if options.is_null() {
        return Err(null_property_error("compatible", &options));
    } else {
        options
    };
    let compatible = truthy(prop(&options, "compatible"));
    let items = match &infos {
        Value::Array(items) => items,
        Value::Null => return Err(null_property_error("length", &infos)),
        _ => return Err("分集缺少有效时长或画面尺寸".into()),
    };
    let positive =
        |info: &Value, key: &str| -> bool { !is_nullish(info) && to_number(prop(info, key)) > 0.0 };
    if items.is_empty()
        || items
            .iter()
            .any(|info| !positive(info, "duration") || !positive(info, "w") || !positive(info, "h"))
    {
        return Err("分集缺少有效时长或画面尺寸".into());
    }
    let first = &items[0];
    let codec = prop(first, "codec");
    let first_codec = codec.as_str().unwrap_or_default();
    let total_value = items.iter().fold(Value::from(0), |sum, info| {
        add(&sum, prop(info, "duration"))
    });
    let total_duration = to_number(&total_value);
    let known = first_codec == "h264" || first_codec == "hevc";
    let same_codec = known && items.iter().all(|info| prop(info, "codec") == codec);
    let video_mode = if same_codec && !(compatible && first_codec != "h264") {
        "copy"
    } else {
        "encode"
    };
    let dynamic_video = items
        .iter()
        .any(|info| prop(info, "videoSignature") != prop(first, "videoSignature"));
    let video_container = if video_mode == "copy" && first_codec == "hevc" && dynamic_video {
        "mpegts"
    } else {
        "mp4"
    };
    let video_tag = if video_container == "mpegts" {
        Value::String("hev1".into())
    } else if video_mode == "encode" {
        if dynamic_video {
            Value::String("avc3".into())
        } else {
            Value::Null
        }
    } else if first_codec == "h264" && dynamic_video {
        Value::String("avc3".into())
    } else {
        Value::Null
    };
    let has_audio = items.iter().any(|info| truthy(prop(info, "audio")));
    let audio_mode = if !has_audio {
        "none"
    } else if video_mode == "encode"
        || items.iter().any(|info| {
            !truthy(prop(info, "audio"))
                || !truthy(
                    js_index_get(prop(info, "audioStream"), "extradata_hash")
                        .unwrap_or(&Value::Null),
                )
                || prop(info, "audioSignature") != prop(first, "audioSignature")
        })
        || (compatible && {
            let name = js_index_get(audio_stream(first), "codec_name").unwrap_or(&Value::Null);
            name != &Value::String("aac".into())
        })
    {
        "encode"
    } else {
        "copy"
    };
    let remux = items.iter().any(|info| {
        let video_time = optional_chain(prop(info, "video"), &["time_base"]);
        let first_video_time = optional_chain(prop(first, "video"), &["time_base"]);
        let audio_time = optional_chain(audio_stream(info), &["time_base"]);
        let first_audio_time = optional_chain(audio_stream(first), &["time_base"]);
        video_time != first_video_time || audio_time != first_audio_time
    });
    let rate_value = js_index_get(prop(first, "video"), "r_frame_rate").unwrap_or(&Value::Null);
    let rate_text = js_string(rate_value);
    let rate_text = if truthy(&Value::String(rate_text.clone())) {
        rate_text
    } else {
        "30/1".into()
    };
    let mut parts = rate_text.split('/').map(string_to_number);
    let numerator = parts.next().unwrap_or(f64::NAN);
    let denominator = parts.next().unwrap_or(1.0);
    let raw_fps = numerator / denominator;
    let fps = js_min(
        60.0,
        js_max(
            1.0,
            if raw_fps == 0.0 || raw_fps.is_nan() {
                30.0
            } else {
                raw_fps
            },
        ),
    );
    let source_rate: f64 = items
        .iter()
        .map(|info| {
            let w = to_number(prop(info, "w"));
            let h = to_number(prop(info, "h"));
            let declared = to_number(prop(info, "videoBitrate"));
            let rate = if declared != 0.0 && !declared.is_nan() {
                declared
            } else {
                w * h * fps * 0.04
            };
            let factor = if prop(info, "codec") == &Value::String("hevc".into()) {
                1.8
            } else {
                1.25
            };
            rate * factor * to_number(prop(info, "duration"))
        })
        .sum::<f64>()
        / total_duration;
    let video_bitrate =
        math_round(js_min(20_000_000.0, js_max(300_000.0, source_rate)) / 1000.0) * 1000.0;
    let audio_bitrate = if has_audio { 96_000.0 } else { 0.0 };
    Ok(json!({
        "videoMode": video_mode,
        "videoContainer": video_container,
        "videoTag": video_tag,
        "audioMode": audio_mode,
        "remux": remux,
        "fps": number(fps),
        "videoBitrate": number(video_bitrate),
        "audioBitrate": audio_bitrate,
        "totalDuration": total_value,
        "estimatedBytes": number(total_duration * (video_bitrate + audio_bitrate) / 8.0),
    }))
}

/// Port of `canUseHardwareFrames` (`src/native/merge-plan.js:38`).
pub fn can_use_hardware_frames(infos: Value, encoder: Value) -> Result<Value, String> {
    if encoder != Value::String("h264_videotoolbox".into()) {
        return Ok(Value::Bool(false));
    }
    let items = match &infos {
        Value::Array(items) => items,
        Value::Null => return Err(null_property_error("every", &infos)),
        _ => return Err(not_a_function_error("infos", "every")),
    };
    let usable = items.iter().all(|info| {
        let codec = prop(info, "codec");
        let known =
            codec == &Value::String("h264".into()) || codec == &Value::String("hevc".into());
        let pix_fmt = js_index_get(prop(info, "video"), "pix_fmt").unwrap_or(&Value::Null);
        let aspect =
            js_index_get(prop(info, "video"), "sample_aspect_ratio").unwrap_or(&Value::Null);
        known
            && pix_fmt == &Value::String("yuv420p".into())
            && (!truthy(aspect) || aspect == &Value::String("1:1".into()))
    });
    Ok(Value::Bool(usable))
}
