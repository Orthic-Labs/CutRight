# Voice Asset Ledger (CR-V2-B3-016)

## Source

- Upstream: `github.com/hexgrad/kokoro`
- Pinned version: v1.0
- Pinned model hash: `496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe6e47ee1f2c53f18ad1e4`
- Licence: Apache-2.0

## Voice file audit

```text
for each voice in available_voices:
    assert voice.license_resolved
    assert voice.licence in {"apache-2.0", "cc-by-4.0", "cc0-1.0"}
    assert voice.sha256 in ledger
    copy voice to runtime/models/voice/<target>/<arch>/voices/
```

A voice with unresolved licence is excluded from the pack. The build
aborts if the resulting pack has zero voices.

## Phonemizer data

The selected native ONNX runtime bundles all required phonemizer data.
No Python or espeak system dependency is permitted.

## Build steps

1. Inspect the ledger for the pinned model hash.
2. Copy model and tokenizer to `runtime/models/voice/<target>/<arch>/`.
3. Copy each audited voice file to the voices subdirectory.
4. Update `runtime/manifests/voice.model.json` with the resolved
   files[] array.
5. Run the pronunciation, determinism, duration, clipping, silence and
   cross-platform waveform fixtures.

## Acceptance

- The exact model hash matches.
- No unresolved voice is copied.
- TTS works with network blocked and empty PATH.
