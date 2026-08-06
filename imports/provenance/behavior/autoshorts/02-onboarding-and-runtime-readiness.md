# Observed behavior: onboarding and model/runtime readiness

Behavior observed at the pinned revision of the reference product.

## Observable behavior

- On first launch, a setup wizard appears before any project work; the
  wizard offers a choice between a fully local workflow and a hosted-API
  workflow.
- For the local workflow, the wizard lets the user pick one language-model
  size from a small menu of named local model options.
- If the chosen local model is not yet present, the product offers to
  download it in place, showing live status text and a percentage progress
  bar until completion.
- The product checks whether the local model runtime is reachable before
  running analysis, and surfaces a plain-language error telling the user
  to start the runtime when it is not.
- Required external media tooling (decoder/encoder binaries) is detected
  from the system path; missing tooling produces a visible warning in the
  interface rather than a silent failure.
- Setup choices persist across relaunches; a dedicated settings surface
  allows changing engines/models later, and a reset action returns the app
  to the first-launch wizard state.

## Implementation-neutral constraints adopted by CutRight

- Readiness checks are explicit, named, and visible: a stage never starts
  on an unverified runtime.
- Long downloads report typed progress (connecting / receiving / complete)
  rather than an indeterminate spinner.
- CutRight rejects the hosted-API workflow branch entirely: onboarding is
  local-only, and no user secrets are collected or stored (the observed
  product's secret collection is a rejected behavior, see index).

## Acceptance test statements

1. Given the local model runtime is not running, when analysis is
   requested, then the surface shows a plain-language "start the runtime"
   error and no stage runs.
2. When a model download is in flight, then the surface shows monotonic
   percentage progress and a final completion state.
3. Given required media tooling is missing from the system path, then a
   visible warning is shown before any pipeline stage is offered.
4. When the user completes onboarding and relaunches, then the wizard does
   not reappear and the chosen engine selection is preserved.
