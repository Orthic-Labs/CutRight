#!/usr/bin/env python3
"""validate_asset_ledger.py — validate an asset ledger produced by scan_assets.py.

Usage: python3 tools/import-closure/validate_asset_ledger.py LEDGER.json
Checks structural completeness: every asset row has class, sha256, size,
and an explicit (non-inherited) licence status.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import read_json  # noqa: E402

HEX64 = re.compile(r"^[0-9a-f]{64}$")
ALLOWED_STATUS = {
    "pending_explicit_row",
    "licensed_for_redistribution",
    "excluded",
    "audited_separately",
}


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    doc = read_json(Path(argv[1]))
    errors = []
    assets = doc.get("assets")
    if not isinstance(assets, list):
        errors.append("missing assets array")
        assets = []
    if doc.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    seen = set()
    for i, asset in enumerate(assets):
        path = asset.get("path") or f"assets[{i}]"
        if not asset.get("path"):
            errors.append(f"assets[{i}]: missing path")
        elif asset["path"] in seen:
            errors.append(f"duplicate asset path: {asset['path']}")
        seen.add(asset.get("path"))
        if not HEX64.match(asset.get("sha256") or ""):
            errors.append(f"{path}: sha256 must be 64 lowercase hex chars")
        if not isinstance(asset.get("bytes"), int) or asset["bytes"] < 0:
            errors.append(f"{path}: bytes must be a non-negative integer")
        if asset.get("asset_class") not in {"model_weights", "voices", "fonts", "luts", "dataset", "textures", "sample_media", "code", "documentation", "configuration", "music", "sfx"}:
            errors.append(f"{path}: unknown asset_class {asset.get('asset_class')!r}")
        if asset.get("licence_status") not in ALLOWED_STATUS:
            errors.append(f"{path}: licence_status {asset.get('licence_status')!r} not allowed")
    if errors:
        for err in errors[:50]:
            print(f"FAIL {err}", file=sys.stderr)
        return 1
    print(f"OK {argv[1]}: {len(assets)} asset rows structurally valid")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
