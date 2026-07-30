// The bundled CutRight Silero VAD worker was removed: HeardRight is now the
// single local-audio service boundary and owns Silero model discovery, runtime
// loading, and inference (see `src/lib.rs::HeardRightProvider::analyze_file_vad`
// and the `audio_vad` module). This build script no longer compiles anything.
// It is retained only as a no-op so the crate keeps building until the file is
// deleted outright (hardening plan §9.4 step 6).
fn main() {}
