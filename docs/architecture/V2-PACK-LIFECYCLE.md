# V2 Pack Lifecycle (CR-V2-B3-005)

## Purpose

Freeze the lifecycle of every CutRight runtime pack: staging, signature/hash
verification, atomic activation pointer, retained previous pack, repair from
local installer payload, and explicit rollback. Release builds may never
repair by internet download or system package manager.

## Two Schemas

- `schemas/runtime/pack-activation.schema.v1.json` — the persistent
  activation record.
- `schemas/runtime/repair-result.schema.v1.json` — the result of a repair
  attempt.

## Activation Flow

```
stage payload -> verify manifest/signature/files -> write activation record -> fsync -> atomic active pointer swap -> retain previous version
```

The stages array on the activation record MUST contain all four stages in
order: `staged`, `verified`, `written`, `pointer_swapped`. A record without
all four is invalid.

## Signature and Hash Verification

Verification covers:

- The manifest hash matches the canonicalised manifest body.
- The signature digest matches the manifest hash.
- The signature algorithm is `blake3+ed25519` or `sha256+rsa-pss-3072`.
- Every file's SHA-256 and BLAKE3 match the stored values.

A failure at any step aborts the activation. The previous activation is
retained and the active pointer is not swapped.

## Atomic Activation Pointer

The active pointer file is a single text file containing the
`activation_id` of the latest record. The atomic swap uses the same
temp-file + rename + fsync pattern as project revisions.

## Retained Previous Pack

Every activation record references the previous activation. The on-disk
pack archive of the previous version is retained. Rollback deletes the
current archive and writes a new activation record that points at the
previous activation.

## Repair Sources

Repairs are exclusively local:

- `installer_payload` — bytes from the offline installer.
- `retained_previous` — bytes from the previously activated pack.
- `local_patch` — a file-level patch validated against the manifest.
- `developer_override` — disabled in release builds.

The release build refuses any `source` other than `installer_payload`,
`retained_previous`, or `local_patch`.

## Dev Override

A compile-time feature flag enables a `developer_override` source for
local development. The flag is OFF in release builds. The activation
record's `dev_override` field is set to true iff the dev override was
used; this is verified at receipt-emission time.

## Repair Result

Every repair attempt produces a `pack_repair_result/v1` record. The record
contains:

- `files_repaired` — paths and hashes of files that were repaired.
- `files_rejected` — paths and reasons for files that were rejected.
- `manifest_hash_after` — the manifest hash after the repair.

A `rejected` or `escalated` result leaves the active pointer untouched.

## Acceptance

- Repairs are exclusively local in release builds.
- Cloud fallback is impossible.
- Internet download is impossible.
- System package manager integration is impossible.
- Previous activation is retained on every swap.
- Active pointer is swapped atomically.

## Future Packs

The lifecycle is identical for every pack kind. Adding a new pack kind
does not change the lifecycle.
