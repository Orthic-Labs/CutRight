# Contributing to CutRight

## The gate is the contract

There is no CI service. `scripts/gate.sh` is the single authoritative gate, and
it must pass locally before every commit:

```bash
bash scripts/gate.sh
```

It runs, failing fast: root cargo `fmt --check` / `clippy -D warnings` / `test`,
then the same three for the Studio cargo workspace (a deliberately separate
workspace so Tauri's dependency graph and lockfile stay isolated), then the
Studio frontend `install` / `typecheck` / `test` / `build`, then the
license/asset resolution scan. `--with-qa` additionally runs the headless
browser QA lane.

Toolchains are pinned: `rust-toolchain.toml`, `.node-version`, and the
`packageManager` field in `apps/studio/package.json`. Do not work around a pin —
change it deliberately, in its own commit.

## Rules that are not negotiable

**Sources are never modified.** Ingest registers source media immutably by
BLAKE3. No command may write to, move, or re-encode a registered source.

**Canonical artifacts are schema-valid.** Every artifact under a fixed schema
version validates against its file in `schemas/` before it is written. Changing
an artifact shape means: bump the schema version, add valid and invalid
fixtures for the new version, add a migration from every supported prior
version, and add the Rust round-trip test. Silent shape drift is a defect —
`source_word_id` reached the Rust model before the schema described it, and the
schema's permissiveness hid it.

**No untyped FFmpeg invocation.** Every external process — FFmpeg, FFprobe,
HeardRight, WhisperX, sidecar workers — goes through the shared process runner:
explicit environment allow-list, timeout with kill-tree, bounded stdout/stderr,
structured exit status, temp cleanup. No command may wait indefinitely.

**A JSON `status: "error"` never accompanies exit zero.** See the exit-code
table in `crates/video-cli/src/main.rs`.

**Artifacts are variant-scoped — and it's enforced, not just documented.** Cut
plan, timeline, transcript, captions, reframe plan, finish plan, final, QA and
export all reference the same variant and the same artifact hashes. Never let
"last command run" decide the contents of a shared file. Variant-scoped reads
go through `require_variant_artifact`
(`crates/video-project/src/io/variant.rs`) and error rather than silently
substituting another variant's artifact; the old `variant_or_generic` fallback
and every generic-alias write it enabled are gone from the codebase.

**Review decisions are constructed by Rust, not the frontend.** Studio sends a
minimal `DecisionIntent`; the backend derives the canonical subject, hashes,
project identity, benchmark state and app version, and returns the persisted
record. A decision is bound to the exact bytes it reviewed.

**HeardRight is the local transcript authority; WhisperX verifies it.** Never
switch the product transcript engine because the verifier produced one cleaner
sample. CutRight does not reach into HeardRight's model internals.

## How to add a provider capability

1. Add the capability name and its request/result payloads to the protocol,
   additively — existing clients must keep working.
2. Negotiate it in the handshake; reject on protocol-major mismatch and
   negotiate minor versions.
3. Return engine/model/protocol identity so it can land in provenance.
4. Add fake-engine tests: correlation, malformed JSON, timeout, early EOF,
   restart.
5. Add a `doctor` probe for it under the right profile.

## How to add a project artifact receipt

Every one of the 14 canonical pipeline stages emits a
`<artifact>.receipt.json` (`crates/video-project/src/receipts.rs`) binding
`stage`, `implementation_version`, input paths with hashes, a
`parameters_blake3`, toolchain identity, and output paths with hashes and
sizes; variants additionally get a per-variant `artifact-receipt.json`. Write
it in the same atomic operation as the artifact. `videoctl receipts verify`
re-hashes every recorded binding against the bytes currently on disk and
exits 6 on the first mismatch — run it after touching receipt-writing code.
Cache keys are content-addressed — source hash, decode policy, toolchain
identity, provider/model/protocol identity, stage implementation version —
never machine-local absolute paths; sidecar workers materialize under
`video_core::content_store::materialize_worker` at a path keyed by the
content hash of their embedded bytes, not by version.

## How to update Studio contracts

The Rust command types are the source of truth. Regenerate or hand-update the
matching TypeScript contract in `apps/studio/src/contracts/`, keep the
round-trip fixture test passing, and make sure the QA mock exercises the real
boundary rather than replacing it — a mock that bypasses Rust cannot catch
deserialization, path, reason, hash or persistence failures.

## Phase gate and evidence

A phase closes on demonstrated behavior in a packaged build, not on the
existence of commands and UI surfaces. Claims land with evidence: the gate
receipt, the artifact opened and inspected, the failing case reproduced. Update
`STATUS.md` from the gate and release process in the same change that alters
what it describes.
