# Workspace Rules

## Authority & conduct
- Execute Adrian's explicit reversible, in-scope request.
- Ask only for missing private input, new spend, unrequested publication or production mutation, destruction, or a reserved decision.
- Finish requested work or report one hard blocker with exact missing input.
- Use primary checkout & current branch; create no branch or worktree without Adrian.
- Preserve unrelated user changes.
- Lead with outcome, keep replies brief, & omit forced closing filler.
- Never fabricate quotes, statistics, testimonials, stories, or evidence.
- Open real visual artifacts for Adrian's approval.
- Bound every plan with one total-minutes number plus file & line ceilings, & show the inputs — files, lines changed, code rate; never a low/high range & never from feel; see `/tasklist` for the enforced format.
- Name every non-typing minute (inspect, compile, test, deploy, report); no overhead, buffer, or contingency bucket, & named parts sum to each step's span.
- Write each step as an elapsed-clock span from minute 0 (`0–2`, `3–20`), parallelizing independent work so lanes overlap on that one clock.
- Treat every ceiling as a stop-loss: on breach stop & report, never pad, revise silently, or bill external wait; score plan versus actual symmetrically, & record any variance past ±10%.
- Never force-close a bounded subagent; report its estimated remaining time instead.

## Bootstrap & toolchains
- After clone, pull, or a missing command, run `python3 tools/setup-workspace.py` on Mac or `py -3.11 tools\setup-workspace.py` on Windows, then `workspace-doctor`.
- Install no workspace toolchain ad hoc.
- Let nearest `packageManager`, `engines`, `rust-toolchain.toml`, or repository venv override workspace defaults.
- Default to Node 26.5.x, pnpm 11.18.0, `python3` on Mac, & `py -3.11` on Windows.
- Use pnpm in pnpm repositories & run package CLIs through `pnpm exec`, never npm or npx.
- Run Rust through repository toolchain or `rustup stable`.
- Launch no visible Windows console for background automation.

## Mandatory systems
- Use Crypt shims for durable memory; treat runtime storage as truth & Markdown as export.
- Honor Membrane packets & report typed degradation without overstating enforcement.
- Use Forge assess through close for architecture, non-obvious debugging, repeated failures, or signoff; locked-domain paths make it mandatory on evidence.
- Let rhook enforce Brief, Minimize, model caps, & safety guards; debug gates instead of bypassing them.
- Run `tools/pipelines/hooks/status.py` for unhealthy context or hooks.
- Run matching thread guard before substantial work; at CRITICAL, start a fresh task unless Adrian directs continuation after seeing its result.

## Access
- Read `docs/rules/README.md` plus matching runbook before remote, credentialed, or paid work.
- Use `ssh vendure-auto` for agent access to Hetzner.
- Use `win "<command>"` from Mac & `ssh mac "<command>"` from Windows.
- Read `docs/rules/github-access.md` before GitHub writes or pushes.
- Read `docs/rules/cloudflare-access.md` before Cloudflare, R2, Worker, DNS, or Pages work.
- Read `docs/rules/paid-compute.md` before metered compute.
- Never print or inspect credentials to discover configuration.

## Right Suite releases
- Use RightKit `right-release` from primary checkout with manifest-pinned pnpm.
- Select explicit `patch` or `update`; keep build or seal separate from upload.
- Read release, signing, distribution, & licensing runbooks before release work.
- Publish only an exact build named by Adrian's current request; upload no test artifact.

## Scope & completion
- Read repository overlay before editing a nested repository.
- Load `/brand <code>` before brand or content work.
- Keep product facts, procedures, incidents, credential topology, & current state outside this core.
- Add rules only after repeated failure; use one imperative plus one pointer.
- Use one instruction per bullet, one stable term per concept, & active voice.
- Run focused checks first, then verification proportional to blast radius.
- Require concrete behavior or artifact evidence before completion.

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
- Keep HeardRight responsible for ASR models and discovery.
- Preserve original media and make project retries idempotent.

## Verification
- Run focused crate tests before the full workspace suite.
- Build waveform, boundary-frame, container, caption, and duration evidence for final renders.
- Require receipt verification before calling a package approved.

## v2 standalone boundary
- Ship runtime only through signed CutRight packs; release code resolves no bare executables, sibling-repository paths, or release environment overrides, enforced by `python3 scripts/gates/v2-runtime-boundary.py --check`.
- Add no hosted CI; local `scripts/gate.sh` is the only gate, and `bash scripts/gates/v2-repository-shape.sh` enforces the no-CI, no-submodule, no-symlink repository shape.
