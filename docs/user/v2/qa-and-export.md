# QA and export

Every version passes through the deterministic QA floors before it
reaches the user.

* **Speech preservation**: timed words are preserved across cuts.
* **Audio/video sync**: the audio timeline matches the video
  timeline within tolerance.
* **Caption coverage**: every captioned word is visible for at
  least one frame at the right time.
* **Effect identity**: every effect renders as its golden fixture.
* **Identity**: brand identity and graphic identity match the
  project profile.

A version that fails any floor lands in `needs_review` and never in
`ready`. The reviewer is shown the failing floor and the affected
segments.

## Export

Export produces the final media for the target platform. The export
pipeline uses the same native render graph as the version preview
and writes the file under the operator's chosen output directory.
The output file is hashed and bound to the source project revision;
the SHA-256 is recorded in the project's receipts.
