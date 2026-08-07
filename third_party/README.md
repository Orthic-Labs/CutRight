# CutRight v2 Third-Party Notice System

Created by CR-V2-B1-022. Frozen inputs: `imports/v2/source-corpus.json`,
`imports/v2/dispositions.json`, `docs/legal/V2-IMPORT-POLICY.md`, and the
THIRD_PARTY.yml schema frozen in `docs/dispatch/v2/book-1/interface-freeze.md`.

## 1. What lives where

| Path | Purpose |
| --- | --- |
| `third_party/README.md` | This document: notice system + templates. |
| `third_party/notices/<source_id>/THIRD_PARTY.yml` | One aggregated notice per materialized corpus source that has copied bytes. Verified by `tools/import-closure/verify_notices.py`. |
| `third_party/notices/clean-room-attestations.md` | Attestation notices for clean-room sources (no copied bytes, so no source notice). |
| `docs/legal/notices/` | Release-facing notice aggregation (e.g. `vox-director.txt`). |
| `runtime/source/README.md` | Corresponding-source archive layout and manifest contract. |
| `scripts/legal/build-corresponding-source.py` | Deterministic corresponding-source archive generator (stdlib-only, offline, local snapshots only). |

Per-subtree notices inside copied trees (`skills/*/THIRD_PARTY.yml`,
`vendor/heardright/THIRD_PARTY.yml`, `imports/provenance/vox-director/THIRD_PARTY.yml`)
remain the primary evidence; the files under `third_party/notices/` aggregate
them per corpus source for release packaging.

## 2. Frozen THIRD_PARTY.yml schema (interface-freeze §3)

```yaml
schema_version: 1
source_id: string            # corpus source_id
name: string                 # upstream project name
canonical_url: string        # upstream URL from the corpus row
revision: string             # immutable revision actually copied (never main/master/latest)
license: string              # licence row name from imports/v2/dispositions.json
notice: |
  Upstream copyright notice text, preserved verbatim.
```

`license` must name a licence row from `imports/v2/dispositions.json`; assets
may never inherit a repository licence silently.

## 3. Entry-kind templates

Five entry kinds exist. `source` is the frozen schema above; the others
extend it with kind-specific fields and are verified by the Book 7 release
validator, not by `verify_notices.py`.

### 3.1 source — copied code/docs (frozen schema)

Used by: `workspace-capabilities`, `heardright`, `vox-director` (see
`third_party/notices/<source_id>/THIRD_PARTY.yml`).

```yaml
schema_version: 1
kind: source
source_id: <corpus source_id>
name: <upstream project name>
canonical_url: <upstream URL>
revision: <immutable commit actually copied>
license: <licence row from imports/v2/dispositions.json>
copied_paths: <destination root, e.g. skills/designer>
notice: |
  <verbatim upstream notice>
```

### 3.2 binary — built binary shipped in a signed runtime pack

Every binary-runtime row MUST carry all of: `source_revision`,
`configure_args`, `patches`, `source_sha256`, `binary_sha256`, `notice_path`.
A row missing any of those fails
`scripts/legal/build-corresponding-source.py`.

```yaml
schema_version: 1
kind: binary
source_id: <corpus source_id>
name: <upstream project name>
canonical_url: <upstream URL>
revision: <source_revision the binary was built from>
license: <licence row>
configure_args: [<exact build configuration>]
patches: [<local patch paths, may be empty>]
source_sha256: <SHA-256 of the corresponding-source archive>
binary_sha256: <SHA-256 of the shipped binary>
notice_path: <path to the governing THIRD_PARTY.yml>
corresponding_source: runtime-source/<component>/<version>/<target>.tar.zst
```

### 3.3 model — model weights shipped in a signed pack

```yaml
schema_version: 1
kind: model
source_id: <corpus source_id>
name: <model name>
canonical_url: <upstream URL>
revision: <model_revision pinned in the corpus>
license: <licence row governing the WEIGHTS, not the repo>
weight_sha256: <exact byte hash frozen by the pack builder>
generated_from_source_model: true|false
pack: <signed pack name>
notice: |
  <verbatim upstream model licence>
```

