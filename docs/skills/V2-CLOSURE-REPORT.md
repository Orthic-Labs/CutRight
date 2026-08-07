# V2 Closure Report — Book 1 (CR-V2-B1-024)

Scope: every source node that Book 1 (`Reproducible Corpus, Licence Closure,
and Standalone Boundary`) brought into contact with, classified into exactly
one of four buckets — **included**, **adapted**, **excluded**, **blocked**.
Zero unclassified nodes: every vendored root under `skills/`, `vendor/`,
`imports/provenance/`, `third_party/`, `runtime/source/`, and the adapted
tool roots appears in exactly one bucket below (§6 cross-check).

Evidence base (read-only inputs for this report):

- `imports/v2/receipts/*.json` (13 receipts)
- `imports/v2/exclusions/*.json` (content, qa, social, writing)
- `imports/v2/heardright-assets.json`
- `imports/v2/dispositions.json` and `imports/v2/dispositions/renderers.json`
- `imports/v2/clean-room/{autoshorts,palmier}.json`
- `imports/v2/source-corpus.json` (32 source nodes)
- `third_party/notices/*` and `fixtures/evals/exclusions.json`

Bucket definitions:

- **Included** — bytes vendored verbatim (notice additions only), the
  first-party shipping base, or a licence row fully closed for shipping in a
  signed runtime pack.
- **Adapted** — content or behavior absorbed with a recorded rewrite,
  adaptation log, or clean-room procedure.
- **Excluded** — considered and rejected with a recorded reason; no bytes in
  the release path.
- **Blocked** — cannot ship as-is: development-only qualification, citation
  only, or licence closure still outstanding.

## 1. Included

| Source node | Revision | Vendored root / evidence | Task |
| --- | --- | --- | --- |
| `cutright` | `7f3e5a61c729d4d877715b9a083d13a2e5ebe277` | First-party Orthic Labs shipping base (MIT); `skills/content-video-editor` is CutRight-native skill content | — |
| `heardright` | `b60bff947f12ffa9d25e94ad27e8ff30db006a24` | `vendor/heardright/` — 238 files copied byte-for-byte (engine, heardright_core, heardright_platform, heardright_capture), third-party notices preserved; receipt `imports/v2/receipts/heardright-source.json` | CR-V2-B1-012 |
| `attached-cutaway-finish-material` | `attachment-manifest:imports/provenance/cutaway-finish/hash-manifest.json` | `imports/provenance/cutaway-finish/` — 15 files, cutaway/ and finish/ trees copied verbatim (`diff -r` clean), hash-manifest bound; provenance-only, never runtime code | CR-V2-B1-014 |
| `llama-cpp` | `6a32c29a746a2e44de463de647f9f6661eb5086b` | MIT licence row closed in `imports/v2/dispositions.json`; ships as self-built binary in a signed runtime pack (pack materialization is a later-book task; `runtime/source/` holds the Book 1 placeholder) | 021 (disposition) |
| `whisper-cpp` | `306c88f4d1286aec1bf96e544632897886af5501` | MIT licence row closed; signed runtime pack as above | 021 (disposition) |
| `silero-vad` | `76e3dc408eb2a5c655c34e230d2d5459b4439daa` | MIT model-weight row closed; exact ONNX bytes + SHA-256 recorded in pack manifest | 021 (disposition) |
| `ffmpeg` | `9047fa1b084f76b1b4d065af2d743df1b40dfb56` | LGPL-2.1-or-later row closed; LGPL-only configure line (no `--enable-gpl`, no `--enable-nonfree`) carried with the installer | 021 (disposition) |
| `qwen3-4b` | `7c69a109fc3fa19c860be9dff46fc23299092018` | Apache-2.0 weights row closed; in-house deterministic GGUF conversion, frozen output hashes | 021 (disposition) |
| `qwen3-vl-4b-instruct` | `ebb281ec70b05090aa6165b016eac8ec08e71b17` | Apache-2.0 weights + mmproj row closed; critic pack manifest | 021 (disposition) |
| `kokoro-82m-v1-0` | `496dba118d1a58f5f3db2efc88dbdc216e0483fc89fe` | Apache-2.0 weights row closed; fixed model hash in pack manifest (voice rows audited separately) | 021 (disposition) |

