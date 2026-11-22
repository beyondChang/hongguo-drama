//! The Rust port of `src/native/merge-plan.js`.
//!
//! The domain logic lives in `hongguo-core::merge_plan`, shared with the three shells;
//! JavaScript semantics live in `hongguo-js`. After changing a signature, run
//! `rust-anything refresh` to regenerate the bindings.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `readMergeInfo` (`src/native/merge-plan.js:5`).
pub fn read_merge_info(data: Value) -> Result<Value, String> {
    hongguo_core::merge_plan::read_merge_info(data)
}

/// Port of `buildMergePlan` (`src/native/merge-plan.js:17`).
pub fn build_merge_plan(infos: Value, options: Value) -> Result<Value, String> {
    hongguo_core::merge_plan::build_merge_plan(infos, options)
}

/// Port of `canUseHardwareFrames` (`src/native/merge-plan.js:38`).
pub fn can_use_hardware_frames(infos: Value, encoder: Value) -> Result<Value, String> {
    hongguo_core::merge_plan::can_use_hardware_frames(infos, encoder)
}
