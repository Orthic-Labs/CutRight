# llama.cpp Source Roots

- Upstream: `github.com/ggerganov/llama.cpp`
- Pinned commit: `6a32c29a746a2e44de463de647f9f6661eb5086b`
- Licence: MIT
- Mode: library or supervised CutRight sidecar, network/server features
  disabled unless required for local IPC tests.

## Targets

```text
runtime/source/llama.cpp/host/auto/
runtime/source/llama.cpp/macos-arm64/arm64/
runtime/source/llama.cpp/linux-x86_64/x86_64/
runtime/source/llama.cpp/windows-x86_64/x86_64/
```

## Build contract

The runtime binary exposes:

- bounded structured generation with `token_limit` ceiling
- deterministic seed
- cancellation
- byte-bounded output buffer
- backend/accelerator identity in the handshake
- exact source commit and binary hashes in the handshake

No HTTP server. No remote model fetch. The local model bytes are
hash-pinned in the cap-ledger.
