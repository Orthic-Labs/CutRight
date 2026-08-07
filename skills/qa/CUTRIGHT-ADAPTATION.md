# CUTRIGHT-ADAPTATION — QA skill (CR-V2-B1-011)

Adaptation log for the vendored QA skill. Source: workspace-capabilities
@ `6ee21f03a787e7b57dc412760a8996ea7a235302`, upstream path `tools/skills/qa`
(provenance: `THIRD_PARTY.yml`; receipt: `imports/v2/receipts/qa.json`;
selection: `imports/v2/selections/qa.json`).

## 1. Import shape and exclusions

- Selected closure: SKILL.md, evals, references (manual, browser-automation), scripts
  (qa.mjs engine, qa-functional.mjs, qa-shot.mjs) — 8 files including the notice.
- Behavioral exclusion with reason (`imports/v2/exclusions/qa.json`): browser-download
  assumptions removed — QA runs only bundled/local tooling. The vendored engine discovers an
  already-installed Chrome/Edge (`findBrowser`); it downloads nothing.

## 2. Runtime contract rewrite (SKILL.md)

- Declared typed plans: **FunctionalQaPlan** (frozen routes, viewports, selectors, states,
  actions → evidence artifacts) and **VisualQaPlan** (state coverage + screenshot evidence +
  VisualReviewResult).
- Execution retyped to signed-pack capabilities: `cutright://capability/qa.qa`,
  `cutright://capability/qa.qa_functional`, `cutright://capability/qa.qa_shot`.
- Typed entry points for other skills preserved: `cutright://skill/qa {"mode":"visual_review"}`
  and `{"mode":"capture"}`; contract-tests mode kept.
- Boundary declared: QA observes local apps under test only; never posts, spends, or mutates an
  account; no network connector.

## 3. Reference rewrites

| Upstream form | CutRight form | Where |
|---|---|---|
| `tools/skills/qa/scripts/` | `skills/qa/scripts/` + signed-pack capability note | `references/manual.md` |
| Machine-specific browser daemon tier (host CLI, machine Windows binary path, daemon-repair steps) | removed from required paths: Tier 1 = bundled local CDP runners, Tier 2 = optional host-native browser capability, Tier 3 = heavy snapshot tools as last resort; daemon-repair steps not vendored | `references/browser-automation.md` |
| `expected_skill: debugger` eval route | marked not-vendored lane (upstream debugger is outside the CutRight v2 selection) | `evals/evals.json` |

## 4. Scripts

- `scripts/qa.mjs`, `scripts/qa-functional.mjs`, `scripts/qa-shot.mjs` — received CR-V2-B1-011
  provenance headers; inert provenance in the base CutRight runtime (no Node on PATH required);
  execute only as the typed capabilities above inside the signed qa runtime pack.
- Substance preserved: dependency-free engine, raw CDP against installed Chrome/Edge,
  viewport-only shots, deterministic mocks at the IPC/API boundary, scoped process teardown.

## 5. Gate evidence

- `python3 tools/import-closure/assert_no_external_refs.py skills/qa` → OK (run at commit time).
- `python3 tools/import-closure/verify_exclusions.py imports/v2/graphs/qa.json imports/v2/exclusions/qa.json` → PASS.
- `python3 tools/import-closure/verify_copy.py imports/v2/graphs/qa.json skills/qa` → PASS on the
  pre-adaptation snapshot (8 files).