HeardRight asset ledger (`imports/v2/heardright-assets.json`, task 013):
1 vendored asset — `engine/src/whisper_mel_128.bin` (MIT-derived per upstream
`WHISPER_MIT` notice; pack-manifest row must close it before the signed
speech pack) — plus `referenced_external_assets`, which are **blocked** (§4).
Full ledger: `docs/legal/HEARDRIGHT-ASSET-LEDGER.md`.

## 2. Adapted

| Source node | Revision | Destination / evidence | Task |
| --- | --- | --- | --- |
| `workspace-capabilities` | `6ee21f03a787e7b57dc412760a8996ea7a235302` | Skills vendored then adapted with per-skill `CUTRIGHT-ADAPTATION.md` logs: `skills/designer` (207 files), `skills/brand` (11), `skills/brand-identity` (7), `skills/content` (61), `skills/writing` (16), `skills/social` (13), `skills/qa` (8), plus `skills/_shared` reference material; receipts `designer/brand/brand-identity/content/writing/social/qa.json` | 007–011 |
| `workspace-capabilities` (tooling concepts) | same pin | Concept adaptations, zero copied bytes: `tools/v2-skill-compiler` + `tools/v2-skill-monitor` (receipt `bounded-run.json`, 35 adapted files), `tools/v2-evals` + `schemas/evals` + `fixtures/evals` (receipt `workspace-evals.json`, 25 files), `tools/v2-gauntlet` + `docs/testing/V2-GAUNTLET.md` (receipt `gauntlet.json`, 16 files) | 018–020 |
| `workspace-capabilities` (renderers) | same pin | `imports/v2/dispositions/renderers.json`: remotion and hyperframes — shipping disposition `provenance_only`, behavior disposition `clean_room_behavior`; native-renderer migration per `docs/architecture/NATIVE-RENDERER-MIGRATION.md` + `fixtures/native-renderer/manifest.json` | 021 |
| `vox-director` | `8b034354dc443edcde7fdb2622e0491df5142fd3` | Provenance snapshot `imports/provenance/vox-director/` (14 files verbatim, receipt `vox.json`) + MIT-licensed director concepts absorbed into `skills/video-director` (CUTRIGHT-ADAPTATION.md + THIRD_PARTY.yml); release-facing notice `docs/legal/notices/vox-director.txt`; third-party notice root `third_party/notices/vox-director/` | 015 |
| `autoshorts` | `f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b` | Clean-room behavior only: observation notes at `imports/provenance/behavior/autoshorts/`, attestation `imports/v2/clean-room/autoshorts.json`; no upstream bytes copied, implementer separation recorded | 016 |
| `palmier-pro` | `397b82e64093f986cbabd89f1a1c93812ff546c2` | Clean-room behavior only: observation notes at `imports/provenance/behavior/palmier/`, attestation `imports/v2/clean-room/palmier.json`; GPL-3.0 source never read during implementation | 017 |

Third-party notice roots for this bucket: `third_party/notices/workspace-capabilities/`,
`third_party/notices/vox-director/`.

## 3. Excluded

Every exclusion carries a recorded reason in the evidence file named.

**`workspace-capabilities` skill sub-nodes** (`imports/v2/exclusions/*.json`):

| Selection | Excluded node | Reason (abridged) |
| --- | --- | --- |
| content | `specialists/kdp` | not a CutRight video lane; stays in venture workspace |
| content | `specialists/carousel` | browser-capture lane; capture routes through `cutright://skill/qa` |
| content | `specialists/demo-recorder` | depends on workspace `tools/demo` runtime; routes through signed runtime pack |
| writing | `specialists/email`, `specialists/blogs`, `specialists/profile-copy`, `specialists/changelog` | not in the selected writing closure |
| qa | (behavioral) | browser-download assumptions removed; bundled/local tooling only |
| social | (behavioral) | no posting, scheduling, spending, or account mutation |

