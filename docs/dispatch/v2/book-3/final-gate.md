# CR-V2-B3-027 — Book 3 authoritative local gate

This freezes the authoritative Book 3 local gate evidence. Book 3 closes
once the runtime-boundary gate, the pack-lock gate, the corresponding-
source gate, the clean-runtime gate and the focused tests all pass.

## Required shape

```text
book: 3
network_attempts: 0
path_fallbacks: 0
ci: forbidden
```

## Commands

```bash
python3 scripts/gates/v2-runtime-boundary.py --check
python3 scripts/legal/validate-v2-ledger.py --scope book-3
bash scripts/qa/v2-clean-runtime.sh
bash scripts/gate.sh --with-qa
```

## Acceptance

- All required checks pass.
- No unresolved materialized runtime asset remains.
- Final manifest binds commit, pack locks and test evidence.

## Notes

- Network attempts must be zero.
- Path fallbacks (system PATH, environment override) must be zero in
  release runs.
- The clean-path runtime smoke test is the canonical proof that the
  release runtime has no external runtime dependencies.
- CI is forbidden; the local gate is the single source of truth.
