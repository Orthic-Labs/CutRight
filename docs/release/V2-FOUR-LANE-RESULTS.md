# V2 Four-Lane Benchmark Results (CR-V2-B7-023)

**Run id:** `v2-rc-four-lane-2026-08-07`
**Generated:** 2026-08-07
**Release candidate:** `release/v2/staging` (SEAL at `release/v2/staging/SEAL.json`)
**Corpus:** `benchmarks/corpus/manifest.json` (5 items, all `missing_fixture: true`)
**Profile:** `reviewed-v2`

## Honest statement

All five corpus items (`golden-recorded-001`, `golden-repurpose-001`,
`golden-explainer-001`, `golden-anchored-001`, `golden-adversarial-001`) carry
`missing_fixture: true`. The benchmark runner therefore reports **Unproven**
for every metric on every lane. Per dispatch policy, **no synthetic result
is promoted to Pass** and **no supported-target claim is made** on the
strength of this run.

## Per-lane status

| Lane            | Project                  | Split       | Metrics        | Kernel/safety | Studio | Clean-machine |
|-----------------|--------------------------|-------------|----------------|---------------|--------|---------------|
| recorded        | golden-recorded-001      | train       | 7 × Unproven   | pass          | pass   | pass (sample) |
| repurpose       | golden-repurpose-001     | train       | 7 × Unproven   | pass          | pass   | pass (sample) |
| explainer       | golden-explainer-001     | calibration | 7 × Unproven   | pass          | pass   | pass (sample) |
| anchored_creative | golden-anchored-001    | test        | 7 × Unproven   | pass          | pass   | pass (sample) |
| adversarial     | golden-adversarial-001   | test        | 7 × Unproven   | pass          | pass   | pass (sample) |

Total: 35 metrics, 0 Pass, 0 Fail, 35 Unproven.

## Supported-target claims

| Target        | supported_target_claim |
|---------------|------------------------|
| macOS-arm64   | **false**              |
| macOS-x86_64  | **false**              |

Until the four lanes record Pass on the RC hashes, no supported target is
claimed. This decision is policy-driven, not a failure of the kernel or
the offline pipeline.

## Kernel and safety floors

| Floor                       | Status |
|-----------------------------|--------|
| kernel_integrity            | pass   |
| sandbox_resource_limits     | pass   |
| network_denial              | pass   |
| secret_scan                 | pass   |
| tamper_detection            | pass   |
| supply_chain_audit          | pass   |

## Studio workflows

```text
IngestMode    pass
EditMode      pass
FinishMode    pass
MigrationMode pass
```

## Clean-machine harness

- Harness: `scripts/qa/v2-clean-machine/run.py`
- Host: `release/v2/clean-machine-host.json`
- 4 rights-cleared samples passed offline.

## Acceptance

- **Release blocking:** `false`
- **Block reason:** Four-lane benchmark metrics are Unproven due to missing
  corpus fixtures. Supported-target claims require at least the four lanes
  to record Pass on the RC hashes; absent that, no claim is made.
- **Path to unblocked:** populate corpus fixtures with rights-cleared
  sources, then re-run the four-lane benchmark against the RC hashes and
  upgrade the lane `support_target_claim` flags individually.

## Artefacts

- `benchmarks/runs/v2-release-candidate/four-lane-results.json`
- `benchmarks/runs/v2-release-candidate/run-manifest.json`
- `release/v2/acceptance/v2-rc-acceptance.json`
