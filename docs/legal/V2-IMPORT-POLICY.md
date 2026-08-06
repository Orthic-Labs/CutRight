# CutRight v2 Import Policy

Status: frozen contract, created by CR-V2-B1-003. Governs every import made
under the v2 corpus (frozen 2026-08-06). Machine-readable form lives in
`imports/v2/dispositions.json`, validated by
`schemas/import/disposition.schema.v1.json`.

## 1. Terminal dispositions

Exactly eight terminal dispositions exist. Every source-corpus row
(`imports/v2/source-corpus.json`) must resolve to exactly one:

| Disposition | Meaning |
| --- | --- |
| `ship_source` | First-party or permissively licensed source that ships in the product. |
| `ship_runtime_pack` | Ships only as a signed CutRight runtime pack (binary, weights, or asset pack), never as loose repo files. |
| `adapt_with_notice` | Material is copied and adapted; upstream notices are preserved per the ledger's `notice_preservation` block. |
| `clean_room_behavior` | Behavior is reimplemented from written observation notes only. No source, asset, or configuration is copied; `copy_source` must be false. |
| `provenance_only` | Material is retained as a paper trail (hash manifests, golden scripts). Never shipped as runtime code. |
| `development_only` | Consulted during development (research citations, qualification candidates). Nothing copied, nothing shipped. |
| `excluded_with_reason` | Considered and rejected; the reason is recorded in the ledger notes. |
| `blocked_unresolved` | Licence question unresolved. Accepted by the import schema so work can continue elsewhere, but release-blocking. |

The Rust-side mirror of this enum is:

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Disposition {
    ShipSource,
    ShipRuntimePack,
    AdaptWithNotice,
    CleanRoomBehavior,
    ProvenanceOnly,
    DevelopmentOnly,
    ExcludedWithReason,
    BlockedUnresolved,
}
```

## 2. Separate licence rows per asset class

Assets never inherit a repository licence. Each imported asset class needs
its own explicit `licence_rows` entry: code, model weights, voices, fonts,
music, SFX, textures, LUTs, sample media, and datasets. A repository
declared "MIT" whose bundled model weights carry a non-commercial clause is
treated as two separate licence rows, and the weights row governs.

Rows whose licence is `audited_separately` (e.g. Kokoro voices,
user-supplied attachments) must be closed with an explicit follow-up row
before the affected asset enters a signed pack.

## 3. Release-blocking rules

The release validator (owned by Book 7) rejects a release when any of the
following hold; the import schema accepts them so the ledger can record
state honestly:

1. Any entry has `disposition: blocked_unresolved` (the schema forces
   `release_blocking: true` for such rows).
2. Any source-corpus row has no matching ledger entry (missing row).
3. Any asset class shipped inside a signed pack lacks an explicit licence
   row.
4. Any `audited_separately` row remains unclosed for a shipped asset.

## 4. Clean-room separation

**AutoShorts** (no declared licence) and **Palmier Pro** (GPL-3.0) are
clean-room sources. For each:

- Observed public behavior is written down in the ledger's `clean_room`
  block before implementation starts.
- Implementer separation: the agent writing the corresponding CutRight
  code does not read the upstream source during implementation.
- A no-copy attestation is recorded; the transitive closure scanner
  (CR-V2-B1-004) rejects any path that would introduce upstream files.
- GPL-3.0 source must never be copied into the MIT-licensed product.

## 5. Notice preservation

**Vox Director** (MIT) and **workspace capabilities** (user-owned with
third-party notices) are `adapt_with_notice` sources:

- Every copied subtree carries a `THIRD_PARTY.yml` with the upstream
  notice.
- Per-root ledger rows in `imports/v2/dispositions.json` map each copied
  root to its licence row.
- Notices aggregate into `docs/legal/notices/` for release packaging.

## 6. Validation

```sh
python3 -m json.tool imports/v2/dispositions.json >/dev/null
python3 scripts/schema-check.py schemas/import/disposition.schema.v1.json imports/v2/dispositions.json
```

Coverage check (every corpus row has a ledger entry) is enforced by the
import closure scanner from CR-V2-B1-004 onward.
