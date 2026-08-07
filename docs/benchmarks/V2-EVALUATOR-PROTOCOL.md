# CutRight v2 Evaluator Protocol

The evaluator protocol binds every finding in a benchmark report to a
deterministic evaluator, an independent critic, and a bounded revision
cycle. It is the single source of truth for what counts as independent
evidence.

## 1. Evaluator categories

Three categories of evaluator exist and they must remain distinct:

1. **Deterministic evaluators** — pure functions over `(inputs, expected)`.
   They run first and return a `pass`, `fail`, `skipped_with_reason`,
   `unsupported`, or `unproven` status (see `V2-TAXONOMY.md`).
2. **Planner self-assessment** — the Director (or shorts director) tagging
   its own confidence. Logged, never counted as independent evidence.
3. **Independent critic** — a separate model instance, separate prompt,
   separate process. Receives the brief, evidence references, semantic
   action diff, before/after samples, and deterministic findings. Returns
   a schema-bound verdict with frame/time evidence. Cannot execute actions.

## 2. Evidence citation

Every verdict must cite `evidence_refs` with exact:

- `source_range` — `[start_ms, end_ms]` from the source.
- `output_range` — `[start_ms, end_ms]` from the rendered output.
- `frame_refs` — sampled frame IDs at fixed N and at events.
- `word_ids` — for speech and boundary findings.

A verdict that contains zero `evidence_refs` cannot pass.

## 3. Revision cycle

```text
planner → deterministic checks → critic_A → at most one revision →
deterministic checks → critic_A → pass or escalate
```

- The first critic run produces a verdict.
- A revision is allowed exactly once per finding.
- A second disagreement, a low-confidence verdict, or a missing critic
  result escalates the finding — never silently passes.

## 4. Blindness and audit

- Variant identity is hidden from the critic where possible.
- The runner logs `evaluator_version`, `seed`, `sampling`, and pack
  locks with every finding.
- Two identical runs must be byte-stable except declared timestamps and
  run IDs.

## 5. Verdict schema

```json
{
  "id": "verdict-001",
  "metric_id": "speech.word_clipping.high_confidence",
  "evaluator": "critic-A",
  "evaluator_version": "1.0.0",
  "seed": 42,
  "status": "fail",
  "evidence_refs": [
    {"source_range": [1230, 1450], "output_range": [1200, 1400], "frame_refs": ["f-00123"], "word_ids": ["w-0001"]}
  ],
  "rationale": "kept high-confidence word 'sales' clipped by 50 ms",
  "confidence": 0.9
}
```

## 6. Self-score logging

The planner's self-assessment is logged next to the independent verdict:

```json
{"evaluator": "planner-A", "self_score": 0.86, "log_only": true}
```

The benchmark report shows both rows but only the independent verdict
counts.
