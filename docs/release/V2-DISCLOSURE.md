# V2 Release Disclosure (CR-V2-B7-025)

**Disclosure id:** `v2-rc-disclosure-2026-08-07`
**Generated:** 2026-08-07
**SBOM:** `release/v2/SBOM.spdx.json`
**Provenance:** `release/v2/provenance.json`
**Notices:** `release/v2/THIRD-PARTY-NOTICES.md`

## Bundled runtime components

The offline installer bundles the CutRight v2 application, the v2
capability registry, the v2 evidence graph, the v2 skill compiler, the
v2 evaluation pipeline, and the worker sandbox. Source for the
runtime is the corresponding-source bundle surfaced under
`release/v2/source/source-manifest.json`. The bundle is LGPL-2.1
compliant; LGPL-2.1 components are accompanied by the corresponding
source bundle.

## Optional packs

Capability packs, skill packs, and model packs are downloadable as
optional, signed bundles. Each pack carries its own licence row in
its descriptor. A pack without a licence disposition is **not**
shipped.

## Model licences

Models used by the installed v2 are local-first and originate from
the v2 model pack registry. Samasource/whisperx-style third-party
endpoints are not invoked by the installed product.

## Unsupported targets

* **Windows** — out of scope for v2.
* **Linux** — out of scope for v2.

The supported targets are `macOS-arm64` and `macOS-x86_64`.

## Privacy defaults

* Telemetry is **OFF** by default.
* Logs are stored locally and never transmitted.
* No cloud endpoint is contacted at runtime.
* No workspace-local path is leaked in the installer.

## Known limitations

* Four-lane benchmark metrics are **Unproven** until the corpus
  fixtures are populated (see `docs/release/V2-FOUR-LANE-RESULTS.md`).
* No supported-target claim is made for macOS-arm64 or macOS-x86_64
  until the four lanes record Pass on the RC hashes.
* The Windows and Linux installers do not exist.

## Verification

The release candidate is sealed by `release/v2/staging/SEAL.json`. The
SBOM and the provenance graph together cover every file in the
bundle. The `verify-provenance` step in `scripts/release/v2-seal.py`
cross-checks the seal against the provenance graph and the actual
bytes; any missing edge or hash mismatch is a release block.

## Licence disposition

Every materialised component in the bundle has a licence row in
`release/v2/staging/licences`. The SBOM contains a `licenseConcluded`
field for every file. Any component whose `licenseConcluded` is
`NOASSERTION` is documented in this disclosure.
