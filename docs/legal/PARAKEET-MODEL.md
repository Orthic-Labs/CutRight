# Parakeet Model Pack (CR-V2-B3-009)

## Source

- Upstream: NVIDIA NeMo Parakeet (open-source ASR model)
- Pin: `<model_kind> <model_size>` selected from the Book 1 HeardRight asset
  ledger (`imports/v2/heardright-assets.json`).
- Licence: determined by the cap-ledger row; the build aborts on
  unresolved licence.

## Why offline

The product may never download the model at runtime. The build script
(`scripts/runtime/build-parakeet-pack.py`) validates the asset
contract; the operator copy is a manual, offline-only step that
replaces the `unresolved` fields in `runtime/manifests/parakeet.model.json`
with real hashes and licences.

## Required assets

```text
encoder
decoder
joiner
tokenizer
vocabulary
```

Each asset must point at a hash-pinned file in the cap-ledger.

## Build steps

1. Inspect the cap-ledger row for the chosen Parakeet model variant.
2. Copy each asset into `runtime/models/parakeet/<target>/<arch>/`.
3. Update `runtime/manifests/parakeet.model.json` with measured hashes
   and the licence disposition.
4. Run the timed-words fixtures under `fixtures/runtime/parakeet/`.
5. Emit the signable speech-pack fragment through the `signature`
   pipeline.

## Acceptance

- No model byte is fetched from a mutable URL.
- The model identity is exact and reproducible.
- Timed words are native; segment-only output fails qualification.
