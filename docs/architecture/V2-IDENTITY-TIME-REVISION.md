# V2 — Stable identifiers, rational time, and immutable revisions

Frozen by **CR-V2-B2-001**.

## 1. Stable identifiers

IDs are opaque strings drawn from `[A-Za-z0-9_-]+`. They are NEVER inferred from names, paths, display labels, or array indexes. Schema: `cutright.identity/v1` (`schemas/core/identity.schema.v1.json`).

Reserved id kinds: `project`, `timeline`, `track`, `clip`, `word`, `evidence_node`, `action_batch`, `job`, `asset`.

## 2. Rational time (no float canonical time)

- Source time: integer nanoseconds OR rational ticks (numerator, denominator, both `u64`).
- Timeline time: rational project ticks (24 000/s video, 48 000/s audio, integer ticks at the chosen rate).
- Conversions are exact or surface a typed rounding error. No silent precision loss.

## 3. Immutable revisions

Acyclic parent graph. Each revision carries `revision_id`, `parents[]` (0..2), `created_at_ns`, `active_pointer`, `compatibility_fp` (BLAKE3 over the frozen public surface). New revisions only from successful action batches. Schema: `cutright.revision/v1`.

## 4. Migration from string IDs / millisecond fields

v1 string IDs preserved by `migrations/v1-to-v2/identity-map.csv`. Millisecond `u64` fields multiply by `1_000_000` to ns and re-anchor to the closest rational tick. Source bindings preserved bit-for-bit; rows that cannot prove preservation report `unsupported_by_ledger_schema`.
