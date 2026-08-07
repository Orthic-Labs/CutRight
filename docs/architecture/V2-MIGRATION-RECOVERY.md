# V2 — Migration, backup, recovery, and compatibility policy

Frozen by **CR-V2-B7-004**.

## Migration scope

v1 CutRight projects migrate cleanly. Each v1 artifact is mapped to an immutable v2 revision:

- v1 CutRight projects → `cutright.project_compat/v1` rows
- legacy skill, finish, and provider records → mapped table
- Remotion effect IDs → translation table to native render-graph equivalents
- prior Studio decisions → preserved as evidence nodes with stable IDs

## Backup before migration

The migration runner creates `.state/backups/{timestamp}.tar.zst` before any destructive step. Original sources and prior finals are preserved by reference; the v2 revision chain never loses a source binding.

## Recovery scenarios

Every recovery scenario produces a `cutright.recovery_report/v1`:

| scenario                | default action |
|-------------------------|----------------|
| interrupted action      | resume from receipt log + active pointer |
| interrupted job         | resume from job DAG; no half-applied state |
| corrupt index           | rebuild from receipt log + revision store |
| missing pack            | prompt local repair payload (offline) |
| tampered receipt        | rollback to last valid revision |
| partial installer       | repair from offline payload, verify checksums |

## Destructive downgrade is rejected

When the active revision or installed packs are newer than the target, the migration refuses and returns `destructive_downgrade_refused`.

## Schema contracts

- `schemas/migrations/project-compatibility.schema.v1.json` — `cutright.project_compat/v1`
- `schemas/recovery/recovery-report.schema.v1.json` — `cutright.recovery_report/v1`
