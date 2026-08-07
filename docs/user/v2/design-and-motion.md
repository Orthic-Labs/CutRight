# Design and motion

The native render graph is the only shipping render runtime. It
handles text, vector, image, mask, transition and procedural motion
without a Node, Chromium or Remotion runtime.

## Design

* Lower-thirds, stat counters, quote cards and identity cards come
  from the bundled `creative` pack. Every effect id is signed and
  every effect has a golden fixture that proves the visual
  requirement.
* Custom designs start from a template. Templates are bound to a
  skill closure and a set of immutable revisions; a new revision is
  required for every change.

## Motion

* Procedural motion is the only kind of motion that does not require
  external media. The motion pack ships a small library of
  deterministic motion curves.
* B-roll and stock footage are loaded from the active pack set.
  Network fetch is **off**. A new B-roll must be installed by the
  operator before it is available.

## Reduced motion

Reduced motion is a first-class concern. The native renderer honours
the `reduced-motion` preference and falls back to static variants
for any motion that the user has declined.
