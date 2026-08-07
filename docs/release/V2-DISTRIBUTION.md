# V2 — Installer, bundle, pack, update, and rollback architecture

Frozen by **CR-V2-B7-003**.

## Base app + offline bundle

The installed product is `base_app` plus a complete offline bundle. An optional separately-signed quality pack may be split off for download-size reasons but is still part of one CutRight product.

## Target-specific installer contents

Per-target installers (macOS DMG, Windows MSI, Linux AppImage/DEB, source tarball) carry:

- base app binaries
- runtime packs with their locks, notices, signatures
- corresponding-source archive for any LGPL component
- sample projects
- repair payload and checksums
- offline documentation

## Local update + rollback

Local update/rollback is verified without requiring a hosted updater for acceptance. The update manifest (`cutright.update_manifest/v1`) explicitly sets `hosted_updater_required: false`.

## Build, sign, package, seal, upload are separate

Each is a distinct local action. Upload is outside this dispatch — the build/seal stops at a local RC. Acceptance verifies the RC, not an upload.

## Schema contracts

- `schemas/release/bundle-manifest.schema.v1.json` — `cutright.bundle_manifest/v1`
- `schemas/release/update-manifest.schema.v1.json` — `cutright.update_manifest/v1`
- `schemas/release/rollback.schema.v1.json` — `cutright.rollback/v1`
