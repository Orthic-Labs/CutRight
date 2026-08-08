# CutRight — v2 as-built status

Generated from the v2 dispatch gate process and frozen source-of-truth
files (`release/v2/VALIDATION.json`, `release/v2/CHECKLIST.csv`,
`docs/dispatch/v2/book-*/`). Update it in the same change that alters
what it describes; do not edit it from memory.

This file replaces the v1-era `STATUS.md` content (which referenced
pre-v2 §5–§14 architecture and §15 phases). v1 narrative is preserved
in git history at the parent of `fix(status): rewrite STATUS.md as
v2-anchored as-built status`.

```yaml
as_of_commit: 7777df7da2a537dd6cd4fc5c9b02a2cb73d99bf4
dispatch_package: CutRight-v2 standalone implementation dispatch (frozen 2026-08-06)
dispatch_status: pass
task_count: 189
task_status: 189/189 done (1 multi-commit split, 1 orchestrator variance logged)
post_execution_fixes: 3 (video-agent registration, video-jobs DAG, video-agent MCP IPv6)
audit_cleanup_fixes: 8 (this audit round)
head: 7777df7da2a537dd6cd4fc5c9b02a2cb73d99bf4
quality_gate: source readiness checks pass; full build gate deferred to build phase
clean_machine_proof: harness implemented; fresh-user qualification deferred to build phase
ci: none                                          # scripts/gate.sh is the contract
known_blockers:
  - fresh_os_user_proof_pending                    # requires build-phase execution on a fresh OS user
  - signed_target_qualification_pending            # requires build, seal, and target qualification
```

## v2 architecture — what shipped

CutRight is now one installable offline product with the five systems
named in `CutRight-v2-Product-Architecture.md`:

1. **Media Kernel** — Rust owns canonical project state, stable IDs,
   rational time, source hashes, revisions, timeline transactions,
   validation, undo, rendering, migrations, receipts, and package
   integrity. Implemented across `crates/video-{actions,capabilities,cli,
   core,project,security,state,services}`.

2. **Evidence and Job Plane** — Hierarchical evidence graph
   (`crates/video-evidence`, `schemas/evidence/`) and content-addressed
   job DAG (`crates/video-jobs`, `schemas/jobs/`). Both planes share a
   stage-fingerprint, resource-budget, and resume/cancel contract.

3. **Embedded Creative Operating System** — Product-local skill runtime
   in `crates/video-sessions` with brand, brand-identity, designer,
   writing, social, planning, asset-validation, native-renderer,
   native-typography, native-motion, native-audio, render-graph,
   creative-critic, and job-plane surfaces (14 lanes). Skills are
   immutable versioned resources with schemas, permissions, evaluations,
   and dependency closure.

4. **Studio** — `apps/studio` (Tauri + React) implements Home → Sources
   → Transcript → Story → Beats → Timeline → Design → Motion & Sound →
   Compare → Finals → QA & Receipts → Settings. Studio is not a generic
   Premiere clone; it exposes the corrective operations automation most
   often gets wrong.

5. **Shared Capability Registry** — One versioned registry describes
   actions, skills, tools, models, runtime packs, renderers, assets,
   permissions, requirements, degradation, schemas, and evaluations. CLI
   commands, Studio bindings, embedded-agent tools, optional MCP tools,
   documentation tables, and contract fixtures are generated from or
   checked against this registry. There is one executor and one
   semantic action vocabulary.

## Book-by-book disposition

