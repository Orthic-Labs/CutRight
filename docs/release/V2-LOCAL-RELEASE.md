# V2 Local Release Procedure

The v2 release is **local**: it never uploads, publishes or tags a remote
service. The full chain is reproducible from the working tree at the
candidate commit, with no network call.

## Stages

1. `v2-build.py` produces a deterministic build artefact under the
   configured `--out` root. The script refuses to proceed when any
   `*TOKEN*` / `*SECRET*` / `*KEY*` environment variable is set.
2. `v2-sign.py` produces a local signature for every released file. The
   signature value is the SHA-256 of the file. The script supports
   `--unsigned-fixture` for tests but **unsigned artefacts cannot
   masquerade as signed**: the manifest receives `"signed": false`.
3. `v2-seal.py` produces the SEAL.json and (later) verifies it. The script
   supports `--seal`, `--verify`, `--verify-provenance` and `--checksums`.

## Acceptance commands

```bash
python3 scripts/release/v2-build.py --help
python3 scripts/release/v2-sign.py --self-test --unsigned-fixture
python3 scripts/release/v2-seal.py --self-test
```

## Upload is intentionally absent

This dispatch does not authorize an upload step. The release candidate
sits under `release/v2/rc/` on the local checkout. A future hosting
project that needs to host the bundle will use these scripts as the source
of truth and add its own upload step.

## Tool flags

`--self-test` is a developer convenience used by CI-free acceptance
fixtures; it does not read or write secrets.
