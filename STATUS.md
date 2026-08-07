# CutRight — as-built status

Generated from the gate and release process. Update it in the same change that
alters what it describes; do not edit it from memory.

```yaml
as_of_commit: b918b07                     # + the monochrome theme pass this file ships with
current_stage: product_phases_landed     # §5-§14 closed; §15 phases 4,5,6,7,8,9 all landed
                                           # (phase 8 built provider-agnostic: no vendor, no keys)
primary_audio_engine: HeardRight
primary_asr: Parakeet TDT v3
word_edge_verifier: WhisperX
cloud_default: disabled
ci: none                                  # scripts/gate.sh is the contract
quality_gate: CLEAN                       # build/lint/types/clippy ran, 0 findings — this is
                                           # ONE gate, not the audit verdict; see below
audit_status: pass                        # six scanners now installed and wired into the gate;
                                           # hadolint is not-applicable (no Dockerfile)
last_full_gate_run: 2026-08-02 — `bash scripts/gate.sh`, one invocation, GATE PASS (17 steps)
known_blockers:
  - register_pick_pending                 # three monochrome registers (graphite/tungsten/pewter)
                                           # are implemented and functional but NOT yet screenshotted
                                           # or picked. brands.md still has no CutRight entry — the
                                           # lock happens only after Adrian picks. See
                                           # apps/studio/design/REDESIGN-SPEC.md.
  - effects_library_5_of_15               # finish.md:108 names 15 starter effects; the registry
                                           # has 5. Remotion/ASS/HyperFrames renderer split is done.
  - libass_absent_locally                 # this machine's ffmpeg has no libass (and no drawtext),
                                           # so the ASS caption renderer fails loudly here. doctor
                                           # reports it honestly rather than degrading silently.
  - candidate_generation_not_editorial    # rough-cut candidates still group words by gap with
                                           # best-take scoring rather than red-thread editorial
                                           # selection. NOTE: §15.4 shorts extraction IS now
                                           # semantic — this is the separate rough-cut path.
  - cloud_provider_unchosen               # §15.6 IS built — full envelope, Disabled + Fake
                                           # adapters only. Enabling a real one (Gemini or
                                           # Twelve Labs per ARCHITECTURE-2026-07-26.md:42) needs
                                           # a provider decision, accepted terms and a budget.
                                           # Nothing can send until then: consent off, budget 0.
  - four_preference_axes_unlearnable      # §15.7 pause policy, effect density, SFX choices and
                                           # hook/CTA structure report unsupported_by_ledger_
                                           # schema: the DecisionRecord reason vocabulary cannot
                                           # distinguish them. Extending that vocabulary is the
                                           # prerequisite, not more inference.
```

## Audit posture (read before trusting anything above the fold)

**`quality_gate: CLEAN` is one gate. It is NOT "the audit passed."** Those are
different predicates and one word cannot carry both. `bash scripts/gate.sh` ran
as a single invocation on 2026-08-02 and exited zero:

| Gate step | Command | Result |
|---|---|---|
| root fmt | `cargo fmt --all -- --check` | PASS |
| root clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS — 0 warnings |
| root test | `cargo test --workspace --locked` | PASS — 215 tests |
| Studio fmt | `cargo fmt --manifest-path apps/studio/src-tauri/Cargo.toml -- --check` | PASS |
| Studio clippy | `cargo clippy --manifest-path apps/studio/src-tauri/Cargo.toml --all-targets --locked -- -D warnings` | PASS — 0 warnings |
| Studio test | `cargo test --manifest-path apps/studio/src-tauri/Cargo.toml --locked` | PASS — 32 tests |
| Studio frontend typecheck | `pnpm typecheck` (`tsc --noEmit`) | PASS |
| Studio frontend test | `pnpm test` (`vitest run`) | PASS — 67 tests |
| Studio frontend build | `pnpm build` | PASS |
| Effects package | `pnpm --dir apps/effects` install/typecheck/test/build | PASS — 10 tests |
| License/asset resolution | `bash scripts/resolve-license.sh` | PASS — 24 assets, all noted |
| Supply chain | `cargo deny check` (deny.toml) | PASS — advisories, bans, licenses, sources |
| Unused deps | `cargo machete` | PASS — zero |

