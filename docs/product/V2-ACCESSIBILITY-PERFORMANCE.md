# V2 Accessibility & Performance (CR-V2-B6-021)

## Accessibility
- Focus management + dialog traps across every new mode.
- Semantic labels, ARIA live regions, keyboard equivalents, contrast.
- Reduced-motion honored at app and output levels.

## Performance budgets
- Initial load: ≤ INITIAL_LOAD_BUDGET_MS (1500 ms).
- Interaction: ≤ INTERACTION_BUDGET_MS (100 ms).
- Memory: ≤ MEMORY_BUDGET_MB (350 MB).

## Implementation notes
- Playhead animation uses refs + rAF; React state commits at bounded UI cadence.
- Long transcripts/evidence/timelines virtualised via existing windowed chunks.
