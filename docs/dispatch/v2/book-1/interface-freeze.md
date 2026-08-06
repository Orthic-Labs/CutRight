# Book 1 Interface Freeze

Frozen by CR-V2-B1-006 before the parallel lanes begin. Changing anything in
this document, `imports/v2/path-map.json`, or `imports/v2/ownership.json`
requires a serial merge task — lanes never edit these files.

## 1. Frozen destination roots

| Root | Lane | Content |
| --- | --- | --- |
| `skills/` | A | Workspace capability skills, adapted with notices |
| `vendor/heardright/` | B | Adapted HeardRight engine/core/platform source |
| `imports/provenance/` | B | Vox snapshot notices; cutaway/finish manifests and golden scripts |
| `runtime/source/` | B | Pinned llama.cpp, whisper.cpp, Silero VAD, FFmpeg sources |
| `third_party/` | C | Generated aggregated notices |
| `docs/legal/notices/` | C | Release-facing notice aggregation |

Every parallel output path has exactly one lane owner. No lane owns
`Cargo.toml`, `package.json` roots, lockfiles, `scripts/gate.sh`,
`AGENTS.md`, or release manifests; serial merge tasks (CR-V2-B1-022 onward)
own integration files.

## 2. Import receipt schema (frozen)

Every import action emits one receipt JSON at
`<destination>/.import-receipts/<source_id>.json`:

```json
{
  "schema_version": 1,
  "source_id": "string — corpus source_id",
  "revision_type": "commit | tag_resolved_to_commit | model_revision | attachment_hash | published_reference",
  "revision": "string — immutable revision from the corpus row",
  "destination": "string — destination root actually written",
  "file_count": 0,
  "total_bytes": 0,
  "sha256_of_sorted_path_list": "64 hex chars",
  "imported_by_task": "string — dispatch task id",
  "imported_at": "RFC 3339 UTC timestamp"
}
```

Receipts are append-only evidence; lane B produces them for vendored and
runtime sources, lane A for skills imports.

## 3. THIRD_PARTY.yml notice schema (frozen)

Every copied subtree carries `THIRD_PARTY.yml`:

```yaml
schema_version: 1
source_id: string            # corpus source_id
name: string                 # upstream project name
canonical_url: string        # upstream URL from the corpus row
revision: string             # immutable revision actually copied
license: string              # licence row governing this subtree
notice: |
  Upstream copyright notice text, preserved verbatim.
```

`license` must name a licence row from `imports/v2/dispositions.json`;
assets may never inherit a repository licence silently.

## 4. Clean-room observation schema (frozen)

Clean-room sources (AutoShorts, Palmier Pro) get one observation note at
`docs/legal/clean-room/<source_id>.md` with this machine-readable header:

```yaml
schema_version: 1
source_id: string
observed_at_revision: string      # immutable revision observed
observation_date: string          # ISO date
observed_behavior: string         # what was observed, not how it is coded
implementer_separation: string    # who/what implements without source access
no_copy_attestation: string       # attestation that nothing was copied
```

The note body describes behavior only: inputs, outputs, ordering, and
constraints. No upstream identifiers, function names, or code structure.

## 5. Lane boundaries

- **Lane A** writes only `skills/**`.
- **Lane B** writes only `vendor/**`, `imports/provenance/**`, and
  `runtime/source/**`, plus the receipts defined above inside those roots.
- **Lane C** writes only `tools/import-closure/**`, `tools/v2-evals/**`,
  `docs/legal/**`, and `third_party/**`.
- Cross-lane reads are allowed; cross-lane writes are not.
- Lanes run the guards from CR-V2-B1-005 and the closure scanner from
  CR-V2-B1-004 against their own roots before committing.
