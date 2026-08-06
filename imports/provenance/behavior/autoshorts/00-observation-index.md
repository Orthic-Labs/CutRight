# Clean-room observation index — short-form clip pipeline product

- **Disposition:** clean_room_behavior (observation only; no source copied)
- **Observed at revision:** `f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b`
- **Observation date:** 2026-08-07
- **Machine-readable record:** `imports/v2/clean-room/autoshorts.json`

These documents describe only externally observable product behavior of a
reference local-first desktop tool that turns long-form recordings into
ranked vertical short-form clip candidates. They deliberately avoid any
source-shaped identifiers (no module, class, function, table, or storage
key names from the observed product). CutRight implementers must work from
these specifications alone.

## Adopted behaviors

| Spec | Behavior |
| --- | --- |
| [01-project-lifecycle.md](01-project-lifecycle.md) | Project library with create/open/rename/delete and per-project status |
| [02-onboarding-and-runtime-readiness.md](02-onboarding-and-runtime-readiness.md) | First-launch setup and model/runtime readiness with visible progress |
| [03-one-click-pipeline.md](03-one-click-pipeline.md) | Single automated chain: ingest → transcript → analysis → ranked moments |
| [04-candidate-cards.md](04-candidate-cards.md) | Ranked candidate cards with selection, naming, and per-card actions |
| [05-progress-recovery-export.md](05-progress-recovery-export.md) | Live progress, relaunch recovery, and final clip export |

## Observed behaviors explicitly REJECTED by CutRight

Each rejected behavior is recorded with the CutRight constraint that
replaces it:

1. **Browser-local secret storage** — the observed product persists user
   API secrets in web-view local storage. CutRight never stores secrets in
   UI state; offline-first means no third-party secrets are needed at all.
2. **Center-crop-only reframing** — the observed product converts wide
   footage to vertical by cropping around the frame center. CutRight plans
   reframing as a typed stage with evidence-backed subject placement, not a
   fixed center crop.
3. **Model output timestamps taken as canonical** — the observed product
   feeds analysis-model timestamp guesses straight into cuts. CutRight
   reconciles every proposed boundary against transcript/decode evidence
   before a cut is committed.
4. **Database rows as the canonical truth** — the observed product treats
   its local database as the source of truth for projects and results.
   CutRight keeps the project package (files on disk) canonical and treats
   any index as disposable/rebuildable.
5. **Cloud-first defaults** — the observed product defaults to, and
   markets, paid cloud transcription/analysis providers. CutRight defaults
   to fully local stages; cloud is never a default path.
6. **Monolithic single-file UI** — the observed product implements its
   entire interface in one large file. CutRight composes the studio from
   registered capability surfaces, not a monolith.
