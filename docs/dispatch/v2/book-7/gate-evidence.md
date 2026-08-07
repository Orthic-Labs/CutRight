# CR-V2-B7-027 — Book 7 final gate evidence

This file is the single-point summary of the Book 7 final local gate. It
is created alongside `final-gate.md`, `final-manifest.json` and
`release/v2/SHA256SUMS.txt`.

## Book

- **book**: 7
- **commit**: see `git log --oneline -1` after the B7-027 commit
- **command**: `CR-V2-B7-027`

## Required invariants

- `product_boundary: standalone_offline`
- `external_runtime_dependencies: 0`
- `network_attempts_in_acceptance: 0`
- `ci: forbidden`
- `publish: false`

## Outcome

| Check | Status |
|---|---|
| `python3 scripts/release/v2-audit.py --bundle release/v2/rc --out release/v2/audit-final` | pass |
| `python3 scripts/qa/v2-clean-machine/run.py --target host --bundle release/v2/rc --result release/v2/clean-machine-final-host.json` | pass |
| `bash scripts/gate.sh --with-qa` | pass |
| `python3 scripts/release/v2-seal.py --checksums release/v2/rc --out release/v2/SHA256SUMS.txt` | pass |

## Targets

| Target | Status | Checksum verified |
|---|---|---|
| host | pass | yes |

## Evidence pointers

- `docs/dispatch/v2/book-7/final-gate.md` — narrative gate closure.
- `docs/dispatch/v2/book-7/final-manifest.json` — machine-readable manifest.
- `release/v2/RC-MANIFEST.json` — release candidate manifest.
- `release/v2/rc/` — sealed artefacts.
- `docs/release/V2-RC-REPORT.md` — release candidate narrative.
- `release/v2/SHA256SUMS.txt` — SHA-256 sums for sealed artefacts.

## Constraints

- No CI. All evidence is local.
- No publication, signed release or external upload occurs in this book.
- The checksum seal binds every artefact to the final manifest.
- The head is recorded in the RC-MANIFEST and the BUILD.json.
