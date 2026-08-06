# Vox Director → CutRight video-director: adaptation record

Adapted from the MIT-licensed Vox Director skill at commit
`8b034354dc443edcde7fdb2622e0491df5142fd3` (provenance snapshot:
`imports/provenance/vox-director/`; licence notice:
`docs/legal/notices/vox-director.txt`). This document lists every upstream
behavior that was absorbed, adapted, or dropped, per the CutRight v2
import policy for `adapt_with_notice` material.

## Absorbed (adapted into typed, local-only vocabulary)

| Upstream concept | CutRight form |
| --- | --- |
| Narrative arc library (hook/payoff, PAS, BAB, AIDA, StoryBrand, how-it-works, timeline, man-in-hole, story spine, origin, myth-buster, listicle, three-act, story circle) | `Arc` enum in `schemas/shot-plan.schema.json`; heuristics in `references/arcs-and-coverage.md` |
| Hook patterns, ≤3s hook rule, firm beat-count presets, proportion split | `HookPattern` enum + pacing rules in the reference |
| Shot sizes and coverage playbooks | `ShotSize` enum (`EST_WIDE`/`WIDE`/`MEDIUM`/`CLOSE`/`DETAIL`) |
| Constrained flat-safe camera vocabulary + banned moves | `CameraMove` / `BannedCameraMove` enums; strict-mode validation |
| Element-motion as independent axis, rigid-paper limits | `ElementMotion` struct in the schema |
| Anti-monotony rhythm presets | Reference section 6; enforced as planning rules |
| Style theme/palette decision | `StyleDecision` typed bake-off (2–4 candidates) |
| A/B/C-roll input modalities | `mode` field (`b_roll`/`a_roll`/`c_roll`) |
| Bounded generation-step semantics | Bounded job rules in `SKILL.md` (declared I/O, retry budget, typed failure states) |

## Dropped — unsupported original behaviors (complete list)

These upstream behaviors are NOT part of the CutRight skill and are not
referenced anywhere under `skills/video-director/`:

1. **Hosted cloud execution** — the upstream `scripts/` directory drives a
   hosted rendering API (cloud job submission, status polling, result
   download). Dropped: CutRight plans are executed only by local, bounded
   CutRight jobs.
2. **Hosted image generation provider names and model selection** — all
   upstream image-model identifiers and provider-routing logic are dropped.
3. **Hosted video generation provider names and model selection** — all
   upstream video-model identifiers are dropped.
4. **Hosted TTS / voice provider names and model selection** — dropped;
   narration stays as typed `narration_intent` text only.
5. **Hosted music provider names and model selection** — dropped.
6. **Asset upload/download code paths** — no upload, download, or sync
   client exists or is referenced here.
7. **Watermarking / branding overlay behavior tied to the hosted tier** —
   dropped.
8. **Provider credentials and API-key handling** — the skill never reads,
   stores, or requests credentials; there are no env-var lookups for
   providers.
9. **Upstream output directories and result-file layouts** — the skill emits
   typed plans only; it writes no media, timeline, or account state.
10. **Upstream skill-packaging artifacts** (`vox-director.skill`,
    `package.json` distribution metadata, localized `*.zh.md` duplicates,
    upstream `AGENTS.md`) — excluded from the provenance snapshot with
    reasons recorded in `imports/v2/receipts/vox.json`.
11. **Showcase media assets** (upstream `assets/` sample videos and
    thumbnails) — excluded from the snapshot; not needed for the concepts.

## Kept as provenance only (not part of the skill)

The selected upstream text materials (`SKILL.md`, `README.md`, `llms.txt`,
`references/`, `examples/`, `LICENSE`) live unmodified under
`imports/provenance/vox-director/` as the attribution and reference record.
They are inputs to this adaptation, not runtime files of CutRight.

## Capability-name policy

All capability references use CutRight names only
(`cutright://skill/content`, `cutright://skill/brand`, …). No upstream
provider, endpoint, model, or account identifier appears in this skill;
`assert_no_external_refs.py skills/video-director` enforces this.
