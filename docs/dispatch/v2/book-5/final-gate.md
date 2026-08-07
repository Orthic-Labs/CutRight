# CR-V2-B5-027 — Book 5 authoritative local gate

This freezes the authoritative Book 5 local gate evidence. Book 5 closes
once the creative-skill lane merge, the native renderer compile, the
critic-citation policy and the focused tests all pass.

## Required shape

```text
book: 5
network_attempts: 0
path_fallbacks: 0
ci: forbidden
legacy_renderer_attempts: 0
critic_axes_passing: 10/10
```

## Commands

```bash
cargo check --workspace --all-targets --locked
cargo test -p video-core --tests --locked
python3 scripts/gates/v2-legacy-renderer.py --check
python3 scripts/gates/v2-creative-critic.py --axes 10
bash scripts/gate.sh --with-qa
```

## Acceptance

- All required checks pass.
- `RenderGraphCompiler::legacy_renderers()` rejects remotion /
  hyperframes / hyper-frames at compile.
- `video-core` test suite passes (91 tests across unit, four_lane_fixtures,
  focused_creative).
- Native renderer fixtures in `fixtures/creative/golden-fixtures.json`
  match the migration-comparison baseline (B5-025).
- Final manifest binds commit, fixture hash, and test evidence.