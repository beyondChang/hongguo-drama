//! The Rust port of `hardwareEncoderArgs` from `src/native/hardware-encoder.js`, reached through
//! a bridge-only wrapper. The async child-process probe belongs to the L4 desktop shell.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// Port of `hardwareEncoderArgs` (`src/native/hardware-encoder.js:8`).
pub fn build_encoder_args(encoder: Value) -> Result<Value, String> {
    let name = encoder.as_str().unwrap_or("");
    let args: Vec<&str> = match name {
        "h264_videotoolbox" => vec![
            "-c:v",
            "h264_videotoolbox",
            "-allow_sw",
            "0",
            "-realtime",
            "0",
            "-q:v",
            "68",
            "-profile:v",
            "high",
            "-pix_fmt",
            "yuv420p",
            "-threads:v",
            "2",
        ],
        "h264_nvenc" => vec![
            "-c:v",
            "h264_nvenc",
            "-preset",
            "p4",
            "-rc",
            "vbr",
            "-cq",
            "21",
            "-b:v",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-threads:v",
            "2",
        ],
        "h264_amf" => vec![
            "-c:v",
            "h264_amf",
            "-quality",
            "speed",
            "-rc",
            "cqp",
            "-qp_i",
            "26",
            "-qp_p",
            "26",
            "-pix_fmt",
            "yuv420p",
            "-threads:v",
            "2",
        ],
        _ => return Err("不支持的硬件编码器".into()),
    };
    Ok(json!(args))
}
