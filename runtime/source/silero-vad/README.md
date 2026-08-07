---
pack: silero-vad
upstream_commit: 76e3dc408eb2a5c655c34e230d2d5459b4439daa
licence: MIT
fixture: silero-vad/parity-16k
---

# Silero VAD Source Roots

The Silero VAD source is checked out once into
`runtime/source/silero-vad/<target>/<arch>/src/` by the build script.
The C++/ONNX reference subset is the only part vendored; no Python or
Torch runtime is shipped.

## Targets

```text
runtime/source/silero-vad/host/auto/src/
runtime/source/silero-vad/macos-arm64/arm64/src/
runtime/source/silero-vad/linux-x86_64/x86_64/src/
runtime/source/silero-vad/windows-x86_64/x86_64/src/
```

## Not vendored

- `silero-vad/src/utils.py` (Python helper)
- any `torchaudio` integration
- the original PyTorch model checkpoint
