#!/usr/bin/env python3
"""scripts/release/validate-samples.py — verify v2 sample projects.

Each v2 sample lives under `samples/v2/<name>/` and is required to ship:

* `project.json` — frozen v2 project descriptor
* `sources/manifest.json` — rights/provenance for the source media
* `analysis/transcript.json` — analysis evidence
* `edit/timeline.json` — edit/timeline
* `acceptance-hashes.json` — SHA-256 of every generated asset

This script walks the sample tree, recomputes the SHA-256 of every
referenced asset, and writes `release/v2/sample-manifest.json`.

The script does not contact any external service. Sample outputs are
small and reproducible from the bytes in the working tree.
"""

from __future__ import annotations
import argparse
import hashlib
import json
import sys
from pathlib import Path

EXPECTED_SAMPLES = (
    "recorded-talking-head",
    "repurpose-podcast",
    "procedural-explainer",
    "anchored-product",
)


def _sha256(p: Path) -> str:
    h = hashlib.sha256()
    with p.open("rb") as fh:
        for chunk in iter(lambda: fh.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def _validate_one(sample_dir: Path) -> dict:
    required = (
        sample_dir / "project.json",
        sample_dir / "sources" / "manifest.json",
        sample_dir / "analysis" / "transcript.json",
        sample_dir / "edit" / "timeline.json",
        sample_dir / "acceptance-hashes.json",
    )
    missing = [str(p.relative_to(sample_dir)) for p in required if not p.exists()]
    if missing:
        raise SystemExit(
            f"sample {sample_dir.name} missing required files: {missing}"
        )

    project = json.loads((sample_dir / "project.json").read_text())
    if project.get("schema_version") != "v2":
        raise SystemExit(
            f"sample {sample_dir.name} project.json must declare schema_version=v2"
        )
    if project.get("lane") not in (
        "creator",
        "speech",
        "vision",
        "creative",
    ):
        raise SystemExit(
            f"sample {sample_dir.name} project.json must declare a known lane"
        )

    sources = json.loads((sample_dir / "sources" / "manifest.json").read_text())
    if not sources.get("rights_cleared"):
        raise SystemExit(
            f"sample {sample_dir.name} sources must assert rights_cleared=true"
        )

    hashes = json.loads((sample_dir / "acceptance-hashes.json").read_text())
    rows = []
    for entry in hashes.get("assets", []):
        path = sample_dir / entry["path"]
        if not path.exists():
            raise SystemExit(
                f"sample {sample_dir.name} asset {entry['path']} does not exist"
            )
        actual = _sha256(path)
        if entry["sha256"] != actual:
            raise SystemExit(
                f"sample {sample_dir.name} asset {entry['path']} hash mismatch"
            )
        rows.append({"path": entry["path"], "size": path.stat().st_size, "sha256": actual})

    return {
        "name": sample_dir.name,
        "lane": project["lane"],
        "content_type": project.get("format_key", {}).get("content_type"),
        "platform": project.get("format_key", {}).get("platform"),
        "title": project.get("title", sample_dir.name),
        "rights_cleared": sources["rights_cleared"],
        "rights_statement": sources.get("rights_statement", ""),
        "tutorial_path": "README.md",
        "expected_stages": project.get("expected_stages", []),
        "assets": rows,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="validate v2 samples")
    parser.add_argument("--samples", default="samples/v2")
    parser.add_argument("--out", required=True)
    args = parser.parse_args(argv)

    root = Path(args.samples).resolve()
    out = Path(args.out).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)

    present = sorted(p.name for p in root.iterdir() if p.is_dir()) if root.exists() else []
    missing = [n for n in EXPECTED_SAMPLES if n not in present]
    if missing:
        raise SystemExit(f"missing samples: {missing}")

    rows = [_validate_one(root / n) for n in EXPECTED_SAMPLES]
    manifest = {
        "schema_version": "v2",
        "kind": "sample",
        "samples_dir": ".",
        "external_runtime_dependencies": [],
        "offline_only": True,
        "samples": rows,
    }
    out.write_text(json.dumps(manifest, indent=2, sort_keys=True))
    print(f"wrote {out} with {len(rows)} samples")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
