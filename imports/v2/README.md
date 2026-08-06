# imports/v2 — provenance-only import staging

Status: **provenance only**. Nothing in `imports/v2/` is release runtime
code. It is the frozen paper trail for the CutRight v2 import programme.

## Contents

- `source-corpus.json` — machine-readable frozen corpus, validated by
  `schemas/import/source-corpus.schema.v1.json`. One row per source in
  `CutRight-v2-Source-Corpus-and-Ledger.md`, frozen 2026-08-06.
- `dispositions.json` — licence and disposition ledger, created by
  CR-V2-B1-003 and validated by `schemas/import/disposition.schema.v1.json`.
- `path-map.json`, `ownership.json` — frozen lane ownership, created by
  CR-V2-B1-006.

## Rules

1. Revisions are immutable: commits, tags resolved to commits, model
   revisions, attachment hashes, or fixed published references. `main`,
   `master`, `latest`, and unversioned download URLs are invalid.
2. Every corpus row needs exactly one terminal disposition.
   `blocked_unresolved` rows and missing rows are release-blocking.
3. Copied material lands in its declared destination root with a
   `THIRD_PARTY.yml`; behaviour-only sources get a clean-room note.
4. Release builds never read `imports/v2/`. The shipping application reads
   only CutRight-owned skills, schemas, binaries, models, assets, and pack
   manifests produced from this corpus.

## Validation

```sh
python3 -m json.tool imports/v2/source-corpus.json >/dev/null
python3 scripts/schema-check.py schemas/import/source-corpus.schema.v1.json imports/v2/source-corpus.json
```
