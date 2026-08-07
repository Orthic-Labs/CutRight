# Book 6 Interface Freeze

**Book:** 6 — Full Studio Authoring Surface, Embedded Agent, and Optional MCP
**Status:** Frozen after CR-V2-B6-006
**Owners:** serial freeze (B6-001 … B6-006); Lane A / Lane B / Lane C from B6-007
**Purpose:** lock the public surface so three independent lanes can build in parallel without redefining each other's contracts.

## 1. Frozen contracts

The following files are authoritative for Book 6 and may not be edited inside
any lane's exclusive path. Only the serial merge tasks (`CR-V2-B6-022`
through `CR-V2-B6-027`) may wire them together.

| Contract                                 | Frozen by      |
| ---------------------------------------- | -------------- |
| `docs/product/V2-STUDIO-IA.md`           | B6-001         |
| `schemas/studio/navigation.schema.v1.json` | B6-001       |
| `schemas/studio/project-view.schema.v1.json` | B6-001      |
| `apps/studio/src/contracts/navigation.ts` | B6-001         |
| `schemas/studio/project-index.schema.v1.json` | B6-002     |
| `docs/architecture/V2-PROJECT-INDEX.md`  | B6-002         |
| `apps/studio/src/contracts/projectIndex.ts` | B6-002      |
| `docs/product/V2-STUDIO-ACTIONS.md`      | B6-003         |
| `schemas/studio/action-intent.schema.v1.json` | B6-003    |
| `apps/studio/src/contracts/actionIntent.ts` | B6-003       |
| `docs/product/V2-TIMELINE-UX.md`         | B6-004         |
| `schemas/studio/timeline-view.schema.v1.json` | B6-004    |
| `apps/studio/src/contracts/timeline.ts`  | B6-004         |
| `docs/product/V2-EMBEDDED-AGENT.md`      | B6-005         |
| `schemas/agent/turn.schema.v1.json`      | B6-005         |
| `schemas/agent/plan.schema.v1.json`      | B6-005         |
| `schemas/agent/tool-result.schema.v1.json` | B6-005       |

## 2. Lane ownership

Lane A — modes/core-workflow
- Home, Sources, Transcript, Story, Beats, Run, Compare, Finals, QA & Receipts.
- File roots: `apps/studio/src/modes/{HomeMode,SourcesModeV2,TranscriptMode,StoryMode,BeatsMode,RunMode,CompareModeV2,FinalsModeV2,QaReceiptsMode}.tsx`,
  `apps/studio/src/hooks/useProjectLibrary.ts`,
  `apps/studio/src-tauri/src/project_index.rs`.
- Tasks: `CR-V2-B6-007` … `CR-V2-B6-011`.

Lane B — modes/authoring
- Timeline, Design, Motion & Sound, Assets + Auditions, corrective operations.
- File roots: `apps/studio/src/modes/{TimelineMode,DesignMode,MotionSoundMode}.tsx`,
  `apps/studio/src/components/timeline/**`, `apps/studio/src/components/design/**`,
  `apps/studio/src/components/motion/**`, `apps/studio/src/components/audio/**`,
  `apps/studio/src/components/{AssetPanel,AuditionPanel,CorrectionBar,HistoryPanel}.tsx`,
  `apps/studio/src/hooks/{useTimeline,useDesign,useAssets,useHistory}.ts`.
- Tasks: `CR-V2-B6-012` … `CR-V2-B6-016`.

Lane C — agent + inspection + a11y / perf
- Embedded agent session, generated tool registry, planning, evidence, diff review,
  composited inspection, sample sheets, optional loopback MCP project navigation
  and write guards, accessibility, reduced motion, keyboard, performance budgets.
- File roots: `crates/video-agent/src/**`, `crates/video-agent/tests/**`,
  `crates/video-project/src/inspect.rs`, `apps/studio/src-tauri/src/{inspect_commands,mcp_settings}.rs`,
  `apps/studio/src/components/{AgentPanel,CompositedInspector}.tsx`,
  `apps/studio/src/a11y/**`, `apps/studio/src/performance/**`,
  `apps/studio/src/contracts/agent.ts`, `docs/product/V2-ACCESSIBILITY-PERFORMANCE.md`.
- Tasks: `CR-V2-B6-017` … `CR-V2-B6-021`.

## 3. Serial merge / integration / acceptance

The following tasks are reserved for the Book integration line and may not
be started by any lane.

- `CR-V2-B6-022` — replace root Studio navigation.
- `CR-V2-B6-023` — integrate persistent jobs, recovery, notifications, digests.
- `CR-V2-B6-024` — deterministic visual QA fixtures for every mode.
- `CR-V2-B6-025` — four-lane Studio workflow tests with embedded agent.
- `CR-V2-B6-026` — local dev app bundle with all v2 modes.
- `CR-V2-B6-027` — authoritative Book 6 local gate and final manifest.

These tasks touch root navigation, the global JobCenter, the dev bundle,
and the QA lane. Lanes A / B / C must not edit those roots.

## 4. Disjoint rule

- Lane A does not touch `apps/studio/src/modes/{Timeline,Design,MotionSound}Mode.tsx`,
  `apps/studio/src/components/timeline/**`, `apps/studio/src/components/design/**`,
  `apps/studio/src/components/motion/**`, `apps/studio/src/components/audio/**`,
  `crates/video-agent/**`, or any Lane B / C UI surface.
- Lane B does not touch the Home mode, the index rebuild backend, the
  Story / Beats / Compare / Finals / QA surfaces, or any Lane A / C UI
  surface outside of cross-mode evidence viewers.
- Lane C does not own UI state for A or B modes; it owns only the
  AgentPanel, the CompositedInspector, the a11y / perf infrastructure, the
  optional MCP surface, and the embedded-agent backend.

## 5. Shared services (no single lane owns)

- `apps/studio/src/contracts/navigation.ts`            — frozen, read-only.
- `apps/studio/src/contracts/projectIndex.ts`         — frozen, read-only.
- `apps/studio/src/contracts/actionIntent.ts`          — frozen, read-only.
- `apps/studio/src/contracts/timeline.ts`             — frozen, read-only.
- `apps/studio/src/contracts/agent.ts`                — Lane C may extend with
  read-only helpers but may not change the discriminator or risk enum.
- `schemas/actions/action-batch.schema.v1.json`       — frozen, read-only.
- `schemas/actions/action-result.schema.v1.json`      — frozen, read-only.
- `schemas/actions/semantic-diff.schema.v1.json`      — frozen, read-only.
- `schemas/capabilities/registry.schema.v1.json`      — frozen, read-only.

## 6. Merge-conflict resolution

A merge conflict between lanes is resolved against this document. If two
lanes legitimately need to touch the same file, the lane that owns the
file per §2 is correct; the other lane must refactor.

## 7. Book 6 / Book 7 boundary

Book 7 owns migration and release hardening. Lane A / B / C must not edit
the v1 → v2 migration paths, the SBOM / licence pipeline, or the
clean-machine acceptance harness. The acceptance surface for those lands in
`CR-V2-B6-026` (dev bundle) and `CR-V2-B6-027` (final gate).