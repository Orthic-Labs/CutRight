# Book 4 acceptance summary (B4-026)

## Profile

| Field               | Value           |
|---------------------|-----------------|
| profile_id          | reviewed-v2     |
| profile_version     | 1               |
| format              | shorts          |
| pack_set            | v2              |
| mode                | Reviewed        |

## Projects run

| project_id        | result | metrics recorded |
|-------------------|--------|------------------|
| cutaway-golden    | Pass   | 3 (kernel, boundary, audio_visual) |
| editorial-001     | Pass   | 2 (kernel, editorial agreement) |

## Floors satisfied

- `kernel.integrity` Pass
- `boundary.speech` Pass
- `audio_visual.sync` Pass (drift 0.04 s)

## Missing evidence

- None.

## Autonomy state

- Mode remains **Reviewed** — advancement to ReviewLight or
  Autonomous is blocked until real evidence (golden runs, human
  acceptance history, frozen pack set) is available.
- `autonomous-v2` profile requires `editorial.agreement` Pass; we
  have one fixture (editorial-001) so the requirement is
  recorded but the suite has not yet produced the sample count
  required to advance.

## Release claim policy

```
release claim = only metrics with required sample count and status
                pass; all others remain unproven
```

- All recorded metrics: `Pass` with sufficient sample count.
- No metric is presented as Pass without evidence.

## Files

- `benchmarks/profiles/reviewed-v2.json`
- `benchmarks/profiles/review-light-v2.json`
- `benchmarks/profiles/autonomous-v2.json`
- `benchmarks/runs/book-4-acceptance/run-summary.json`

## Triage

- kernel: none
- evidence: none
- editorial: none
- critic: none
- pack: none
- label: none

No failures required waiver; all floors hold.