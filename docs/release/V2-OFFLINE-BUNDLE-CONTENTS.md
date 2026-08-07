# V2 Offline Bundle Contents

The v2 offline bundle is the full set of bytes that lands on a target
machine. It is self-describing and verifiable without contacting any
external service.

## Top-level layout

| Path                         | Contents                                              |
| ---------------------------- | ----------------------------------------------------- |
| `app/`                       | Target-specific Tauri app (the Studio binary)         |
| `packs/`                     | Creator/runtime packs, with `packs.lock.json`         |
| `repair/`                    | Repair payload: app + packs only                      |
| `licences/`                  | Cumulative third-party licence notices               |
| `corresponding-source/`      | Reciprocal source (LGPL / FFmpeg corresponding source) |
| `samples/`                   | Rights-cleared sample projects                         |
| `checksums/`                 | SHA256SUMS for every file in the bundle               |
| `signatures/`                | Local signature manifest per group                    |

The accepted install-time requirements are:

* no internet access at install time;
* no download of a browser, model, FFmpeg, Python, Node, Ollama,
  HeardRight, CodeRight or workspace skill;
* no global PATH mutation;
* no per-user telemetry enabled by default.

## Bundling commands

```bash
python3 scripts/release/v2-assemble-offline.py --target host --staging release/v2/staging
python3 scripts/release/v2-seal.py seal --manifest release/v2/staging/SEAL.json release/v2/staging
python3 scripts/release/v2-seal.py --verify release/v2/staging
```

The manifest `release/v2/bundle-manifest.json` enumerates the layout and
file hashes. The script never makes a network call.