### 3.4 asset — non-code, non-weight material (sample media, datasets, LUTs…)

```yaml
schema_version: 1
kind: asset
source_id: <corpus source_id>
name: <material name>
canonical_url: <upstream or attachment origin>
revision: <attachment_hash manifest reference>
license: <asset-class licence row; never inherited from a repo licence>
asset_sha256: <hash from the materialized manifest>
notice: |
  <verbatim notice or audited-separately status>
```

Used by: `attached-cutaway-finish-material` (provenance-only, never ships as
runtime code).

### 3.5 clean-room — behavior observed, nothing copied

Clean-room sources get **attestation notices, not source notices**, because
`copy_source` is false. See `third_party/notices/clean-room-attestations.md`.

```yaml
schema_version: 1
kind: clean-room
source_id: <corpus source_id>
name: <upstream project name>
canonical_url: <upstream URL>
revision: <immutable revision observed, never copied>
license: <upstream licence status as observed (may be none)>
observed_behavior: <what was observed, not how it is coded>
implementer_separation: <who/what implements without source access>
no_copy_attestation: <attestation that nothing was copied>
```

## 4. Corresponding-source archive policy

For every binary shipped in a signed runtime pack under a reciprocal
obligation (FFmpeg LGPL-2.1-or-later first), CutRight ships the
corresponding source as a deterministic archive under
`runtime-source/<component>/<version>/<target>.tar.zst`, generated by
`scripts/legal/build-corresponding-source.py` from the pinned local snapshot
in `runtime/source/<component>/`. The tool never fetches from the network;
if zstd compression is unavailable offline it falls back to an uncompressed
`.tar` with the same manifest contract. See `runtime/source/README.md`.

## 5. Current notice inventory (corpus date 2026-08-06)

Materialized sources with copied bytes — one THIRD_PARTY.yml each:

- `workspace-capabilities` → `third_party/notices/workspace-capabilities/THIRD_PARTY.yml`
- `heardright` → `third_party/notices/heardright/THIRD_PARTY.yml`
- `vox-director` → `third_party/notices/vox-director/THIRD_PARTY.yml`
- `attached-cutaway-finish-material` → `third_party/notices/attached-cutaway-finish-material/THIRD_PARTY.yml`

`cutright` itself is first-party MIT (`ship_source`); its dependency notices
aggregate here, so it needs no notice file of its own.

Clean-room attestations (no copied bytes): `autoshorts`, `palmier-pro` —
see `third_party/notices/clean-room-attestations.md`.

## 6. Unresolved rows are stated, never presented as resolved

No model bytes were copied in Book 1. Every unresolved model row in
`imports/v2/heardright-assets.json` (`referenced_external_assets`) stays
explicitly pending and gets NO model notice until the signed pack builder
freezes exact bytes and a licence row closes it:

| asset_id | status | why |
| --- | --- | --- |
| parakeet-tdt-primary | `blocked_unresolved` | exact upstream revision unverified; no CutRight redistribution grant |
| parakeet-rnnt-unified | `blocked_unresolved` | exact upstream revision unverified; no CutRight redistribution grant |
| silero-vad-coreml-16k | `blocked_unresolved` | generated CoreML conversion has no recorded licence row |
| whisper-large-v3-turbo-coreml | `blocked_unresolved` | conversion bytes unverified |
| whisper-tokenizer-json | `blocked_unresolved` | byte identity unverified |
| whisper-win-ggml-bin | `blocked_unresolved` | quantized bytes carry no recorded licence row |
| silero-vad-onnx-16k | `pending_not_materialized` | MIT permits redistribution, but exact SHA-256 must be frozen from the pinned silero-vad source before the row closes |
| sherpa-kws-wakeword-transducer, kws-owner-manifests | `excluded` | wake-word lane not adopted by CutRight v2 |

Kokoro-82M voices and phonemizer assets are `audited_separately` in the
ledger; no voice notice exists until each voice has its own provenance and
redistribution row. `blocked_unresolved` rows are release-blocking per
`docs/legal/V2-IMPORT-POLICY.md` §3; this scaffold asserts nothing resolved
for them.
