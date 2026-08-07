# CR-V2-B2-026 — Cross-surface transaction and contract tests

This document freezes the focused cross-surface test evidence for Book 2 task `CR-V2-B2-026`.

## Scope

The same action batch is executed through four surfaces and the canonical
JSON, the resulting revision, the receipt and the error codes are asserted
to be byte-for-byte identical:

| Surface | Test file |
|---|---|
| Direct Rust | `tests/v2/action_surfaces.rs` (`execute_direct`) |
| JSON CLI | `tests/v2/action_surfaces.rs` (`execute_cli`) |
| Studio Tauri command | `apps/studio/src/action-contract.test.ts` (`executeTauri`) |
| Loopback MCP adapter | `tests/v2/action_surfaces.rs` (`execute_mcp`) |

## Fixtures

* `asset_plan` — drives `cap.asset.plan` over two sample inputs.
* `evidence_read` — drives `cap.evidence.read` over the `evidence_graph` scope.
* `stale_revision` — same fixture with a stale revision id to assert rejection.
* `interruption` — same fixture with a `cap.interrupt` action to assert short-circuit.

## Acceptance contract

```text
assert_eq!(direct.canonical_json(), cli.canonical_json());
assert_eq!(direct.canonical_json(), tauri.canonical_json());
assert_eq!(direct.canonical_json(), mcp.canonical_json());
```

All four surfaces must surface:

* the same `revision` id (`rev-<fixture>:<revision>`).
* the same `receipt` id (`rcpt-<fixture>`).
* the same canonical JSON body (the surface marker is stripped before
  comparison for the Tauri path).
* the same permission-set id embedded in the body.

## Bypass resistance

The focused tests assert that no surface can bypass:

1. the permission check (every result carries the `actions` array),
2. the revision check (stale revisions are rejected consistently),
3. the canonical JSON envelope (the same body is emitted through every
   surface).

## Commands

```bash
cargo test --workspace --locked action_surfaces
pnpm --dir apps/studio test -- --run action-contract
```

## Evidence

The two suites above compile and pass under the focused checks listed in
the dispatch file. The full authoritative local gate is reserved for
`CR-V2-B2-027`.
