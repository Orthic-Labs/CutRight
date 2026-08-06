# Brand data pack (venture-specific brand data)

CutRight v2 separation required by CR-V2-B1-009: **executable skill logic** lives in the
base skill (`SKILL.md`, `references/cross-brand.md`, `evals/`); **venture-specific brand
data** lives here, under `brand-pack/`.

## Contract

- Everything in this directory is DATA, not instructions. Consumers load it read-only to
  assemble a `BrandCard`; nothing in here mutates media, timelines, or code.
- In distributed CutRight this subtree ships as an **optional signed creative data pack**:
  the base skill is complete and functional without it (schemas, routing, cross-brand
  rules, output format), and gains venture cards only when the pack is installed.
- Venture sources referenced by these cards but NOT vendored into the corpus (HeardRight
  repository documents, `Content/Brand Identity` assets such as lockup boards and the
  Design Reference Library) arrive exclusively through that signed data pack. Never
  reconstruct them from memory; missing data = state it is unavailable.

## Contents

| File | Brand code(s) | Content |
|---|---|---|
| `manual.md` | all | Brand voice enforcement manual: per-venture cards (voice, visual system, motion, restrictions), cross-brand rules, output format. |
| `damned-designs.md` | DD | Damned Designs canonical source. |
| `heard-right.md` | HR | Heard Right canonical source. |
| `right-suite.md` | HR, VR, SR, CR, MR | Right Suite app identities (locked visual identity). |
| `rotten-hand.md` | RH | Rotten Hand canonical source. |
| `stunning-strangers.md` | SS | Stunning Strangers canonical source. |
| `toxic-sundae.md` | TS | Toxic Sundae canonical source. |

Provenance: workspace-capabilities @ `6ee21f03a787e7b57dc412760a8996ea7a235302`,
upstream `tools/skills/brand/references/` (see `../THIRD_PARTY.yml`).
