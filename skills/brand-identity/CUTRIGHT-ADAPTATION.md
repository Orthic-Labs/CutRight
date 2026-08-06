# CUTRIGHT-ADAPTATION — Brand Identity skill (CR-V2-B1-009)

Adaptation log for the vendored Brand Identity skill. Source: workspace-capabilities
@ `6ee21f03a787e7b57dc412760a8996ea7a235302`, upstream path `tools/skills/brand-identity`
(provenance: `THIRD_PARTY.yml`; import receipt: `imports/v2/receipts/brand-identity.json`).

## 1. Reference rewrites

| Upstream form | CutRight form | Where |
|---|---|---|
| `/designer` | `cutright://skill/designer` | `references/manual.md` (×4), `SKILL.md` token-set note |
| `/audit-visual` (skill not vendored) | `cutright://skill/qa {"mode":"visual_review"}` | `references/manual.md` (×3), `SKILL.md` |
| `/architect` (skill not vendored) | marked upstream/not-vendored; refactor tracked manually | `references/manual.md` (×3) |
| `/brand` | `cutright://skill/brand {"brand_code":"<code>"}` | `SKILL.md` |
| `tools/skills/_shared/parametric-design.md` | `skills/_shared/parametric-design.md` (vendored under CR-V2-B1-007) | `references/manual.md` |
| `tools/skills/_shared/anti-slop.md` | `skills/_shared/anti-slop.md` | `references/manual.md` |
| venture Design Reference Library + Color Bears paths (upstream Windows machine paths) | `brand-pack/design-reference-library/...` targets declared; content arrives via the optional signed creative data pack (not vendored) | `references/visual-reference-libraries.md` |

## 2. Typed artefacts + contracts

- `SKILL.md` declares the CutRight v2 artefacts: **BrandSystem**, **BrandTokenSet**
  (the `.brand/tokens.json` contract), **BrandRestrictionSet**; delivery is AssetDelivery
  only — locked assets are never overwritten (upstream rule 8 preserved).
- `EFFECT_PROFILES` retyped `asset_read, asset_delivery_emit`;
  `MAY_CALL_SKILLS` retyped to `cutright://skill/brand`.

## 3. Scripts

- `scripts/color-check.mjs` — zero-dependency WCAG/OKLCH color math (no network, no child
  processes, no writes). Received the CR-V2-B1-009 provenance header; invoked only as the
  typed capability `cutright://capability/brand_identity.color_check` inside the signed
  runtime pack.

## 4. Rules preserved (per task procedure)

Locked-identity invariants, accessibility standard (WCAG 2.2 AA normative), reproduction
tests, signature-mechanism requirement, and the brand-registry differentiation guard are
all carried over unchanged in `references/manual.md` and `references/brand-registry.md`.

## 5. Gate evidence

- `python3 tools/import-closure/assert_no_external_refs.py skills/brand-identity` → OK (run at commit time).
