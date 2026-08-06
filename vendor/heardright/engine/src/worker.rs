#[cfg(test)]
#[path = "worker_sections/replay_contract.rs"]
mod replay_contract;
#[cfg(test)]
#[path = "worker_sections/replay_effect_sink.rs"]
mod replay_effect_sink;
#[cfg(test)]
#[path = "worker_sections/wav_replay_source.rs"]
mod wav_replay_source;
#[path = "worker_sections/worker_clock.rs"]
mod worker_clock;

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/worker_sections/section01.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/worker_sections/unified_probe_lane.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/worker_sections/section02.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/worker_sections/section03.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/worker_sections/section04.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/worker_sections/section05.rs"
));