**`vox-director` sub-nodes** (8, receipt `vox.json`): `assets` (unaudited
showcase media incl. celebrity likenesses), `scripts` (hosted-provider
execution glue), `package.json`, `vox-director.skill`, `AGENTS.md`,
`.gitignore`, `README.zh.md`, `SKILL.zh.md` (packaging/hygiene/localized
duplicates).

**`heardright` sub-nodes** (16, receipt `heardright-source.json`): the
`tauri-app-next` application surface (`src`, `src-tauri`, `public`,
`artifacts`, `.cache`, `.cargo`, `parakeet-rs-bench`, `ios-native`,
`macos-native`, `bakeoff`, `qa`, `quality`, `verification`, `scripts`,
`docs`) and `heardright-engine/check.out` — app shells, generated artifacts,
and bench/verification trees not required by the selected crates.

**Workspace eval sources** (`fixtures/evals/exclusions.json`): 42 rows, every
surveyed `tools/skills/*/evals/evals.json` at the pin that was not imported
wholesale — out-of-scope lanes (ads, architect, coder, commit, cortex,
council, debugger, dispatch, handoff, jfdi, marketing + specialists,
research, seo, tasklist), hosted-provider specialists (image-enhancement,
seedance), `provenance_only` renderer evals (remotion), and skill evals whose
derivatives were adapted into `fixtures/evals/cases/` (e.g. transcription →
`cve-transcribe-captions`). Enforced by `tools/v2-evals/run.py`.

Upstream gaps recorded at import (absent at the pin, therefore uncopyable,
not excluded by choice): `designer/engine/scripts/lib/designer-paths.mjs`
(dangling in 11 scripts, receipt `designer.json`); `content/specialists/video-editor`
(referenced upstream but absent, receipt `content.json`).

## 4. Blocked

| Source node | Revision | Why blocked |
| --- | --- | --- |
| `mediapipe` | `f8ef212d5c962c0e853db7e59d217056b187084b` | `development_only`: telemetry-disabled, network-blocked qualification and per-model licence closure required before any vision-pack entry opens |
| `qwen3-5-4b` | `851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a` | `development_only`: qualification candidate; never silently replaces selected models |
| Research corpus (16 nodes): `research-videollamb`, `research-flash-vstream`, `research-providellm`, `research-lvagent`, `research-salova`, `research-adaptive-keyframe-sampling`, `research-longvale`, `research-vidhalluc`, `research-ave-compass`, `research-speecheditbench`, `research-unieditbench`, `research-five`, `research-v2v-bench`, `research-avid`, `research-taro`, `research-mmaudio` | ICCV/CVPR/arXiv citations in `imports/v2/source-corpus.json` | Citation only; no code or data copied |
| `heardright` referenced external assets: `parakeet-tdt-primary` (+ rows in `referenced_external_assets`) | upstream revision NOT verified at pin | Exact upstream bytes must be fetched, hashed, and licence-closed in the pack manifest before entering any signed pack |

## 5. Catalogue compiled in this task

- `skills/catalog.lock.json` — compiler pack from
  `tools/v2-skill-compiler` (`pack_id cutright-skill-pack-v1`,
  `pack_hash sha256:2f786f9bf312148c460f217ba6b836a3621d8251cfaae96d2fae67dca882ea30`),
  9 skills, deterministic (byte-identical across independent recompiles).
- `skills/catalog.json` — human/agent-readable projection of the same lock:
  stable IDs, versions (`0.1.0`), content hashes, committed `SKILL.md`
  hashes, dependencies, permissions, resources, `MAY_CALL_SKILLS` edges,
  eval-suite refs (`fixtures/evals/suites/import.json`), and the call-graph
  topology.
- `apps/studio/src/generated/skillCatalog.ts` — TypeScript read model,
  pure projection of the lock; `pnpm typecheck` in `apps/studio` passes.

