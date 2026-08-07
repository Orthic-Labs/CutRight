# V2 — Crate dependency DAG

Frozen by **CR-V2-B2-006**.

## Diagram

```
                        ┌──────────────────────────┐
                        │   apps/studio            │
                        └──────────┬───────────────┘
                        ┌──────────▼───────────────┐
                        │   crates/video-project   │  (orchestrator)
                        └──────────┬───────────────┘
              ┌────────────────────┼─────────────────────┐
   ┌──────────▼────────┐  ┌────────▼────────┐  ┌─────────▼─────────┐
   │ crates/video-     │  │ crates/video-   │  │ crates/video-     │
   │ actions (Lane P-A)│  │ state (Lane P-C)│  │ capabilities(P-B) │
   └──────────┬────────┘  └────────┬────────┘  └─────────┬─────────┘
              └────────────────────┼─────────────────────┘
                        ┌──────────▼───────────────┐
                        │   crates/video-core      │
                        └──────────────────────────┘
```

CLI → project only. Studio (frontend) → IPC → project. Optional loopback MCP → project only. vendor/heardright NO upstream dep. skills/* NO upstream Rust dep.

## Rules

1. **No cycles.** `cargo metadata` runs at gate time; cycles fail.
2. **Lower never depends on higher.** `video-core` never depends on state/actions/capabilities/project/cli/studio.
3. **Orchestrator-only composition.** `video-project` is the only crate that composes the others.
4. **Generated bindings are outputs.** Code generation in lane P-B produces artifacts excluded from rust-analyzer workspace discovery.

## Lane-ownership table

| crate / dir                                  | owner       |
|----------------------------------------------|-------------|
| `crates/video-core/`                         | orchestrator |
| `crates/video-actions/`                       | P-A          |
| `crates/video-capabilities/` + `bindings/`    | P-B          |
| `crates/video-state/`                         | P-C          |
| `crates/video-sessions/` + migrations/       | P-C          |
| `crates/video-project/` + `crates/video-cli/` | orchestrator |
| `apps/studio/`                               | orchestrator |
