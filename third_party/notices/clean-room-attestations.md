# Clean-Room Attestation Notices

Created by CR-V2-B1-022. These are **attestation notices, not source
notices**: the clean-room sources below have `copy_source: false` in
`imports/v2/source-corpus.json`, so no upstream bytes exist in the CutRight
tree and the frozen THIRD_PARTY.yml source-notice schema does not apply.
Entry kind: clean-room (template in `third_party/README.md` §3.5).

Machine-readable attestations live in the ledger's `clean_room` blocks
(`imports/v2/dispositions.json`) and in the observation specs
`imports/v2/clean-room/autoshorts.json` (CR-V2-B1-016) and
`imports/v2/clean-room/palmier.json` (CR-V2-B1-017).

## autoshorts

- source_id: `autoshorts`
- upstream: https://github.com/JayWebtech/autoshorts
- observed at revision: `f17b04cdd97ef65c32b81b31b36bb6eb5d013d5b` (observed only, never copied)
- upstream licence status: **no declared licence** — recorded as observed
  fact; CutRight claims no licence grant from upstream.
- observed behavior: project library, onboarding, model/runtime readiness,
  one-click pipeline, candidate cards, progress reporting, recovery and
  export behavior, as written in implementation-neutral observation notes.
- implementer separation: behavior is specified from observation notes only;
  the implementing agent never opens AutoShorts source while writing
  CutRight code.
- no-copy attestation: no AutoShorts source, asset, or configuration file is
  copied into the CutRight tree; the transitive closure scanner
  (CR-V2-B1-004) rejects any such path.
- rejected behaviors (explicitly not reproduced): browser-local API keys,
  center crop, direct model timestamps, database as canonical truth,
  cloud-first defaults, monolithic UI.

## palmier-pro

- source_id: `palmier-pro`
- upstream: https://github.com/palmier-io/palmier-pro
- observed at revision: `397b82e64093f986cbabd89f1a1c93812ff546c2` (observed only, never copied)
- upstream licence status: **GPL-3.0** — GPL-3.0 source must never be copied
  into the MIT-licensed CutRight product, and none was.
- observed behavior: typed editing actions, stable element IDs, composited
  inspection, undo, variants, job management, and skill catalog behavior as
  described by public documentation and observable behavior.
- implementer separation: GPL-3.0 Swift source is never read during
  implementation of the corresponding Rust surfaces; design flows only from
  written behavior specifications. No Swift declarations, descriptions,
  schemas, comments, or implementation structure were copied.
- no-copy attestation: no Palmier Pro source, binary, or asset is copied
  into the MIT-licensed CutRight product; the closure scanner enforces this
  boundary.

## Honesty statement

No upstream licence is claimed, waived, or implied by either attestation.
AutoShorts declares no licence; Palmier Pro is GPL-3.0. CutRight's right to
the corresponding implementations rests solely on clean-room reimplementation
from written behavior observations, with observer/implementer separation
recorded above and enforced by the closure scanner.
