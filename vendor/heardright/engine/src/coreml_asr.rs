#[path = "coreml_asr_sections/window_health.rs"]
mod window_health;
pub use window_health::{
    clear_calibration_window_stats, clear_latest_asr_window_stats, install_asr_window_input,
    latest_asr_window_stats, take_calibration_window_stats, take_latest_asr_window_stats,
    AsrWindowInput, AsrWindowInputScope, AsrWindowStats, AsrWindowStatsEnvelope,
    CalibrationWindowStats, NoEmissionGap, TensorHealth, ASR_WINDOW_STATS_UPSTREAM_INPUTS_NEEDED,
};

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/coreml_asr_sections/section01.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/coreml_asr_sections/section02.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/coreml_asr_sections/decode_window.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/coreml_asr_sections/decode_topk.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/coreml_asr_sections/section03.rs"
));
