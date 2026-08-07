# V2 Release Candidate Report (CR-V2-B7-026)

**Release candidate id:** `v2-rc-2026-08-07`
**Generated:** 2026-08-07
**Status:** `local_release_candidate`
**Publish status:** `not_requested`
**Upload status:** `not_performed`
**Tag:** `none`
**Head:** `a9154a3` (CR-V2-B3-027)

## Build

```text
python3 scripts/release/v2-build.py --self-test --profile release --target host --out release/v2/rc
```

The build runs in `--self-test` mode for the local RC; the harness
exists for the operator to reproduce the build without any remote
service. The build script refuses to start when any environment
variable named `*TOKEN*`, `*SECRET*`, or `*KEY*` is set, so the
operator cannot accidentally embed credentials.

## Seal

```text
python3 scripts/release/v2-seal.py seal --manifest release/v2/rc/SEAL.json release/v2/rc
python3 scripts/release/v2-seal.py verify release/v2/rc
```

The seal enumerates every file in the bundle, its SHA-256 hash, and
its signature (empty for the local RC). Verification reproduces the
hashes and confirms the bundle is intact.

## Checksums

```text
python3 scripts/release/v2-seal.py checksums --out release/v2/rc/checksums.txt release/v2/rc
```

The `checksums.txt` file lists the SHA-256 of every file in the
bundle. It is itself sealed by the bundle SEAL.

## Signing status

The local RC ships **unsigned**. The signing script is held aside
for the operator's manual invocation; the script never uploads. This
keeps the local RC free of any external service contact.

## External services

None. The RC records `external_services_contacted: []`.

## Targets

| Target        | Status      | Signed | Seal                          |
|---------------|-------------|--------|-------------------------------|
| macOS-arm64   | local_only  | false  | `release/v2/rc/SEAL.json`     |
| macOS-x86_64  | local_only  | false  | `release/v2/rc/SEAL.json`     |

Windows and Linux are OUT OF SCOPE for v2.

## Manifest binding

```text
release_candidate = app + packs + source + tests + audits + acceptance
```

The RC-MANIFEST binds every artifact. The four-lane results, the
audit pack, the SBOM, the provenance graph, the disclosure, and the
clean-machine harness are all referenced.

## Artefacts

- `release/v2/RC-MANIFEST.json`
- `release/v2/rc/BUILD.json`
- `release/v2/rc/RC-MANIFEST.json`
- `release/v2/rc/SEAL.json`
- `release/v2/rc/checksums.txt`
