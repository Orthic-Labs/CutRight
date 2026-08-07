# CUTRIGHT-ADAPTATION — Writing skill (CR-V2-B1-011)

Adaptation log for the vendored Writing skill. Source: workspace-capabilities
@ `6ee21f03a787e7b57dc412760a8996ea7a235302`, upstream path `tools/skills/writing`
(provenance: `THIRD_PARTY.yml`; receipt: `imports/v2/receipts/writing.json`;
selection: `imports/v2/selections/writing.json`).

## 1. Import shape and exclusions

- Selected closure: SKILL.md, evals, references (manual, script, research-article,
  content-repurposer, repurpose-content), specialists editorial + copywriting (with hook/copy
  craft, ad copy, ad-assets, craft-research) — 16 files including the notice.
- Explicit exclusions with reasons (`imports/v2/exclusions/writing.json`):
  - `specialists/email`, `specialists/blogs`, `specialists/profile-copy`, `specialists/changelog`
    — lanes excluded per the CR-V2-B1-011 procedure.

## 2. Typed handoff artefacts (SKILL.md)

- Header retyped: `EFFECT_PROFILES: source_read, asset_delivery_emit`;
  `MAY_CALL_SKILLS: cutright://skill/brand`.
- Declared local typed artefacts: **ScriptPlan** (timed script output, consumed by content/social)
  and **PackageCopy** (gated conversion copy, consumed by designer). Prose is delivered as
  AssetDelivery; the skill never posts, schedules, or publishes.
- Excluded lanes listed with pointer to the exclusions file; boundaries retyped
  (content/designer/social typed; upstream marketing/ads marked not vendored).

## 3. Reference and specialist rewrites

| Upstream form | CutRight form | Where |
|---|---|---|
| `/brand` loads | `cutright://skill/brand {"brand_code":"<code>"}` (typed result BrandCard) | `references/manual.md`, `specialists/editorial/GUIDE.md`, `specialists/copywriting/references/hook.md`, `references/repurpose-content/reference.md` (prose) |
| `audit-visual` pairing | `cutright://skill/qa {"mode":"visual_review"}` | `references/manual.md`, `specialists/copywriting/GUIDE.md` |
| `tools/skills/_shared/*.md` | `skills/_shared/*.md` (vendored under CR-V2-B1-007) | `references/manual.md` |
| `.claude/rules/brands.md` voice rules | BrandCard from the brand skill | `references/manual.md`, `specialists/copywriting/references/hook.md` |
| blogs / email / profile-copy / changelog route rows | explicit excluded-lane rows pointing at the exclusions file | `references/manual.md`, `specialists/editorial/GUIDE.md`, `evals/evals.json` |
| `redesign` / designer route | `cutright://skill/designer` | `specialists/editorial/GUIDE.md` |
| `brand-identity` route | `cutright://skill/brand-identity` | `specialists/editorial/GUIDE.md`, `specialists/copywriting/GUIDE.md` |
| `ads` / `marketing` / `seo` routes | marked upstream lanes not vendored into CutRight v2 | `references/manual.md`, `specialists/copywriting/GUIDE.md`, `specialists/copywriting/references/ad.md`, `specialists/copywriting/references/hook.md` |
| Venture storytelling corpus paths (upstream Windows machine paths) | provenance note: corpus not vendored; extracted craft rules carried inline; arrives with signed creative data pack when installed | `specialists/copywriting/references/craft-research.md` |
| Optional external jury (host library invocation, file-scheme import) | typed review step via the qa skill (visual_review mode) or the evidence-graph review; opt-in only | `references/script.md`, `references/research-article.md`, `references/repurpose-content/reference.md`, `specialists/copywriting/references/ad.md`, `specialists/editorial/GUIDE.md` |

## 4. Evals retyped

- Rows for excluded lanes (blogs, email, profile, changelog) now expect the exclusion notice.
- Not-vendored routes (marketing) annotated; visual audit pairing retyped to the qa skill.

## 5. Gate evidence

- `python3 tools/import-closure/assert_no_external_refs.py skills/writing` → OK (run at commit time).
- `python3 tools/import-closure/verify_exclusions.py imports/v2/graphs/writing.json imports/v2/exclusions/writing.json` → PASS.
- `python3 tools/import-closure/verify_copy.py imports/v2/graphs/writing.json skills/writing` → PASS
  on the pre-adaptation snapshot (16 files).
