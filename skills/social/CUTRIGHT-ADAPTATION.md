# CUTRIGHT-ADAPTATION — Social skill (CR-V2-B1-011)

Adaptation log for the vendored Social skill. Source: workspace-capabilities
@ `6ee21f03a787e7b57dc412760a8996ea7a235302`, upstream path `tools/skills/social`
(provenance: `THIRD_PARTY.yml`; receipt: `imports/v2/receipts/social.json`;
selection: `imports/v2/selections/social.json`).

## 1. Import shape and exclusions

- Selected closure: SKILL.md, evals, references (manual, cross-platform content + platforms,
  post-templates, reverse-engineering, instagram, pinterest, twitter, youtube) — 13 files
  including the notice.
- Behavioral exclusion with reason (`imports/v2/exclusions/social.json`): posting, scheduling,
  and account connectors are excluded — no skill may post, schedule, spend, or mutate an account.
  Platform rules are carried only as versioned data.

## 2. Typed artefacts + hard boundary (SKILL.md)

- Header retyped: `DISCOVERY_PROFILE: D1_SCOPED_SOURCE`;
  `EFFECT_PROFILES: source_read, asset_delivery_emit`; `EXTERNAL_REQUESTS_MAX: 0`
  (upstream `external_research, connector` profile removed);
  `MAY_CALL_SKILLS: cutright://skill/brand, cutright://skill/designer, cutright://skill/content, cutright://skill/writing`.
- Declared local typed artefact: **PlatformConstraintSet** (versioned platform rules consumed
  when shaping platform-native artifacts).
- Hard boundary declared: publishing steps are emitted as plan data for the operator; live
  analytics are operator-provided (no session automation, no API tokens).

## 3. Reference rewrites

| Upstream form | CutRight form | Where |
|---|---|---|
| `EFFECT_PROFILES: external_research, connector` + request budget 12 | `source_read, asset_delivery_emit`, requests 0 | `references/manual.md` |
| `/brand <codes>` loads | `cutright://skill/brand {"brand_code":"<code>"}` (typed result BrandCard) | `references/manual.md`, `references/instagram/reference.md`, `references/pinterest/reference.md`, `references/youtube/reference.md`, `evals/evals.json` (prose) |
| `tools/skills/_shared/*.md` | `skills/_shared/*.md` (vendored under CR-V2-B1-007) | `references/manual.md` |
| `/designer`, `/content`, `/content carousel` routes | `cutright://skill/designer`, `cutright://skill/content`, `cutright://skill/qa {"mode":"capture"}` | `references/manual.md`, `references/instagram/reference.md`, `references/pinterest/reference.md` |
| `/marketing`, `/seo`, `/marketing-design` routes | upstream lanes not vendored (marked); design retyped | `references/manual.md`, `references/pinterest/reference.md`, `references/youtube/reference.md` |
| Venture storytelling corpus path (upstream Windows machine path) | provenance note: corpus not vendored; extracted craft rules carried inline | `references/content/reference.md` |
| Venture workspace marketing-context files | operator-provided context (files not vendored) | `references/content/reference.md` |
| Logged-in `agent-browser` IG dashboard scraping + daemon-repair steps | excluded: operator-provided analytics evidence; no connectors, no session automation | `references/instagram/reference.md` |
| Pinterest Business API + Tailwind scheduling | excluded connectors; cadence as publishing-plan data | `references/pinterest/reference.md` |
| `/transcribe` chain | `cutright://capability/content.transcribe` (signed content runtime pack) | `references/youtube/reference.md` |

## 4. Rules preserved

Platform-native craft rules, hard gates (platform-native, brand, hook, proof, anti-slop, series,
action), council role passes, per-platform formats/limits/cadence, and the SS passion-project
guard are all carried unchanged as versioned data.

## 5. Gate evidence

- `python3 tools/import-closure/assert_no_external_refs.py skills/social` → OK (run at commit time).
- `python3 tools/import-closure/verify_exclusions.py imports/v2/graphs/social.json imports/v2/exclusions/social.json` → PASS.
- `python3 tools/import-closure/verify_copy.py imports/v2/graphs/social.json skills/social` → PASS
  on the pre-adaptation snapshot (13 files).
