# CutRight v2 Golden Corpus

The golden corpus is the rights- and source-bound reference set that every
benchmark run evaluates against. It is intentionally small at the start;
rows are added one at a time with full provenance before any metric may
claim release evidence.

## Layout

```text
benchmarks/corpus/
├── manifest.json          # canonical project + split + rights list
├── rights/                # per-project rights + consent + provenance
│   ├── <project_id>.json
│   └── ...
├── sources/               # (out of band) the actual media bytes once owned
└── labels/                # human annotations / reviewer IDs
```

## Rights record schema

Every project gets a record under `rights/<project_id>.json` with:

- `licence` — `local_only` or `redistributable`.
- `owner` — the human or organisation granting the right.
- `consent_record` — written consent / release / model release.
- `source_hashes` — BLAKE3 hashes of every media file.
- `provenance` — where the media came from (record date, session, capture device).
- `reviewer_ids` — who annotated the labels.

## Split policy

Every project is assigned exactly one split — `train`, `calibration`, or
`test`. The split is bound to the speaker, recording session, and source
program so no near-duplicate can cross splits.

## Adding a project

1. Add the row to `manifest.json` with `missing_fixture: true` while the
   media bytes are not yet on disk.
2. Once the bytes are present, hash them with BLAKE3 and append to
   `source_hashes`.
3. Drop the `rights/<project_id>.json` file with the schema above.
4. Run `python3 scripts/benchmarks/validate-corpus.py benchmarks/corpus/manifest.json`.

Placeholder rows with `missing_fixture: true` are excluded from runnable
benchmark reports and counted as `unproven`.