| Book | Title | Tasks | Evidence |
|------|-------|-------|----------|
| 1 | Reproducible Corpus, Licence Closure, and Standalone Boundary | 27/27 | `docs/dispatch/v2/book-1/{baseline.md, baseline-disposition.md, interface-freeze.md, orchestrator-variance.md, licence-report.md, merge-receipt.md, standalone-source-audit.json}` |
| 2 | Shared Capability Registry, Typed Actions, and Transactional Project State | 27/27 | `docs/dispatch/v2/book-2/{interface-freeze.md, capabilities.md, final-gate.md, final-manifest.json, focused-tests.md, gate-evidence.md, merge-receipt.md}` |
| 3 | Signed Runtime Packs, Hierarchical Evidence Graph, and Durable Job Plane | 27/27 | `docs/dispatch/v2/book-3/{interface-freeze.md, clean-runtime.md, final-gate.md, final-manifest.json, focused-tests.md, gate-evidence.md, merge-receipt.md}` |
| 4 | Benchmark-First Evaluation and Editorial Intelligence | 27/27 (B4-027 split into code + evidence commits — dispositioned) | `docs/dispatch/v2/book-4/{interface-freeze.md, B4-003-receipt.md, B4-027-duplicate-disposition.md, acceptance-summary.md, final-gate.md, final-manifest.json, gate-evidence.md, merge-receipt.md}` |
| 5 | Embedded Creative Operating System and Native Finish Renderer | 27/27 | `docs/dispatch/v2/book-5/{interface-freeze.md, merge-receipt.md, final-gate.md, final-manifest.json, gate-evidence.md}` |
| 6 | Full Studio Authoring Surface, Embedded Agent, and Optional MCP | 27/27 | `docs/dispatch/v2/book-6/{interface-freeze.md, dev-bundle.md, final-gate.md, final-manifest.json, gate-evidence.md, merge-receipt.md, visual-qa.md, workflow-tests.md}` |
| 7 | Measured Autonomy, Security Hardening, Offline Distribution, and Release Acceptance | 27/27 (clean-machine proof pending) | `docs/dispatch/v2/book-7/{interface-freeze.md, final-gate.md, final-manifest.json, gate-evidence.md, merge-receipt.md}` + `release/v2/{RC-MANIFEST.json, SHA256SUMS.txt, SBOM.spdx.json, provenance.json, THIRD-PARTY-NOTICES.md, clean-machine-host.json, sample-manifest.json, validation, acceptance, audit, source, rc, staging}` |

## Crate surface (v2)

```text
crates/video-actions/        B2 typed actions + capability registry binding
crates/video-agent/          B6 embedded agent + optional MCP loopback
crates/video-benchmarks/     B4 deterministic + model-based evaluators
crates/video-capabilities/   B2 capability registry model + validator
crates/video-cli/            B2/B7 single executor exposed as JSON CLI
crates/video-core/           B5 native compositor, render-graph compiler,
                             typography, motion, audio
crates/video-editorial/      B4 editorial engine + variant orchestration
crates/video-evidence/       B3 hierarchical evidence graph + retrieval
crates/video-feedback/       B7 decision records + per-format autonomy
crates/video-inference/      B3 signed runtime-pack resolution + adapters
crates/video-jobs/           B3 content-addressed job DAG, cache, resume
crates/video-media/          FFmpeg/FFprobe toolchain + render paths
crates/video-project/        canonical project state + revisions
crates/video-providers/      cloud envelope (Disabled + Fake only)
crates/video-recovery/       B7 crash recovery + project repair
crates/video-runtime/        B3 pack doctor, verification, local repair
crates/video-security/       B7 sandboxed worker execution + tamper detection
crates/video-services/       B3 runtime/evidence/job service façade
crates/video-sessions/       B5 product-local skill runtime + resolver
crates/video-state/          B2 immutable project revision storage
```

## Release candidate (v2 RC)

`release/v2/RC-MANIFEST.json` records the existing local release candidate.
It does not qualify the current repaired worktree; a new candidate must be
built and sealed during the build phase.

- Status: `local_release_candidate`
- Publish: `not_requested`
- Upload: `not_performed`
- Tag: `none`
- Targets: `macOS-arm64`, `macOS-x86_64` (`local_only`, `signed: false`,
  `seal: release/v2/rc/SEAL.json`)
- Packs: `v2-capability-core` (unsigned), `v2-skill-runtime` (unsigned)
- Source: `release/v2/source/source-manifest.json` + corresponding-source
- Samples: 4 production lanes (`recorded-talking-head`, `repurpose-podcast`,
  `procedural-explainer`, `anchored-product`)
