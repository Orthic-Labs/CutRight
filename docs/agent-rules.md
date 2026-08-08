# CutRight Rules

## Purpose
CutRight is a local headless video-editing pipeline with Rust-owned project state and typed media boundaries.
Treat verified media evidence and human framing approval as release gates.

## Canonical sources
- Read `README.md` for pipeline and gate behavior.
- Read `docs/architecture.md` for components and flows.
- Read `STATUS.md` for current implementation state.
- Read the relevant evidence and project schema docs before changing media contracts.

## Commands
- Run `cargo fmt --all --check`.
- Run `cargo clippy --workspace --all-targets -- -D warnings`.
- Run `cargo test --workspace`.
- Run `cargo run -p videoctl -- doctor` before media work.
- Use `videoctl --dry-run` before any effectful project command.

## Locked invariants
- Register immutable source media with BLAKE3 and reverify it before use.
- Route every probe and render through typed FFprobe and FFmpeg boundaries.
- Keep timestamp arithmetic and canonical project JSON in Rust.
- Require independent transcription evidence before destructive word-edge cuts.
- Require approved reframe plan and approved anchors before vertical delivery.
- Build the speech engine from `vendor/heardright` and resolve models only from signed CutRight packs.
- Preserve original media and make project retries idempotent.

## Verification
- Run focused crate tests before the full workspace suite.
- Build waveform, boundary-frame, container, caption, and duration evidence for final renders.
- Require receipt verification before calling a package approved.
