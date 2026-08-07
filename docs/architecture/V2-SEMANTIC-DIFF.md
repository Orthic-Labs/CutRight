# V2 — Semantic dry-run and diff output

Frozen by **CR-V2-B2-005**.

## 1. What the diff describes

For every supported action family — cuts, restores, moves, take swaps, retimes, captions, graphics, audio, colour, exports, settings — the diff schema (`schemas/actions/semantic-diff.schema.v1.json`) carries: `action_kind`, `target_id`, `before_id`/`after_id`, `range` (rational-tick start/end), `duration_delta_ns`, `evidence_refs[]`, `confidence` (0..1), `risk_flags[]`.

## 2. Order stability

Stable order: by `(timeline_id, track_id, start_ns, action_kind)`. Snapshot tests do not flap on hash/pointer noise.

## 3. Dry-run vs real apply parity

Same validator and apply planner, on a staged clone. Byte-identical `planned_revision` and diff for same `expected_revision`. Only observable difference: active pointer swap + receipt write.

## 4. Renderers

Stable JSON object (the schema itself) and human-readable line: `cut clip_clip_5 (12.000 → 14.000 s) removes filler "um" [-200 ms, conf 0.92]`. No log parsing permitted to reconstruct the diff.
