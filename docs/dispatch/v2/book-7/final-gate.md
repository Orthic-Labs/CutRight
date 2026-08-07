# CR-V2-B7-027 — Final authoritative local gate

This freezes the final authoritative local gate evidence for Book 7 task
`CR-V2-B7-027`. The gate closes Book 7 once every claim is fulfilled.

## Required shape

```text
book: 7
product_boundary: standalone_offline
external_runtime_dependencies: 0
network_attempts_in_acceptance: 0
ci: forbidden
publish: false
```

## Commands

```bash
python3 scripts/release/v2-audit.py --bundle release/v2/rc --out release/v2/audit-final
python3 scripts/qa/v2-clean-machine/run.py --target host --bundle release/v2/rc --result release/v2/clean-machine-final-host.json
bash scripts/gate.sh --with-qa
python3 scripts/release/v2-seal.py --checksums release/v2/rc --out release/v2/SHA256SUMS.txt
```

## Acceptance

- Every claimed target has passing clean-machine and acceptance result.
- All required local gates pass and checksums verify.
- The final manifest states no CI and no publication.

## Notes

- The local RC is unsigned. The signing script is held aside for the
  operator's manual invocation; the script never uploads.
- The clean-machine harness proves the release runtime has zero external
  runtime dependencies.
- CI is forbidden; the local gate is the single source of truth.
- No publication, signed release or external upload occurs in this book.
