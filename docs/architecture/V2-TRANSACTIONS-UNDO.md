# V2 — Transactional apply, inverse actions, and failure semantics

Frozen by **CR-V2-B2-003**.

## 1. Staged apply pipeline

1. **Staged clone** — clone active revision; no writes to live project.
2. **Full semantic validation** — every action validated against staged revision.
3. **Atomic artifact writes** — temp-file + rename.
4. **Revision commit** — staged state becomes new immutable revision.
5. **Receipt emission** — `action_result/v1` written.
6. **Active-pointer swap** — only after stages 1–5 succeed.

On failure, active pointer is NOT advanced; staged clone is discarded.

## 2. Inverse action generation

Every applied batch produces a corresponding `inverse_batch/v1`. Inverse batches are generated at apply time (not author time). Non-reversible actions declare why and require a preserved prior revision.

## 3. Failure codes

`stale_revision`, `missing_target`, `invalid_range`, `permission_denied`, `resource_limit`, `partial_output`, `unknown_action_kind`, `validation_error`. `partial_output` requires inverse rollback.

## 4. Interruption injection points

`INJ_BEFORE_REVISION_COMMIT`, `INJ_BEFORE_RECEIPT_EMIT`, `INJ_BEFORE_ACTIVE_SWAP` for atomicity tests. Recovery examines staged clone + receipt log + active pointer and resumes/rolls back deterministically.
