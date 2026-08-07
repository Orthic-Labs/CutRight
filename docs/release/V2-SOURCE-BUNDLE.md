# V2 Source Bundle

The v2 source distribution is the working tree at the exact release commit.
It is recorded in a single `source-manifest.json` that the release pipeline
seals, verifies, and binds to the offline installer manifest.

## Shape

```text
source bundle = git tree at <HEAD> + recorded file hashes + counted subtree sizes
```

The bundle is **manifest-only** by design:

* the working tree already lives in git, so copying bytes is redundant;
* `source-manifest.json` records the exact `HEAD` commit, the SHA-256 of the
  top-level files that gate the build (`AGENTS.md`, `LICENSE`, `Cargo.toml`,
  `Cargo.lock`, `package.json`, `pnpm-lock.yaml`, `tsconfig.json`,
  `vite.config.ts`, `README.md`, `CONTRIBUTING.md`), and per-subtree file
  counts for the protected directories;
* the offline installer manifest references the same `HEAD` and a sealed
  file list, so the two manifests must agree.

## FFmpeg corresponding source

The Media Kernel uses FFmpeg 8.1 built with an LGPL-2.1-or-later
configuration. The corresponding source obligation is fulfilled separately:

* a vendored source is shipped under `vendor/ffmpeg` (or a local path
  passed via `--ffmpeg-corresponding`);
* the offline installer manifest records the path, SHA-256 and size of
  the FFmpeg corresponding source;
* the same FFmpeg row appears in `release/v2/THIRD-PARTY-NOTICES.md`
  with the exact source revision, configure line and licence.

## Excluded categories

The bundle never carries:

* `.git` (history is recovered from the `HEAD` recorded in the manifest);
* `target`, `node_modules`, `.venv`, `.cache` (build outputs);
* `private_benchmark_media` (rights-restricted benchmark sources);
* `credentials`, `secrets` (operator-controlled material);
* `model_bytes_not_redistributable` (model packs shipped under their own
  signature manifest, not as part of the source distribution);
* `workspace_only` provenance (per-workspace fixtures and skill-closure
  rows that are not part of the public release).

`private_data_removed: true` is asserted in the manifest; the absence of
these fragments is verified by the same `_count_files` walk that records
the protected subtree counts.

## External runtime dependencies

The `external_runtime_dependencies` list is empty. All shipped models
(Parakeet TDT, Qwen3-4B, Qwen3-VL-4B-Instruct, Silero VAD, Kokoro-82M)
and runtimes (llama.cpp, whisper.cpp, FFmpeg) are vendored into signed
packs and resolve through the active pack lock, never through a
runtime `PATH` lookup or a hosted URL.

## Commands

```bash
python3 scripts/release/v2-source-bundle.py --target host --out release/v2/source
python3 scripts/release/v2-seal.py seal --manifest release/v2/source/SEAL.json release/v2/source
python3 scripts/release/v2-seal.py --verify release/v2/source
```

## Acceptance

* `source-manifest.json` is reproducible: rerunning the script with the
  same `HEAD` produces the same hashes and counts;
* the offline installer manifest (`release/v2/bundle-manifest.json`)
  records the same `HEAD` and a `source_bundle_path` row that names this
  directory;
* the FFmpeg corresponding source, when supplied, is verified in the
  same seal pass;
* no private benchmark media, credentials, caches or model bytes are
  included; the manifest's `private_data_removed` flag is `true`.
