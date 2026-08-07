# CR-V2-B7-027 — Final authoritative local gate

This defines the final authoritative local gate for Book 7 task
`CR-V2-B7-027`. Current status is pending until these commands pass against
the newly built candidate.

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
python3 scripts/release/v2-seal.py --verify release/v2/rc
python3 scripts/qa/v2-clean-machine/run.py --target host --bundle release/v2/rc --result release/v2/clean-machine-final-host.json --fresh-os-user
bash scripts/gate.sh --with-qa
python3 scripts/release/v2-seal.py --checksums release/v2/rc --out release/v2/SHA256SUMS.txt
```

## Acceptance

- Every claimed target has passing clean-machine and acceptance result.
- Strict seal verification, all required local gates, and checksums pass.
- The final manifest states no CI and no publication.

## Notes

- The local RC is unsigned. The signing script is held aside for the
  operator's manual invocation; the script never uploads.
- A passing fresh-user clean-machine result proves the release runtime has
  zero external runtime dependencies.
- CI is forbidden; the local gate is the single source of truth.
- No publication, signed release or external upload occurs in this book.