Total across the suites: **324 tests** (215 root + 32 Studio backend +
67 Studio frontend + 10 effects), all passing.

**The scanner gates are no longer unproven.** They were, for the whole
hardening campaign: six of the seven tools the plan names were simply not
installed, so duplication, dead-export, unused-dep, license-graph and
unsafe-density could not certify anything. All six are now installed
(`jscpd`, `knip`, `license-checker`, `cargo-deny`, `cargo-machete`,
`cargo-geiger`); `hadolint` is genuinely not-applicable because this repo has
no Dockerfile. `cargo-deny` and `cargo-machete` are wired into
`scripts/gate.sh` behind a tool-presence check, so if one goes missing later
the gate prints SKIP with its install command rather than passing silently.

What they found and what was done: two orphaned `video-media` dependencies
removed after verifying zero references; `deny.toml` written because an
unconfigured cargo-deny rejects every license including MIT, which is a
misconfigured tool rather than a finding; `publish = false` declared on the
internal crates, which is both accurate and what lets the path-dependency
wildcard exemption apply. `jscpd` reports 1.46% duplication across 122 files
and `knip` four unused exported types that are deliberate contract mirrors of
the Rust API — both left alone, on the record.

Reasoning-lens coverage on 2026-08-02: eight lenses ran (architecture,
correctness, security/data-safety/release-readiness, minimize, doc-drift/
citation-integrity, ai-slop/naming, resilience/platform-parity, a11y/
performance). All 15 decomposition candidates were assessed — seven confirmed
and split, eight `not-needed`, **zero `undetermined`**. Every confirmed finding
from that pass is fixed; the open items are the unproven scanner gates above
and the product-phase work below.

## What is closed

**Review contract (§5).** Studio sends a minimal `DecisionIntent`; Rust builds
the authoritative record — canonical non-injectable subject, artifact hash,
project and benchmark provenance, app version — appends it under a cross-process
lock in one buffered write, and returns exactly what was persisted. Replay
preserves stale and missing records instead of silently dropping them, and
reports malformed lines. Reason vocabularies are target-specific; verdict
controls are mode-gated; finals are per preset; QA acknowledgement binds the
report hash.

**Variant scoping (§6).** A hash-bound selection record gates final rendering;
`render final`, `reframe plan`, `finish validate`, `qa` and `export otio`
resolve the selected variant rather than a hard-coded `natural.mp4`. Gap
compaction implements the tight/natural pause policy. Cut-plan validation
accepts reordered non-overlapping source intervals. The timebase is explicit
and rational. Legacy layouts migrate with backups.

**Benchmark (§8).** HeardRight is the transcript authority in every branch.
Status is one of `primary_accepted`, `verifier_edges_required`,
`manual_review_required`, `verifier_unavailable`; two clean providers accept
HeardRight instead of deadlocking, and only `manual_review_required` blocks
destructive word-edge automation. The zero-unmatched-words requirement is gone,
replaced by coverage, unmatched-content rate and delta distributions, with
thresholds in `schemas/benchmark-policy.v1.json`. Reports bind source,
transcript, envelope and engine identity hashes.

**Schema (§8.4/§8.5).** `source_word_id` is described, unknown fields are
rejected, and semantic validation covers ordering, uniqueness, source
references and timebase rationality, with valid and invalid fixtures per
version.

**Local audio boundary (§9).** One HeardRight session serves transcription and
VAD behind a handshake, with unique request and trace ids, hard-fail response
correlation, per-request timeouts, bounded stderr, exactly one controlled
restart, and no model-directory knowledge or network fallback. VAD provenance
persists to `analysis/vad-<source>.provenance.json`.

**Process safety (§10.1/§10.6/§10.8).** Every spawn goes through one runner:
environment allow-list, timeout, kill-tree, byte caps, cancellation, temp
cleanup, telemetry. Atomic writes use collision-proof temp names. A JSON
`status: "error"` can no longer exit zero — see the exit-code table in
`crates/video-cli/src/main.rs`.

