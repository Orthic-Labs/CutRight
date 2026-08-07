# CR-V2-B3-025 — Network-blocked clean-path runtime smoke

This document freezes the clean-path runtime smoke test for Book 3 task
`CR-V2-B3-025`.

## Required shape

```text
env -i HOME="$TMP/home" PATH="" CUTRIGHT_PACK_ROOT="$TMP/packs" ./cutright-clean-runtime-harness
```

## Procedure

1. Launch the harness with a temporary HOME, an empty PATH, blocked
   outbound network and only the staged application/packs.
2. Probe media, transcribe, run VAD, run verifier, load Director, load
   critic, synthesise TTS, build basic scene/face evidence and execute
   a tiny cached job twice.
3. Assert the second run uses the verified cache and no component
   attempts repair or download.
4. Capture process, network and file evidence.

## Components probed

| Component | Required | Notes |
|---|---|---|
| `media.probe` | yes | FFmpeg/FFprobe from the staged media pack |
| `speech.transcribe` | yes | HeardRight / WhisperX from the staged speech pack |
| `speech.vad` | yes | Silero VAD via the staged speech pack |
| `evidence.verify` | yes | Evidence graph + receipt verification |
| `studio.director` | yes | Director planner |
| `studio.critic` | yes | Critic scorer |
| `tts.synthesize` | yes | TTS voices from the staged media pack |
| `scene.evidence` | yes | Basic scene evidence |
| `face.evidence` | yes | Basic face evidence |
| `job.cached` | yes | A tiny cached job run twice |

## Acceptance

- Every required component succeeds.
- Network attempt count is zero.
- The second run shows expected cache hits and identical hashes.

## Fixtures

- `fixtures/runtime/clean-smoke/fixture.json` — declared expectations.
- `fixtures/runtime/clean-smoke/packs/{media,speech,tracker}/MANIFEST` —
  staged pack files for the harness.

## Commands

```bash
bash scripts/qa/v2-clean-runtime.sh
cargo test --workspace --locked clean_runtime
```
