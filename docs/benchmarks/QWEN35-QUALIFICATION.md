# Qwen3.5-4B Qualification (CR-V2-B3-015)

## Purpose

Run the qualification suite for Qwen3.5-4B without making it a release
dependency. The candidate is `no_promote`; the active pack lock is
unchanged.

## Source

- Upstream: `github.com/QwenLM/Qwen3.5`
- Pinned revision: `851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a`
- Licence: Apache-2.0

## Suite

1. Director fixtures (structured-output, tool-choice, editorial-eval)
2. Critic fixtures (layout-collision, identity-preservation,
   label-preservation, crop-stability, visual-instruction,
   temporal-order)
3. Target runtime tests
4. Memory and latency measurements
5. Deterministic structured-output checks

## Comparison

Results are compared against the selected Qwen3/Qwen3-VL packs using
blind result IDs. The script aborts if any required target fails.

## Promotion gate

Promotion is a SEPARATE step. The qualification candidate never
modifies the active release pack manifests. The merge step
`CR-V2-B3-022` enforces this by checking that no candidate manifest
appears in `runtime/manifests/`.

## Acceptance

- `qualification.mode == "no_promote"`
- `active_pack_lock_unchanged()`
- Results include failures and unsupported runtime features.
- No automatic promotion occurs.
