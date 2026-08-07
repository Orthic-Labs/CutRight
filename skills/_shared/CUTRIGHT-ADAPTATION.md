# CUTRIGHT-ADAPTATION — skills/_shared (workspace-capabilities shared material)

Adaptation log for the shared reference material vendored under `skills/_shared`.
Source: workspace-capabilities @ `6ee21f03a787e7b57dc412760a8996ea7a235302`
(provenance: `skills/_shared/THIRD_PARTY.yml`; import graph:
`imports/v2/graphs/_shared.json`).

The six files were added byte-for-byte in CR-V2-B1-007 (`fd4f2e8`) (recorded
deviation D2 in `docs/dispatch/v2/book-1/merge-receipt.md`). Three of them were
subsequently adapted in CR-V2-B1-011 (`173ceab`, recorded deviation D1) to
remove references to the venture workspace checkout; the rewrites are recorded
below. The upstream notice bytes (`THIRD_PARTY.yml`) are unchanged.

## Rewrites made in commit 173ceab (CR-V2-B1-011)

| File | Upstream form | CutRight form | Semantic change |
|---|---|---|---|
| `illustrate/GUIDE.md` | Step 4 read the target brand entry in the workspace file `/Volumes/D/claude/.claude/rules/brands.md` before choosing colors | Step 4 loads the target brand entry via `cutright://skill/brand {"brand_code":"<code>"}` (typed result: `BrandCard`) | External workspace file read replaced with a typed CutRight skill call; no behavior lost |
| `illustrate/references/style-contract.md` | Right Suite rule read the app's locked light/dark tokens in `/Volumes/D/claude/.claude/rules/brands.md` & substituted its accent | Right Suite rule reads the app's locked light/dark tokens from the brand card (`cutright://skill/brand {"brand_code":"HR"}`) & substitutes its accent | Same token lookup, now routed through the CutRight-local brand skill |
| `parametric-design.md` | Header cited the workspace research corpus path `/Volumes/D/claude/newproj/para design/` (5 research docs, 2026-07) | Header cites the upstream parametric-design research corpus generically (5 research docs, 2026-07; venture workspace source, not vendored — this document carries the distilled rules) | Absolute workspace path citation replaced with a provenance statement; the distilled contract text itself is unchanged |

## Scope

Only the three files above diverge from the frozen import graph; every other
file under `skills/_shared` remains byte-for-byte identical to the imported
bytes. `skills/_shared` is shared reference material (no `SKILL.md`) and is not
a catalogue entry.
