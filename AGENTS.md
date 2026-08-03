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
- Bound plans with low/likely/high ETA plus file, byte, line, & active-second ceilings.

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
- Use Sentinel assess through close for architecture, over two changed files, non-obvious debugging, repeated failures, or signoff.
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
