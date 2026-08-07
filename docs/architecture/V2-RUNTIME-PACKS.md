# V2 Runtime Packs (CR-V2-B3-001)

## Purpose

Freeze the on-disk contract for every **CutRight runtime pack**: how a pack is
identified, versioned, target-locked, hashed, signed, measured, and declared
compatible. Replacement for every system-tool and sibling-app dependency that
used to live outside the repo.

Two artefacts carry the contract:

1. `schemas/runtime/pack-manifest.schema.v1.json` — the **manifest** body that
   describes every file inside a pack, signed by the publisher.
2. `schemas/runtime/pack-lock.schema.v1.json` — the **release lock** that ships
   a pack. Locks are immutable, signed, and ship measurement evidence.
3. `schemas/runtime/pack-signature.schema.v1.json` — the **detached signature**
   that covers the manifest body, every file entry, every compatibility
   declaration, and the manifest hash.

Together these three schemas replace every ad-hoc runtime resolution path.

## Identifier and Version

A pack id is a stable, dotted lowercase string that mirrors the capability
vocabulary frozen in `V2-CAPABILITY-ACTION-CONTRACT.md`:

```
^[a-z][a-z0-9_.-]+$
```

Examples: `media.ffmpeg`, `speech.parakeet`, `inference.llama-cpp`,
`voice.kokoro`. The id MUST NOT change across versions.

Versions are strict semver with an optional pre-release suffix:

```
^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9.-]+)?$
```

Semantic version alone is **not** a compatibility declaration. The
`compatibility` block is the source of truth.

## Target Triple

Every pack targets a triple (`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`,
…). A release lock pins one triple, one arch. Universal packs are allowed but
only as a derived lock that combines per-arch locks.

## Files and Hashes

Every file entry in the manifest carries:

- `path` — relative path inside the pack archive (forward slashes, no leading
  slash).
- `sha256` — SHA-256 of the file bytes.
- `blake3` — BLAKE3 of the file bytes.
- `size_bytes` — exact byte count.
- `mode` — POSIX mode if executable.
- `kind` — one of `binary`, `model`, `config`, `licence`, `manifest`,
  `signature`.

Empty or zero hashes are invalid in release locks. The manifest body
**without** the `manifest_hash` field is canonicalised (sorted keys) and hashed
with BLAKE3 to produce `manifest_hash`.

## Requirements

The `requirements` block declares minimum disk, memory, CPU, and the
availability of AVX2/AVX-512/NEON/GPU. Packs that require SIMD features MUST
declare them; the runtime refuses to load a pack on a host that lacks them.

Peer packs are declared with a version range. Ranges follow the same grammar
as Cargo:
- `^1.2.3` — compatible release
- `~1.2.3` — patch-level only
- `1.2.3` — exact
- `1.2.3-2.0.0` — inclusive range

## Capabilities

Every pack exports a list of capabilities it implements, each with a name,
semver version, and a BLAKE3 hash of the canonicalised parameters object. The
hash binds the capability's effective parameters so the registry can detect
silently-changed parameters.

## Compatibility

`compatibility` is the canonical declaration of fit. It is broken into four
parts:

- `application` — `min`, `max`, and explicit `excludes` against the running
  CutRight version.
- `project` — minimum revision id and a list of forbidden target ids.
- `benchmark_profile` — list of `{profile, floor}` pairs that the pack must
  meet before autonomy is enabled.
- `peer_packs` — explicit boolean compatibility flags against other packs.

Compatibility is **explicit**, not inferred from semver. A release lock that
omits a `peer_packs` declaration for a known peer is invalid.

## Signature

`signature` is a detached `pack_signature/v1` object. It must cover:

- the manifest hash (`digest`),
- every file entry (`covers.files` = true),
- the compatibility declaration (`covers.compatibility` = true).

Algorithms are limited to:

- `blake3+ed25519` — preferred for performance.
- `sha256+rsa-pss-3072` — allowed for legal/compatibility reasons.

Any other algorithm is rejected at load time.

## Lock vs Manifest

The manifest is the unsigned description of a pack. The lock is the immutable
release artefact:

- Locks cannot contain mutable URLs (`mirror_urls` is `maxItems: 0`).
- Locks cannot contain empty measurements (the `anyOf` clause forces at least
  one of `cold_start_ms`, `warm_start_ms`, `tokens_per_sec`, `rtf`).
- Locks carry `released_at_ns` and `released_by` so the provenance is
  reproducible.

A non-release fixture manifest is allowed to omit some fields for tests;
release locks must be fully populated.

## Verification Flow

1. Parse the lock against `pack-lock.schema.v1.json`. Reject any unknown
   field.
2. Verify the manifest hash: parse the manifest, drop the `manifest_hash`
   field, canonicalise, BLAKE3-hash, compare.
3. Verify the signature: digest matches the manifest hash; verify with the
   public key named in `signature.key_id`.
4. For every `files[]` entry, recompute SHA-256 and BLAKE3; compare to the
   stored values.
5. Verify compatibility against the current `application.min/max/excludes`
   and the host's measured runtime (SIMD, memory, disk).
6. Verify the requirements block against the host's measured capabilities.
7. Verify peer-pack compatibility against the resolved peer manifests.

If any step fails the pack is refused. The auditor must surface the failing
step in the receipt.

## Acceptance

- Release locks cannot contain mutable URLs, missing measurements, or
  unresolved licences.
- Compatibility is explicit rather than inferred from semantic version alone.
- Signature covers every file entry and compatibility declaration.
- No field is optional-on-release; the schema's `required` array forces every
  release lock to be complete.

## Future Packs

The schemas are designed to absorb more pack kinds without a schema bump.
Adding a new kind is a `V2-CRATE-DAG.md` change, not a schema change.
