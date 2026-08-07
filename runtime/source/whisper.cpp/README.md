# whisper.cpp Source Roots

- Upstream: `github.com/ggerganov/whisper.cpp`
- Pinned commit: `306c88f4d1286aec1bf96e544632897886af5501`
- Licence: MIT
- Verifier role only: the binary is exposed as a *quality verifier*; it
  never promotes its output to canonical transcript authority.

## Targets

```text
runtime/source/whisper.cpp/host/auto/
runtime/source/whisper.cpp/macos-arm64/arm64/
runtime/source/whisper.cpp/linux-x86_64/x86_64/
runtime/source/whisper.cpp/windows-x86_64/x86_64/
```

## Verifier contract

```text
pub struct VerificationResult {
    pub coverage: f32,
    pub unmatched_content_rate: f32,
    pub boundary_deltas: Distribution,
    pub decision: VerificationDecision,
}
```

The verifier reports whether the canonical transcript is *consistent*
with its own decoding. A mismatch is preserved as evidence, not as a
destructive overwrite.
