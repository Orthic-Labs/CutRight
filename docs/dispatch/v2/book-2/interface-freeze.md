# Book 2 — Interface freeze

Frozen by **CR-V2-B2-006**.

## Lane ownership

| lane | owns |
|------|------|
| P-A  | `crates/video-actions/` — typed Action enum, validation, dry-run, atomic apply, undo |
| P-B  | `crates/video-capabilities/` + generated bindings |
| P-C  | `crates/video-state/`, `crates/video-sessions/`, migrations, contract fixtures |

## Dependency direction (frozen)

```
core (identity/time/revision)  ←  state  ←  actions  ←  capabilities
                                             ↑
                                      project orchestrates
                                             ↓
                                lower crates never depend on
                                project / CLI / Studio / MCP
```

- `crates/video-core` is the bottom layer (frozen in B2-001).
- state/actions/capabilities depend only on `video-core`.
- `crates/video-project` orchestrates; never depended on by lower crates.
- CLI, Studio, optional MCP depend on `video-project` only.

## Serial (orchestrator-owned) surface

Root workspace manifest, single `ActionExecutor` integration, CLI plumbing, Studio backend wiring, optional loopback MCP adapter — reserved for B2-022…B2-027.

## Frozen public names (lanes may use but not rename)

From B2-001: `Identity`, `IdentityKind`, `Revision`, `ParentLink`, `ActivePointer`, `CompatibilityFingerprint`, `TimeNs`, `Ticks`, `RationalTicks`.
From B2-002: `Capability`, `CapabilityKind`, `CapabilityRegistry`, `ActionBatch`, `Action`, `ActionKind`.
From B2-003: `ActionResult`, `FailureCode`, `InverseBatch`, `ApplyPipeline`.
From B2-004: `PermissionSet`, `Scope`, `SessionBinding`.
From B2-005: `SemanticDiff`, `DiffEntry`, `RiskFlag`.
