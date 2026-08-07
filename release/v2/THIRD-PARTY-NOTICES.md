# CutRight v2 — Third-Party Notices

The CutRight v2 release candidate bundles runtime components listed
under `release/v2/staging/licences/DEPENDENCY-LICENSES`. Model packs,
fonts, voices, templates, SFX, and sample assets are listed under
`release/v2/audit/third-party-assets.json`.

## Bundled runtime components

* Rust standard library and Cargo registry crates — see
  `release/v2/staging/licences/DEPENDENCY-LICENSES`.
* Tauri runtime — MIT.
* React / Vite build-time dependencies — MIT.
* BLAKE3 — CC0-1.0.

## Optional packs

Capability packs, skill packs, and model packs are downloadable as
optional, signed bundles. Each pack carries its own licence row inside
its descriptor (`pack.licence_row`).

## Unsupported targets

Windows and Linux installers are OUT OF SCOPE for v2. The RC ships
`macOS-arm64` and `macOS-x86_64` only.

## Privacy defaults

Telemetry is OFF by default. The privacy-safe logging surfaces
diagnostics locally and never transmits them.

## Known limitations

See `docs/release/V2-DISCLOSURE.md` for the full disclosure.