**Diagnostics (§11).** `doctor --profile core|audio|render|studio|all`, with
`--strict` and `--write-receipt`. Probes are active: real temp-dir lifecycle,
real tiny encodes for h264_videotoolbox/libx264/AAC/zscale re-probed by the
paired ffprobe. Checks this crate cannot honestly verify report `missing` with
remediation rather than a fabricated `ok`.

**Studio integrity (§12).** Asset grants are exact files, not a recursive grant
of the project tree, and source grants require a regular file and a successful
media probe. Relink only rewrites the manifest on a BLAKE3 match and records
every attempt. `ArtifactState` distinguishes missing from corrupt from stale.
`project_revision` supplements `generated_at`. Project identity is random and
immutable — identically named projects no longer collide, and existing projects
gain an instance id on migration without losing their original id.

**Per-deliverable QA and preset captions (§13).** `qa` writes one report per
preset — `qa/<preset>.report.json` (`crates/video-project/src/qa.rs`), each
bound to its final's hash via a matching
`render/finals/<preset>.provenance.json` — and refuses a QA run built against
a mixed-variant artifact graph. Every run also refreshes `qa/summary.json`
from whatever `qa/*.report.json` files currently exist on disk, not just the
preset just run. `package social` writes `exports/package-manifest.json`
with content hashes and RFC-3986-escaped OTIO file URLs. Evidence is built
from the selected timeline's actual cut boundaries, and
`analysis/transcript-packed.md` now covers every registered source, not just
the first.

**Stage receipts, content-addressed caching, sidecar and toolchain identity
(§10.2–§10.5).** `crates/video-project/src/receipts.rs` writes a
`<artifact>.receipt.json` for each of the 14 canonical pipeline stages plus a
per-variant `artifact-receipt.json`; `videoctl receipts verify` re-hashes
every recorded input/output against the bytes on disk and exits 6 on any
binding that no longer holds (`crates/video-cli/src/main.rs`). Every
FFmpeg/FFprobe spawn goes through one `process_runner`, now living in
`video-core` (`crates/video-core/src/process_runner.rs`) rather than
duplicated per crate, with a duration-scaled timeout, an environment
allow-list, output byte caps, and kill-tree cleanup; one resolved
`MediaToolchain` (`crates/video-media/src/toolchain.rs`) is threaded through
every render/probe path instead of `ffprobe` being resolved independently of
`ffmpeg`. Embedded sidecar workers materialize by content hash via
`video_core::content_store::materialize_worker`
(`crates/video-core/src/content_store.rs`) — same bytes, same path; changed
bytes, new path — rather than being keyed by version. The transcription
cache's identity dropped the absolute source path
(`crates/video-project/src/transcription.rs`,
`transcription_cache_identity_survives_a_moved_source`).

**Variant contamination fixed (§6, follow-up).** `variant_or_generic` and
every generic-alias artifact write are gone from the codebase (grep confirms
zero remaining references). Variant-scoped reads now go through
`require_variant_artifact` (`crates/video-project/src/io/variant.rs`, used
from `final_render.rs`, `export.rs`, `reframe.rs`, `timeline.rs`,
`evidence.rs`, `qa.rs`, `finish.rs`) and error rather than silently
substituting another variant's artifact.

**HeardRight/WhisperX discovery (§9, follow-up).** WhisperX discovery no
longer hardcodes a machine-local path — no `/Volumes/D/...` (or any other
absolute-path) literal remains in `crates/` or `apps/`.

**Studio hardening — capabilities, CSP, fonts, accessibility.** Tauri ships
an explicit capability file
(`apps/studio/src-tauri/capabilities/default.json`) scoped to
`core:default`, `dialog:allow-open` and the two `window-state` permissions —
previously an empty/implicit grant. `tauri.conf.json`'s CSP `style-src` is
`'self'` only; `'unsafe-inline'` is gone. Bundled font licenses
(Tanker/Geist/Spline Sans Mono) are documented in
`apps/studio/src/assets/fonts/LICENSES.md` and shipped at
`apps/studio/public/LICENSES.md`, linked from `index.html` so the notice
travels with the built app. In the frontend: `useFocusTrap.ts` traps focus in
all three dialogs, `Transcript.tsx` carries `aria-current` and an
`aria-live` region, the waveform in `SourcesMode.tsx` is keyboard-operable,
`usePlayback.ts` no longer commits React state on every rAF tick, and
`useWindowedChunks.ts` windows both the transcript and source-word lists.

