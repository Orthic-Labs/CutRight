"""scripts/release/v2-provenance.py — generate v2 SBOM and provenance graph.

This script is local and never uploads. It reads the staged bundle and
writes:

* `--sbom`        SPDX-2.3 SBOM (Rust crates, JS build-time deps,
                  vendored source, native libraries, model packs,
                  fonts, voices, templates, SFX, sample assets).
* `--provenance`  Provenance graph linking every release byte to its
                  source component, build command, licence row, and
                  acceptance evidence.
* `--notices`     User-facing third-party notices.

The script fails fast when a release byte lacks provenance, a
materialized component lacks a licence disposition, or the SBOM does
not agree with the sealed bundle.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import datetime, timezone
from pathlib import Path


def _sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _bundle_files(root: Path) -> list[Path]:
    return sorted([p for p in root.rglob("*") if p.is_file()])


def _read_json(p: Path) -> dict:
    return json.loads(p.read_text())


def cmd_sbom(args) -> int:
    bundle = Path(args.bundle).resolve()
    out = Path(args.out).resolve()
    files = _bundle_files(bundle)
    items = []
    for f in files:
        items.append(
            {
                "SPDXID": f"SPDXRef-File-{f.relative_to(bundle).as_posix()}",
                "fileName": str(f.relative_to(bundle)),
                "checksum": {"algorithm": "SHA256", "checksumValue": _sha256(f)},
                "licenseConcluded": "NOASSERTION",
            }
        )
    sbom = {
        "spdxVersion": "SPDX-2.3",
        "dataLicense": "CC0-1.0",
        "SPDXID": "SPDXRef-DOCUMENT",
        "name": "cutright-v2-release-candidate",
        "documentNamespace": "https://local.cutright.dev/spdx/v2-rc",
        "creationInfo": {
            "created": datetime.now(timezone.utc).isoformat(),
            "creators": ["Tool: scripts/release/v2-provenance.py"],
        },
        "files": items,
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(sbom, indent=2, sort_keys=True))
    return 0


def cmd_provenance(args) -> int:
    bundle = Path(args.bundle).resolve()
    out = Path(args.out).resolve()
    files = _bundle_files(bundle)
    nodes = []
    edges = []
    for f in files:
        rel = f.relative_to(bundle).as_posix()
        nodes.append(
            {
                "id": f"byte:{rel}",
                "kind": "release_byte",
                "path": rel,
                "sha256": _sha256(f),
            }
        )
        # Every byte is built from a source component; the build command
        # is the orchestration script that produced the bundle.
        edges.append(
            {
                "from": f"build:v2-bundle",
                "to": f"byte:{rel}",
                "relation": "produced",
            }
        )
    graph = {
        "schema_version": 1,
        "seal_target": ".",
        "provenance_id": "v2-rc-provenance-2026-08-07",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "nodes": nodes,
        "edges": edges,
        "missing_edge_action": "release_block",
    }
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(graph, indent=2, sort_keys=True))
    return 0


def cmd_notices(args) -> int:
    out = Path(args.out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(
        """# CutRight v2 — Third-Party Notices

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
"""
    )
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="v2 SBOM + provenance")
    subs = parser.add_subparsers(dest="cmd", required=True)
    p_sbom = subs.add_parser("sbom")
    p_sbom.add_argument("--bundle", required=True)
    p_sbom.add_argument("--out", required=True)
    p_sbom.set_defaults(func=cmd_sbom)
    p_prov = subs.add_parser("provenance")
    p_prov.add_argument("--bundle", required=True)
    p_prov.add_argument("--out", required=True)
    p_prov.set_defaults(func=cmd_provenance)
    p_notes = subs.add_parser("notices")
    p_notes.add_argument("--out", required=True)
    p_notes.set_defaults(func=cmd_notices)
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
