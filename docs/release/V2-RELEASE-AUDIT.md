# V2 Release Audit (CR-V2-B7-024)

**Audit id:** `v2-rc-release-audit-2026-08-07`
**Generated:** 2026-08-07
**Bundle:** `release/v2/staging`
**SEAL:** `release/v2/staging/SEAL.json`

## Policy

```text
release audit status = pass only if all policy.required_checks == pass
skipped is never coerced to pass
unproven is never coerced to pass
```

`pass_only_when_all_required_checks_pass` is enforced. Skipped and
unproven checks are recorded in `release/v2/audit/audit.json` but never
reduce the gate to `pass`.

## Required checks

| # | Check                       | Status | Evidence                                            |
|---|-----------------------------|--------|-----------------------------------------------------|
| 1 | threat_model                | pass   | `docs/security/V2-THREAT-MODEL.md`                  |
| 2 | sandbox_resource_limits     | pass   | `crates/video-worker/src/sandbox.rs`                |
| 3 | network_denial              | pass   | `scripts/qa/v2-clean-machine/run.py`                |
| 4 | secret_scan                 | pass   | `release/v2/audit/secret-scan.json`                 |
| 5 | pack_tamper                 | pass   | `release/v2/audit/pack-tamper.json`                 |
| 6 | project_tamper              | pass   | `release/v2/audit/project-tamper.json`              |
| 7 | licence_ledger              | pass   | `release/v2/staging/licences`                       |
| 8 | corresponding_source        | pass   | `release/v2/source/source-manifest.json`            |
| 9 | dependency_licences         | pass   | `release/v2/staging/licences/DEPENDENCY-LICENSES`   |
| 10 | forbidden_renderer         | pass   | `scripts/gates/v2-no-legacy-renderer.py`            |
| 11 | forbidden_runtime          | pass   | `scripts/gates/v2-runtime-boundary.py`              |
| 12 | skill_closure              | pass   | `tools/v2-skill-compiler/closure.json`              |
| 13 | source_corpus_leakage      | pass   | `release/v2/audit/source-corpus-leakage.json`       |
| 14 | installer_permissions      | pass   | `release/v2/audit/installer-permissions.json`      |

Required: 14 — Pass: 14 — Fail: 0 — Skipped: 0 — Unproven: 0.

## Skipped and unproven

None. Every required check ran on the RC.

## Installer contents

The macOS-arm64 and macOS-x86_64 installers do not require elevation,
do not autostart on login, do not write outside the user-chosen app
directory, and do not make any network call after install. Windows and
Linux installers are OUT OF SCOPE for v2.

## Release-blocking findings

`release_blocking_finding: false`

## Audit status

`pass`

## Artefacts

- `release/v2/audit/audit.json`
- `release/v2/audit/secret-scan.json`
- `release/v2/audit/pack-tamper.json`
- `release/v2/audit/project-tamper.json`
- `release/v2/audit/source-corpus-leakage.json`
- `release/v2/audit/installer-permissions.json`
