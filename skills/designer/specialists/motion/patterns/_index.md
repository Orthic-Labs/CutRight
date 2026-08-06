# Motion Patterns — Index

44 patterns across 9 categories. Each pattern describes intent + defaults, and each category file
ends with a **Canonical code** section — copy those working values instead of re-deriving them.
For engine choice per scene, see `stack.md`.

## Loading

Read `_index.md` (this file) to find the right pattern. Then load only the relevant category file. 1 category per motion task, ~3-6KB loaded.

## Categories

| File | Patterns | Use for |
|---|---|---|
| [`entrance.md`](./entrance.md) | fade-in, slide-up, scale-up, stagger-children, mask-reveal, type-on | Elements appearing |
| [`exit.md`](./exit.md) | fade-out, slide-down-exit, scale-down, mask-collapse | Elements leaving |
| [`spatial.md`](./spatial.md) | parallax-scroll, scroll-progress, pinned-section, scrubbed-timeline, pinned-storytelling | Scroll-linked, scroll-driven, scroll-locked |
| [`attention.md`](./attention.md) | pulse, glow, count-up, badge-ping | Drawing focus |
| [`layout.md`](./layout.md) | shared-layout-morph, list-reorder, accordion-height, tabs-indicator | Position/size changes, shared transitions |
| [`gesture.md`](./gesture.md) | drag, swipe, magnetic-cursor, hover-lift, pinch | User-driven motion |
| [`state.md`](./state.md) | modal-open, drawer-slide, toggle-flip, popover-origin, toast-slide | UI state transitions |
| [`continuous.md`](./continuous.md) | spinner, marquee, skeleton-shimmer | Loops and loading states |
| [`showpiece.md`](./showpiece.md) | pin-and-scrub, parallax-layers, horizontal-scroll-section, scroll-scrub-text-fill, load-choreography, smooth-scroll, magnetic-hover, marquee | **Showpiece register ONLY** (SKILL.md §Registers) — immersive marketing/campaign surfaces |

## Picking a pattern

1. What's the **intent**? (enter, exit, attention, state change, etc.)
2. Load the matching category file.
3. Within the file, find the pattern whose `Use when` / `Avoid when` matches your case.
4. Apply the pattern's defaults (timing, easing, distance from `principles.md` §7).
5. Choose engine per scene from `stack.md`.

## When no pattern fits

If the category files don't have what you need:
- It might be an anti-pattern (most "creative" animations fall here).
- It might be a designer-authored asset (Rive, Lottie — see `stack.md`).
- It might be a 3D / WebGL effect (R3F — see `stack.md`).
- It might require inventing a new pattern. Document it in `motion-plan.md` and propose adding it to the right category file.

## Anti-patterns (do not use by default)

- bounce, shake, flash, jello, wobble, tada — only in narrow contexts
- elastic, back, easeOutBounce as defaults — feels playful/playground, not premium
- rotating elements on hover "because cool" — almost never earned
- continuous background animation with no semantic purpose
- 3+ different motion languages on one page

These are cut from the 96 motion-vocabulary terms for the reasons in upstream `docs/ARCHITECTURE-MOTION.md` §5.2 (not vendored into the CutRight corpus).