**§14 decomposition.** `video-project` split from a small number of large
files into per-concern modules (`analysis.rs`, `benchmark.rs` +
`benchmark/`, `candidates.rs`, `cut_plan.rs`, `evidence.rs`, `export.rs`,
`final_render.rs`, `finish.rs`, `ingest.rs`, `io.rs` + `io/{atomic_io,srt,
variant}.rs`, `package.rs`, `project_init.rs`, `qa.rs`, `qa_probes.rs`,
`receipts.rs`, `reframe.rs`, `remap.rs`, `rough_render.rs`, `shorts.rs`,
`snapshot.rs`, `timeline.rs`, `transcription.rs` — 23 top-level files plus
the two submodule directories); `video-media` into 10 files
(`audio.rs`, `captions.rs`, `evidence.rs`, `final_render.rs`, `probe.rs`,
`process.rs`, `reframe.rs`, `rough_render.rs`, `toolchain.rs`,
`waveform.rs`); `video-cli`'s command surface into `cli.rs`; the Studio
backend into `commands.rs`, `decision_contract.rs`, `decision_store.rs`,
`artifact_state.rs`, `project_identity.rs`, `project_scope.rs`,
`source_integrity.rs`, `relink_history.rs`, `tests.rs`, plus `main.rs`; and
the Studio frontend into `App.tsx` with `hooks/`, `modes/` and `components/`
directories.

## What is open

**The register pick is the one thing waiting on a person.** Three monochrome
registers — graphite (bone accent), tungsten (warm bone), pewter (white) — are
implemented as token themes over one structure and are functional (typecheck,
67 tests, build all pass), but they have NOT been screenshotted or chosen. A
saturated chromatic accent was rejected on the grounds that it competes with
the footage, which is the actual content of a review tool; state is carried by
luminance, weight and position instead, with one desaturated red-orange
reserved for rejected/blocked/failed. `brands.md` still has no CutRight entry
and must not get one until Adrian picks. Spec:
`apps/studio/design/REDESIGN-SPEC.md`.

**Cloud has no provider, by design.** §15.6 is built — consent, hard budget,
upload policy, content-hash cache, retention/delete, outage fallback, dedupe —
with only `Disabled` and `Fake` adapters. Consent defaults off and the budget
defaults to zero, so nothing can leave the machine. Enabling Gemini or Twelve
Labs (named in `ARCHITECTURE-2026-07-26.md:42`) needs a provider decision,
accepted terms and a budget; the trait already models both a one-shot and an
index-then-query lifecycle so that is an adapter, not a redesign.

**Three honest limits inside what landed**, each stated in code rather than
papered over:
- Four preference axes (pause policy, effect density, SFX choices, hook/CTA
  structure) report `unsupported_by_ledger_schema`. The `DecisionRecord`
  reason vocabulary cannot distinguish them, so extending that vocabulary is
  the prerequisite — more inference over the same data would be invention.
- Active-speaker attribution in reframing uses transcript timing plus a
  nearest-face continuity heuristic. There is no lip-sync or voice-print
  model in this repo.
- This machine's FFmpeg is built without `libass` and without `drawtext`, so
  the ASS caption renderer fails loudly here and `doctor` reports it missing.
  Remotion covers the branded-kinetic effects with real type; ASS would cover
  cheap fixed karaoke captions on a machine whose FFmpeg has libass.

**Rough-cut candidate generation is still gap-based** with best-take scoring,
not red-thread editorial selection. §15.4 made SHORTS extraction semantic;
this is the separate rough-cut path and the distinction matters. Reframing,
by contrast, IS temporal now (§15.5) — sampled tracks with bounded-acceleration
smoothing, not one anchor per segment.

**The effect library is 5 of 15.** `finish.md:108` names fifteen starter
effects. The renderer split (ffmpeg / ass / remotion / hyperframes) is done and
Remotion is real and pinned; ten effects remain unbuilt, and HyperFrames has no
implementation anywhere in the repo — it is a reserved enum variant that fails
loudly, per `finish.md:69`.
