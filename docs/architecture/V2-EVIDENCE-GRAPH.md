# V2 Hierarchical Evidence Graph (CR-V2-B3-002)

## Purpose

Freeze the on-disk contract for the **hierarchical evidence graph** that
every CutRight project builds. The graph is a content-addressed,
typed DAG of evidence nodes and edges. It is the canonical source of truth
for every "what is in this video" question the rest of the v2 system asks.

## Three Schemas

- `schemas/evidence/node.schema.v1.json` — a single evidence node.
- `schemas/evidence/edge.schema.v1.json` — a typed edge between two nodes.
- `schemas/evidence/graph.schema.v1.json` — the top-level graph bundle.

## Node Kinds

The full vocabulary is fixed in `node.schema.v1.json`. The kinds are grouped
by domain:

- **Source**: `source`, `asset`.
- **Visual**: `scene`, `shot`, `visual_event`, `frame`, `face`, `subject`,
  `pose`, `gesture`, `text_region`, `motion_region`.
- **Audio**: `audio_stream`, `speaker_turn`, `utterance`, `word`,
  `speech_region`, `music_section`, `bar`, `beat`, `transient`.
- **Editorial**: `editorial_beat`, `claim`.

Adding a new kind is a schema change AND a `V2-CRATE-DAG.md` change. No
ad-hoc extension points are exposed.

## Node Identity

A node MUST carry:

- `id` — stable, content-derived BLAKE3 hash of the canonicalised node body.
- `kind` — one of the enumerated kinds.
- `source_revision` — the project revision that materialised the node.
- `producer` — `{capability, version, parameters_hash, model_pack}`.
- `receipt` — `{receipt_id, batch_id, status}` referring to the producing
  action batch.
- `payload_hash` — BLAKE3 hash of the canonicalised payload object.

Optional:

- `range` — rational nanosecond range. Omitted only for time-less semantic
  nodes (e.g. an `editorial_beat` that aggregates beats).
- `confidence` — required for machine-produced nodes; omitted for
  analyst-authored editorial nodes.
- `payload` — bounded producer-defined object.
- `labels` — typed key/value annotations.
- `notes` — free-text; max 2048 chars; NEVER used as canonical identity or
  timing.
- `redacted` — true if the payload was removed for rights/sensitivity.

## Edges

Edges are typed and directed. The kind enumeration is exhaustive:

`contains`, `overlaps`, `supports`, `contradicts`, `derived_from`,
`same_subject`, `same_take`, `spoken_by`, `visualises`, `synchronised_with`,
`precedes`, `follows`, `references`, `anchors`, `replaces`.

- `same_subject`, `same_take`, `synchronised_with`, `overlaps` are the only
  symmetric kinds. `symmetric: true` is required.
- `derived_from` is acyclic: a node cannot derive from itself.

## Derived-Nodes Trace Invariant

Every derived node MUST trace to immutable source bytes. This is enforced by:

1. The `source_revision` field must reference a known revision.
2. Every `derived_from` edge must chain back to a `source` node within the
   same graph.
3. Every `payload_hash` must be reproducible from the producing batch's
   receipt.

Time-less semantic nodes (e.g. `editorial_beat`) trace to timed supporting
nodes via `contains` edges. A graph with an `editorial_beat` that does not
contain any timed node is invalid.

## Graph Cycles

Cycles are allowed only for the declared symmetric edge kinds. A graph
validator walks the graph and rejects any cycle that includes a non-symmetric
edge. The validator runs at every graph persistence step.

## Raw Model Prose Is Banned

The producer MUST NOT use raw model prose as canonical node identity or
timing. This is enforced by:

- `id` is a BLAKE3 hash, not a string.
- `range` is a rational nanosecond pair, not a model quote.
- `notes` is free-text and never participates in canonicalisation.

## Graph Identity

The graph is content-addressed. `graph_hash` is the BLAKE3 hash of the
canonicalised body (nodes and edges sorted by id). Two graphs with the same
hash are byte-equivalent.

## Persistence

The graph schema explicitly requires:

- `graph_id` — stable identifier.
- `project_revision` — the project revision this graph is built for.
- `source_revisions` — every revision that contributed at least one node.
- `graph_hash` — for de-duplication.
- `frozen` — true iff the graph is read-only; mutations must produce a new
  graph.

A graph is stored as a single JSON file under the project's evidence
directory. It is content-addressed by `graph_hash`. The video-evidence crate
owns the read/write implementation.

## Acceptance

- Every derived node traces to immutable source bytes.
- Time-less semantic nodes still trace to timed supporting nodes.
- Graph cycles are allowed only for declared symmetric relation types.
- Raw model prose is never canonical identity or timing.

## Future Packs

Adding new edge kinds is a schema change. Adding new node kinds is a schema
change AND a `V2-CRATE-DAG.md` change. The graph is designed to be
extensible without breaking the canonical id pattern.