- Tests: `benchmarks/runs/v2-release-candidate/four-lane-results.json`,
  `release/v2/acceptance/v2-rc-acceptance.json`
- Audits: `release/v2/audit/audit.json`, `SBOM.spdx.json`,
  `provenance.json`, `docs/release/V2-DISCLOSURE.md`
- Recorded candidate verification: `true` (historical candidate only)

## Native renderer

`crates/video-core/src/native_effect_renderer.rs` is the executable effect
renderer. `schemas/effects/registry.json` contains all 15 starter effects and
maps every entry to `native`.

`crates/video-core/src/render_graph.rs` (B5-017..021) provides:

- `RenderGraph`, `RenderGraphNode`, `RenderGraphEdge`,
  `RenderGraphCompileError`
- `RenderGraphCompiler::legacy_renderers()` returns
  `["remotion", "hyperframes", "hyper-frames"]`; any node whose inputs or
  `via` props resolve to a forbidden name produces
  `RenderGraphCompileError::LegacyRenderer(_)` and is rejected before
  compilation.

Remotion, ASS, and HyperFrames are retired, non-executable migration
references. Native rendering is the only shipping effect path.

## Honest limits inside what landed

These are recorded in code rather than papered over:

- **Fresh-user qualification remains build-phase work.** The harness and
  four sample lifecycle command are implemented, but B7-027 still requires
  execution against the newly built candidate on a fresh OS user with
  networking blocked.

- **Graphite is locked.** Production always selects Graphite. Tungsten and
  Pewter remain QA-only comparison themes, and the suite brand reference
  records the lock.

- **All 15 starter effects use native rendering.** ASS, Remotion, and
  HyperFrames are not executable dependencies, so local libass availability
  does not block the product path.

- **Rough-cut selection is semantic.** Red-thread selection uses semantic
  evidence with deterministic fallback and manual-review escalation when
  confidence is insufficient.

- **Cloud has no provider, by design.** B7-011 envelope is built —
  consent, hard budget, upload policy, content-hash cache,
  retention/delete, outage fallback, dedupe — with only `Disabled` and
  `Fake` adapters. Consent defaults off and the budget defaults to zero,
  so nothing can leave the machine.

## Disposition notes (audit round)

This audit round also recorded three disposition notes that answer the
specific shape deviations the audit flagged:

- `docs/dispatch/v2/book-1/baseline-disposition.md` — the v2 dispatch's
  B1-001 abort condition was satisfied at execution time (parent of
  `0963e7d` is `7f3e5a6`, the pinned baseline). Post-execution HEAD has
  since advanced through 189 task commits and post-release fixes, which
  is the expected outcome of a completed dispatch.

- `docs/dispatch/v2/book-4/B4-027-duplicate-disposition.md` — the
  duplicate `CR-V2-B4-027` commit message reflects a legitimate
  code + evidence split (commits `09ccce9` and `edfdb79`, 57 seconds
  apart). Both halves serve one task's acceptance.

- `docs/dispatch/v2/book-1/B1-malformed-id-disposition.md` — the
  `CR-V2-B1:` commit (`a80618c`) is a malformed task ID, but it was
  logged in `docs/dispatch/v2/book-1/orchestrator-variance.md` V-1 as
  preparatory tooling that fell between defined task boundaries.
  AGENTS.md requires variances past ±10% or outside manifest task
  accounting to be recorded; this one was.

## Tracking

- Workspace-side CHECKLIST: `release/v2/CHECKLIST.csv` (regenerated from
  git log; 189/189 tasks present + 1 malformed orchestrator row)
- Workspace-side VALIDATION: `release/v2/VALIDATION.json` (task pins
  per task SHA, baseline disposition, multi-commit + orchestrator
  variance references)
- Release manifest: `release/v2/RC-MANIFEST.json`
- Hash manifest: `release/v2/SHA256SUMS.txt` + `release/v2/rc/checksums.txt`
