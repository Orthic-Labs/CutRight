# Palmier observed behavior: rejected behaviors

Behaviors observed at the pinned revision that CutRight deliberately does NOT
adopt. Same observation provenance as 01-project-behavior.md.

## PALM-B-REJECT-001 — Hosted generative providers

- Observed: image/video/audio generation through hosted model providers
  requiring login, subscription, and network calls.
- Rejected: CutRight v2 ships no hosted generation. Generation runs only on
  local, bundled, licence-closed runtimes inside signed runtime packs. The
  variant-management behavior (PALM-B-VARIANT-001) is kept; the provider
  surface is not.

## PALM-B-REJECT-002 — Account, telemetry, and subscription surfaces

- Observed: account/login flows, subscription gating, and telemetry
  collection.
- Rejected: CutRight v2 has no account system, no telemetry, and no feature
  gating in the editing surface.

## PALM-B-REJECT-003 — Network-exposed control endpoint as the agent boundary

- Observed: the editor exposes a local HTTP control endpoint for external
  agents, including one-click client registration.
- Rejected: CutRight agents operate through in-process typed CutRight
  actions and capability-registry contracts, not a network protocol surface.
  If an external control boundary is ever added, it is a separate, explicitly
  scoped book.

## PALM-B-REJECT-004 — Closed-source processing components

- Observed: part of the generative pipeline is closed source.
- Rejected: every CutRight runtime component is source-available and
  licence-closed in the disposition ledger; nothing ships without a licence
  row.

## PALM-B-REJECT-005 — Platform-pinned UI framework assumptions

- Observed: editing UI assumptions tied to one OS release line and specific
  UI frameworks.
- Rejected: CutRight behavior specs are UI-framework neutral; the v2
  architecture owns presentation. Only domain behaviors above are adopted.