**Compile-path deviation (recorded):** the committed compiler CLI
(`--root <root> --pack-out <file> --topology-out <file>`) requires
`id`/`version` front matter and has no `compile --root skills --out` form,
while lane A skills carry `name`/`description` front matter. Per task
constraints (no compiler-source edits, no skill-content rewrites), the lock
was produced by compiling a scratch adapter root whose `SKILL.md` copies add
`id: <dir-name>` / `version: 0.1.0` lines and flatten the one multi-line
description (`video-director`); bodies are byte-identical. Lock
`content_hash` values therefore cover adapted front matter; `catalog.json`
carries `committed_skill_md_sha256` for verification against the committed
tree. `_shared` is shared reference material (no `SKILL.md`) and is not a
catalogue entry. Reconciling the compiler with lane A front matter is a
finding for CR-V2-B1-027.

**Known cycle (recorded, deferred):** the `MAY_CALL_SKILLS` call graph in
`catalog.json` `topology.dependency_cycles` contains `[brand, brand-identity]`
and `[content, social]`; lock `dependencies` are empty (no skill declares
`depends:`), so every declared dependency resolves. Stale
`fixtures/evals/known-missing-skills.json` allow-list and the
`validate_skill_topology` cycle failure are deferred to CR-V2-B1-027 per
dispatch.

## 6. Vendored-root cross-check (zero unclassified)

| Vendored root | Bucket | Source node |
| --- | --- | --- |
| `skills/designer`, `skills/brand`, `skills/brand-identity`, `skills/content`, `skills/writing`, `skills/social`, `skills/qa`, `skills/_shared` | adapted | `workspace-capabilities` |
| `skills/content-video-editor` | included | `cutright` (first-party) |
| `skills/video-director` | adapted | `vox-director` |
| `vendor/heardright` | included | `heardright` |
| `imports/provenance/cutaway-finish` | included | `attached-cutaway-finish-material` |
| `imports/provenance/vox-director` | adapted | `vox-director` (snapshot for the absorbed concepts) |
| `imports/provenance/behavior/autoshorts` | adapted | `autoshorts` (clean-room notes) |
| `imports/provenance/behavior/palmier` | adapted | `palmier-pro` (clean-room notes) |
| `runtime/source/speech/.gitkeep` | included | speech runtime-pack licence rows (whisper-cpp, silero-vad, kokoro-82m-v1-0) |
| `third_party/notices/workspace-capabilities` | adapted | `workspace-capabilities` |
| `third_party/notices/vox-director` | adapted | `vox-director` |
| `third_party/notices/heardright` | included | `heardright` |
| `third_party/notices/attached-cutaway-finish-material` | included | `attached-cutaway-finish-material` |

All 32 `imports/v2/source-corpus.json` source nodes resolve to exactly one
bucket: included = cutright, heardright, attached-cutaway-finish-material,
llama-cpp, whisper-cpp, silero-vad, ffmpeg, qwen3-4b, qwen3-vl-4b-instruct,
kokoro-82m-v1-0 (10); adapted = workspace-capabilities, vox-director,
autoshorts, palmier-pro (4); blocked = mediapipe, qwen3-5-4b, 16 research
citations (18); excluded = 0 whole source nodes — exclusions in §3 are
recorded sub-nodes of the included/adapted sources above. 10 + 4 + 18 + 0 =
32. **Unclassified: 0.**

## 7. Findings deferred to CR-V2-B1-027

1. Stale `fixtures/evals/known-missing-skills.json` allow-list (all 6 listed
   skills now exist under `skills/`) → `validate_skill_topology.py`
   `MAY_CALL_SKILLS` cycle FAIL (merge-receipt §6.1).
2. `vendor/heardright/engine/Cargo.toml` sibling-repository `path =`
   dependency references → `scripts/gates/v2-repository-shape.sh` FAIL
   (merge-receipt §6.2).
3. Compiler/front-matter convention mismatch (this task's compile-path
   deviation, §5) plus the recorded call-graph cycles.
4. `fixtures/evals/exclusions.json` rows marked "revisit when
   skills/brand-identity lands" (brand-identity has since landed).
