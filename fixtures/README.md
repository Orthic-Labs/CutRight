# CutRight fixture format

Fixtures are project packages with real source media kept outside Git. The package checked into a
test suite contains the canonical JSON artifacts and a `sources/manifest.json` entry whose `path`
points at the local media fixture. Source files are never copied into or modified by tests.

Phase 0 fixtures must include:

- `project.json`;
- `sources/manifest.json`;
- at least one word-level `analysis/transcript.json`;
- a `edit/timeline.json` with rational FPS;
- a round-trip assertion for every JSON artifact.

Synthetic JSON is suitable for contract tests. Media acceptance gates must use bridge-period footage
from Adrian's real camera, room, speech, VFR, rotation, and HDR cases.
