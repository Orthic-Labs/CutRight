# CutRight v2 Benchmark & Editorial DAG

This document is the binding release DAG for Book 4. It complements the
interface-freeze document with the run-order and audit chokepoints the
integration lane must traverse.

## 1. Lane ownership

```text
lane_a: crates/video-benchmarks/**
lane_b: crates/video-editorial/src/deterministic/**
lane_c: crates/video-editorial/src/narrative/**
```

Lanes A, B, C are independent inside Book 4. The four-lane integration
line (serial tasks 022–027) owns the workspace boundary and the audit
deliverables.

## 2. Required phrases

The release DAG must contain the following phrases (checked by
`scripts/architecture/check_crate_dag.py`):

```text
contracts
Lane A
Lane B
Lane C
merge
four-lane
audit
SBOM
release candidate
final gate
```

## 3. Lane contracts

Each lane's contracts cover a fixed set of artifacts:

- Lane A: benchmark evaluator interfaces, runner, report, profile, and
  benchmark run artefacts. Lane A never writes production project state.
- Lane B: deterministic candidates, beats, takes, scoring, faults,
  disfluency, dead-air, and boundary consensus. Lane B is pure
  deterministic logic.
- Lane C: narrative arcs, Director request, ordering, hook/payoff/CTA,
  truthfulness, shorts, confidence, critic, and the façade in
  `crates/video-editorial/src/engine.rs`.

## 4. Merge order

Lanes A, B, C are merged into the workspace in fixed order. The merge
task (021) wires the `EditorialEngine` façade and produces a merge
receipt in `docs/dispatch/v2/book-4/merge-receipt.md`.

## 5. Audit and SBOM

The four-lane integration owner produces an audit chokepoint and a
SBOM derivation task. The audit runs after the merge is verified and
before the release candidate.

## 6. Final gate

The final gate is run by the Book 4 close task (027) and is the only
gate that may mark Book 4 as closed.
