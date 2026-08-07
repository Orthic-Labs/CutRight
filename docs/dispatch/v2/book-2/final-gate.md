# CR-V2-B2-027 — Book 2 authoritative local gate

This freezes the authoritative Book 2 local gate evidence. Book 2 closes
once the invariant `all mutation surfaces use video-executor` holds and
the listed gates pass.

## Required shape

```text
book: 2
required_invariant: all mutation surfaces use video-executor
ci: forbidden
```

## Commands

```bash
bash scripts/gates/v2-capability-drift.sh
python3 scripts/architecture/check_crate_dag.py docs/architecture/V2-CRATE-DAG.md
bash scripts/gates/v2-repository-shape.sh
bash scripts/gate.sh --with-qa
```

## Acceptance

- All required checks pass.
- Generated registry bindings are clean.
- Final manifest binds the exact commit and evidence.

## Notes

The loopback MCP adapter (B2-025) and the cross-surface contract tests
(B2-026) bind every mutation surface to the shared `video-executor`. CI
is intentionally forbidden for this book; the local gate is the single
source of truth.
