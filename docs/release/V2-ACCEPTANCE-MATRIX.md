# V2 — Release acceptance matrix and supported-target claims

Frozen by **CR-V2-B7-005**.

## Required for any `supported` claim

A target is supported ONLY if all five checks pass on a clean machine with the network blocked:

1. **Installer OK** — `cutright.install` completes without error; bundle manifest verified.
2. **Runtime OK** — `videoctl runtime resolve` succeeds for every shipped capability.
3. **Benchmark OK** — full four-lane benchmark (B4) passes within tolerance.
4. **Workflow OK** — full Studio workflow tests (B6) pass end-to-end.
5. **Security OK** — final security/privacy/licence/supply-chain audit (B7-024) clean.

Source-build / headless support is **separate** from desktop release support and may not be advertised as a desktop claim.

## Supported targets

See `config/release/targets.json`. Unsupported targets are recorded explicitly there.

## Acceptance result schema

`schemas/release/acceptance-result.schema.v1.json` — `cutright.acceptance_result/v1`. Every release produces one result per target. Evidence path is recorded.

## Claim discipline

Any marketing copy that names a target must point to a matching `acceptance_result/v1` row with all five booleans `true`.
