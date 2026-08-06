# CUTRIGHT-ADAPTATION — Brand skill (CR-V2-B1-009)

Adaptation log for the vendored Brand skill. Source: workspace-capabilities
@ `6ee21f03a787e7b57dc412760a8996ea7a235302`, upstream path `tools/skills/brand`
(provenance: `THIRD_PARTY.yml`; import receipt: `imports/v2/receipts/brand.json`).

## 1. Venture data separated from executable logic

Upstream kept venture cards under `references/`. CutRight separates them:

| Upstream | CutRight | Role |
|---|---|---|
| `references/manual.md` | `brand-pack/manual.md` | DATA — per-venture cards, cross-brand rules, output format |
| `references/damned-designs.md` | `brand-pack/damned-designs.md` | DATA — DD canonical source |
| `references/heard-right.md` | `brand-pack/heard-right.md` | DATA — HR canonical source |
| `references/right-suite.md` | `brand-pack/right-suite.md` | DATA — HR/VR/SR/CR/MR identities |
| `references/rotten-hand.md` | `brand-pack/rotten-hand.md` | DATA — RH canonical source |
| `references/stunning-strangers.md` | `brand-pack/stunning-strangers.md` | DATA — SS canonical source |
| `references/toxic-sundae.md` | `brand-pack/toxic-sundae.md` | DATA — TS canonical source |
| — | `brand-pack/MANIFEST.md` | NEW — data-pack contract |

Base skill keeps schemas and generic logic only: `SKILL.md` (routing, `BrandCard` schema,
non-mutation guarantee), `references/cross-brand.md` (generic cross-brand motion rules),
`evals/`. `brand-pack/` is the optional signed creative data pack boundary.

## 2. Reference rewrites

| Upstream form | CutRight form | Where |
|---|---|---|
| `D:\Claude\heardright\` venture repo sources | declared data-pack-only content (not vendored; never reconstructed from memory) | `brand-pack/heard-right.md` |
| `D:\Claude\Content\Brand Identity\Right Suite\identity\lockups.html` | upstream venture asset, arrives via signed creative data pack | `brand-pack/right-suite.md` |
| `tools/skills/brand/references/<f>` | `skills/brand/brand-pack/<f>` | `brand-pack/heard-right.md`, `brand-pack/manual.md` |
| `/brand DD` … `/brand TS` slash invocations | `cutright://skill/brand {"brand_code":"<code>"}` | `brand-pack/manual.md`, `SKILL.md` frontmatter |
| `/brand-identity` | `cutright://skill/brand-identity` | `SKILL.md` |
| downstream "Writing, Content, Designer, Ads, Social, Marketing" | typed `cutright://skill/writing` / `content` / `designer` / `social`; upstream `ads` + `marketing` routing noted as not vendored | `SKILL.md` |

## 3. Typed artefact + contract additions

- `SKILL.md` now declares the `BrandCard { brand_id, voice, visual, restrictions, provenance }`
  artefact (task implementation shape) and the non-mutation guarantee: brand rules never
  mutate source media, timeline cuts, rendered assets, or code.
- `MAY_CALL_SKILLS` retyped to `cutright://skill/brand-identity`; effect profile stays
  read-only (`source_read`).

## 4. Gate evidence

- `python3 tools/import-closure/assert_no_external_refs.py skills/brand` → OK (run at commit time).
