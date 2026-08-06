# V2 Gauntlet — local hardening lane

`tools/v2-gauntlet` is CutRight's diff-scoped evidence gauntlet: a pure-std
Rust binary that mechanically probes whether the tests around a change can
actually fail. It is a concept adaptation of the workspace evidence gauntlet
(source_id `workspace-capabilities`, pin
`6ee21f03a787e7b57dc412760a8996ea7a235302`, `tools/gauntlet`); no upstream code
was copied.

## The three layers

1. **Changed-line mutation testing.** For every changed line in the supplied
   manifest, the first matching mutator injects one syntactic fault
   (`return N` → `return 0`, `return x` → `return !(x)`, `==` → `!=`,
   `===` → `!==`, `&&` ↔ `||`, `>` → `>=`, `<` → `<=`) into a throwaway copy
   of the workspace, reruns the tests, and restores nothing because the copy
   is discarded. A mutant that **survives** proves the tests cannot fail on
   that change; the layer is `Failed`.
2. **Changed-line coverage.** Probes for an instrumenting backend
   (`cargo llvm-cov`). When the backend is absent the layer is `Unproven` —
   never `Passed`. When present, it computes the fraction of changed lines
   hit at least once; any inconclusive step degrades to `Unproven`.
3. **Seeded test-order randomisation.** A xorshift64-seeded Fisher–Yates
   shuffle reorders the discovered test set for `--order-runs` runs. The seed
   and every derived run order are recorded in the receipt; the same seed
   always reproduces the same order.

## Layer statuses

```rust
pub enum LayerStatus {
    Passed,
    Failed,
    Skipped { reason: String },
    Unproven { reason: String },
}
```

- `Skipped` — nothing runnable matched (e.g. unsupported mutation shape, no
  test files discovered). Reasons are always recorded.
- `Unproven` — a required backend is unavailable or a run was inconclusive.
  **An unavailable backend is `Unproven`, never a pass.**
- Only `Failed` is a gate failure; `Skipped`/`Unproven` are reported, not
  laundered into success.

## Language support

Rust (`.rs`) and TypeScript (`.ts`, `.tsx`, `.mts`) changed files carry
mutator tables. Any other extension is reported per line as
`Skipped { reason: "unsupported_file_type" }`; a supported file whose changed
line matches no mutator is `Skipped { reason: "unsupported_mutation_shape" }`.
Unsupported shapes are never silently ignored.

## Usage

```bash
# Unit + integration tests for the crate itself.
cargo test --manifest-path tools/v2-gauntlet/Cargo.toml --locked

# Self-test: proves the contract on committed fixtures. The weak fixture
# (tools/v2-gauntlet/fixtures/weak) MUST fail with a surviving mutant.
cargo run --manifest-path tools/v2-gauntlet/Cargo.toml -- --self-test

# Normal run against a workspace with a changed-line manifest.
cargo run --manifest-path tools/v2-gauntlet/Cargo.toml -- \
  --workspace path/to/crate \
  --changed path/to/changed.json \
  --seed 12648430 --order-runs 3 \
  --receipt-out /tmp/gauntlet-receipt.json
```

Changed-manifest shape: `{"files":[{"path":"src/lib.rs","lines":[5,6]}]}`
(1-based line numbers).

## Receipt

The receipt is a deterministic local JSON document (sorted, no timestamps):
workspace, seed, per-layer status, every mutant with its outcome, the list of
surviving mutants, and each recorded run order. Exit code is `1` only when a
layer is `Failed`.

## Policy

- The gauntlet is **optional** for normal book gates.
- It becomes **required only in the final release audit**, and only when its
  pinned toolchain (rustup channel pinned at the repository root, plus any
  coverage backend named in the receipt) is available; otherwise the affected
  layers report `Unproven` and the audit records them as such.
- **No CI integration.** The gauntlet never talks to GitHub Actions or any
  hosted service; receipts stay on the local machine.

## Anti-gaming constraints (enforced in code)

- Never weaken a test to make the gauntlet pass.
- Never report an unrun check as `Passed`.
- Mutators that do not match a line's syntactic shape are `Skipped` with a
  reason, never silently ignored.
- A surviving mutant is always `Failed`, never downgraded.
- Mutated sources live only in throwaway copies; the original tree is never
  modified.
