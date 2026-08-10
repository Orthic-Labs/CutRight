# CutRight driver-host adaptation

Task: `CR-F-B6-001`.

This crate imports the bounded driver-host contract shape from the pinned
CodeRight driver-host source at `/Volumes/D/claude/coderight/engine/crates/driver_host`.
Only provider discovery, executable attestation, launch descriptions, denied
environment names, stdio ownership, Claude stream-json parsing, Codex app-server
JSON-RPC parsing, normalized events, replay cursors, and fail-closed parsing were
retained. CodeRight delegation, coding tools, repository mutation, memory,
browser, web, IDE/iOS, native conductor, daemon routes, UI, bundled runtimes,
and local models are intentionally absent.

The implementation is source-copied into CutRight and has no Cargo dependency,
build path, IPC call, or runtime lookup pointing at CodeRight. CutRight policy,
leases, receipts, and provider containment are applied by later tasks.

Provenance: source directory inspected 2026-08-09; imported source is not a
runtime dependency. Parity gates: `cargo test -p video-driver-host`, workspace
crate-DAG validation, `cargo clippy --workspace --all-targets -- -D warnings`,
and `cargo fmt --all --check`.
