# Observed behavior: ranked candidate cards

Behavior observed at the pinned revision of the reference product.

## Observable behavior

- After the analysis stage, the project view lists candidate moments as
  cards ordered by their rank.
- Each card shows the moment's title/hook text, its score or rank, and a
  short reason the analysis gave for choosing it.
- Each card has an independent on/off selection control; a header control
  reports how many cards are currently selected.
- The user can give any candidate a custom display name that persists.
- Each card offers a per-card action to produce its vertical clip right
  away, independent of the other cards.
- A bulk action produces vertical clips for every currently selected card,
  one at a time, showing which card is being worked on.
- Once a card's clip exists, the card shows that the clip is ready and
  offers a way to view/open it; caption availability is shown separately
  from clip availability.

## Implementation-neutral constraints adopted by CutRight

- Selection is per-candidate and durable; the set of selected candidates
  is a user decision the pipeline must honor downstream.
- Per-card and bulk render actions are both available; bulk is sequential,
  never parallel-without-bound.
- Custom names are user data and must survive relaunch.

## Acceptance test statements

1. When analysis completes, then the card list order matches the rank
   order and every card shows title, rank/score, and reason.
2. When the user toggles selection on three cards, then the header count
   reads three and the selection survives a relaunch.
3. When the user renames a candidate, then the new name persists across
   relaunch and is used on the exported clip's file name.
4. When the bulk render action runs over selected cards, then exactly one
   card is marked "working" at a time and each finishes with a ready state
   or a visible failure.
5. When a card's clip is ready, then opening it from the card plays the
   vertical clip without re-rendering.
