# HeardRight CutRight Patches (CR-V2-B3-008)

## Purpose

CutRight consumes the vendored HeardRight crates
(`vendor/heardright/core`, `vendor/heardright/engine`,
`vendor/heardright/platform`) and the new
`vendor/heardright/cutright-adapter` glue. The adapter removes every
discovery path the vendored crates use to find a system-installed
HeardRight. All resources resolve through the `PackResourceResolver`.

## Removed behaviour

- **Engine discovery** — `which heardright`, `~/.heardright`,
  `/Applications/HeardRight.app` are no longer inspected.
- **Installed-app paths** — `HEARDRIGHT_HOME`, `HEARDRIGHT_DATA`,
  `HEARDRIGHT_MODEL_DIR` are no longer consulted.
- **User model discovery** — `~/Library/Application Support/HeardRight/models/`
  are no longer consulted.
- **Network fallback** — no remote fetch; every model and dictionary
  byte is hash-pinned in the cap-ledger.

## Added behaviour

- `PackResourceResolver` is the only entry point for any resource.
- `AdapterIdentity` records the vendored commit and pack hashes.
- `BoundedStderr` bounds the vendored engine's stderr at 64 KiB.
- `Cancellation` is a first-class token, not a sidecar kill.
- `SessionOptions` enforces a hard timeout (default 30 s).

## Identity

The vendored commit is recorded in `runtime/manifests/speech-engine.source.json`
under `source.commit`. Replacement of any vendored crate requires a
new manifest hash and a new commit entry. The merge step
`CR-V2-B3-022` is the only step that lifts these into the
shipping manifest.

## Patch series

| Patch | Description |
|-------|-------------|
| 0001  | Replace engine discovery with `PackResourceResolver`. |
| 0002  | Replace environment-variable model paths with cap-ledger. |
| 0003  | Hook `BoundedStderr` and `Cancellation` into engine calls. |
| 0004  | Surface `AdapterIdentity` from every session entry point. |

## Acceptance

- Release code contains no HeardRight install/path environment variables.
- The vendored engine still transcribes a fixture file with WER ≤ baseline.
- The adapter identity is present in every receipt.
