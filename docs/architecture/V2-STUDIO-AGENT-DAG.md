# CutRight v2 Studio + Agent DAG

This document is the workspace-level view of the Book 6 Studio authoring
surface, embedded agent, and optional MCP. It freezes the contract
hierarchy, the lane ownership, the merge sequence, the four-lane
acceptance, the audit and SBOM trail, and the final gate.

It exists so the architecture, capability, and tooling layers can answer
the question "what owns what?" without diffing source. It is the
authoritative input to `scripts/architecture/check_crate_dag.py`.

## 1. Contracts layer (frozen by B6-001 … B6-006)

```text
studio  ──→ navigation / project-index / project-view / action-intent / timeline-view
agent   ──→ turn / plan / tool-result
actions ──→ action-batch / action-result / inverse-batch / semantic-diff
caps    ──→ capability-registry / permission-set
```

The contracts layer is read-only for every lane; only the serial freeze
tasks may add a contract. Every UI mutation resolves to one of these
contracts.

## 2. Lane A — modes / core-workflow

```text
Home → Sources → Transcript → Story → Beats → Run → Compare → Finals → QA & Receipts
```

Lane A owns:

- the rebuildable project library,
- sources / transcript (immutable source + corrected text),
- story / beats (selection, alternates, score),
- run / make-versions (DAG, jobs, digest),
- compare / finals / QA (selection history, receipts).

It consumes Lane B timeline outputs through the registered executor and
consumes Lane C agent suggestions through the action-intent pipeline.

## 3. Lane B — modes / authoring

```text
Timeline → Design → Motion & Sound → Assets → Auditions → Correction Bar / History
```

Lane B owns:

- the non-destructive timeline editor,
- design (creative plan, asset requests, delivery review),
- motion & sound (effects, audio graph, audition),
- assets / auditions panels,
- corrective operations + comprehensive undo / redo.

Lane B does not mutate Lane A's edit selections directly. The Story /
Beats layer remains the source of "selected take".

## 4. Lane C — agent + inspection + a11y / perf

```text
embedded agent ──→ planner ──→ read tools ──→ action batch ──→ executor
                                                              │
                                                              ▼
                                             composited inspection (sample sheets)
                                                              │
                                                              ▼
                                                optional loopback MCP
                                                              │
                                                              ▼
                                          a11y / reduced-motion / keyboard / perf
```

Lane C owns:

- the agent session, planner, executor, communication style,
- composited timeline inspection + sample sheets,
- optional loopback MCP project navigation + write guards,
- accessibility / reduced motion / keyboard / perf budgets.

The optional MCP surface reuses Lane B and Lane A shared services; it
adds no second executor and no second capability registry.

## 5. Lane merge sequence (serial, B6-022 … B6-027)

```text
1. CR-V2-B6-022  replace root Studio navigation (route state + ModeRail + router)
2. CR-V2-B6-023  integrate persistent jobs, recovery, notifications, digests
3. CR-V2-B6-024  deterministic visual QA fixtures for every mode
4. CR-V2-B6-025  four-lane Studio workflow tests with embedded agent
5. CR-V2-B6-026  build local dev bundle with all v2 modes
6. CR-V2-B6-027  authoritative local gate, final manifest, SBOM trail
```

Merge conflicts are resolved against `docs/dispatch/v2/book-6/interface-freeze.md`.

## 6. Acceptance surfaces

- **Functional QA**: deterministic fixtures, asserted state, captured selectors.
- **Visual QA**: light / dark / reduced-motion / app-only viewports.
- **Workflow QA**: four-lane create → make → review → select.
- **MCP QA**: optional loopback MCP bound to its project, write-guarded.
- **Runtime boundary**: no `PATH` discovery, no network, no cloud key.

## 7. Audit, SBOM, release candidate, final gate

The four-lane acceptance feeds into:

1. `audit` — capability drift, schema-version drift, lockfile drift.
2. `SBOM` — generated for the dev bundle, hashes recorded in the manifest.
3. `release candidate` — `release/v2/RC-MANIFEST.json` plus dev bundle hash.
4. `final gate` — `scripts/gate.sh --with-qa` plus the Book 6 final gate.

The final gate is the only authoritative run; it freezes
`docs/dispatch/v2/book-6/final-manifest.json` and `final-gate.md`.

## 8. Lane ownership matrix

| Surface                                 | Owner      |
| --------------------------------------- | ---------- |
| `apps/studio/src/App.tsx`               | serial     |
| `apps/studio/src/components/ModeRail.tsx` | serial   |
| `apps/studio/src/hooks/useStudioRouter.ts` | serial   |
| `apps/studio/src/modes/HomeMode.tsx`    | Lane A     |
| `apps/studio/src/modes/SourcesModeV2.tsx` | Lane A   |
| `apps/studio/src/modes/TranscriptMode.tsx` | Lane A  |
| `apps/studio/src/modes/StoryMode.tsx`   | Lane A     |
| `apps/studio/src/modes/BeatsMode.tsx`   | Lane A     |
| `apps/studio/src/modes/RunMode.tsx`     | Lane A     |
| `apps/studio/src/modes/CompareModeV2.tsx` | Lane A   |
| `apps/studio/src/modes/FinalsModeV2.tsx` | Lane A    |
| `apps/studio/src/modes/QaReceiptsMode.tsx` | Lane A  |
| `apps/studio/src/modes/TimelineMode.tsx` | Lane B    |
| `apps/studio/src/modes/DesignMode.tsx`  | Lane B     |
| `apps/studio/src/modes/MotionSoundMode.tsx` | Lane B |
| `apps/studio/src/components/timeline/**` | Lane B    |
| `apps/studio/src/components/design/**` | Lane B     |
| `apps/studio/src/components/motion/**`  | Lane B     |
| `apps/studio/src/components/audio/**`   | Lane B     |
| `apps/studio/src/components/CorrectionBar.tsx` | Lane B |
| `apps/studio/src/components/HistoryPanel.tsx` | Lane B |
| `crates/video-agent/**`                 | Lane C     |
| `crates/video-project/src/inspect.rs`   | Lane C     |
| `apps/studio/src/components/AgentPanel.tsx` | Lane C |
| `apps/studio/src/components/CompositedInspector.tsx` | Lane C |
| `apps/studio/src/a11y/**`               | Lane C     |
| `apps/studio/src/performance/**`        | Lane C     |

## 9. Shared services

| Service                              | Owner        |
| ------------------------------------ | ------------ |
| Capability registry                  | shared infra |
| Action executor                      | shared infra |
| Project read model                   | shared infra |
| Job plane / DAG / recovery           | shared infra |
| Receipt tree / tamper detection      | shared infra |

Shared services are not editable inside any lane; only the Book 7
release-hardening line touches the registry-level surface.

## 10. Anti-promises

- No lane may redefine a mode owned by another lane.
- No lane may add a second executor or a second capability registry.
- No lane may bypass the contracts layer with a JSON-editing affordance.
- No lane may own UI state on behalf of another lane.
- No Book 6 acceptance step may run unverified fixtures or random IDs.