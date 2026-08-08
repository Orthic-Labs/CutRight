#!/usr/bin/env python3
"""Verify hash-bound native & Cutaway/Finish fixtures."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path


def main() -> int:
    manifest_path = Path(sys.argv[1] if len(sys.argv) > 1 else "fixtures/macos-native/MANIFEST.json")
    repository = Path(__file__).resolve().parents[1]
    for relative in (
        "schemas/finish-plan.schema.json",
        "schemas/native-render-receipt.schema.json",
    ):
        try:
            json.loads((repository / relative).read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            print(f"FAIL: cannot parse {relative}: {error}")
            return 1
    try:
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"FAIL: cannot read fixture manifest: {error}")
        return 1
    if manifest.get("schemaVersion") == 1:
        references = manifest.get("references")
        fixture_names = manifest.get("fixtures")
        if manifest.get("purpose") != "cutaway-finish-golden-contracts" or not isinstance(references, list) or len(references) != 14:
            print("FAIL: invalid Cutaway/Finish reference manifest")
            return 1
        seen: set[str] = set()
        for item in references:
            relative = item.get("upstreamRelativePath") if isinstance(item, dict) else None
            expected = item.get("sha256") if isinstance(item, dict) else None
            candidate = Path(relative) if isinstance(relative, str) else Path("/")
            if (
                not isinstance(relative, str)
                or relative in seen
                or candidate.is_absolute()
                or ".." in candidate.parts
                or item.get("license") != "MIT"
                or item.get("disposition") != "provenance-only"
                or not re.fullmatch(r"[0-9a-f]{64}", str(expected))
            ):
                print(f"FAIL: unsafe Cutaway/Finish reference: {relative!r}")
                return 1
            resolved = (repository / candidate).resolve()
            if repository not in resolved.parents or not resolved.is_file() or hashlib.sha256(resolved.read_bytes()).hexdigest() != expected:
                print(f"FAIL: Cutaway/Finish reference hash mismatch: {relative}")
                return 1
            seen.add(relative)
        if not isinstance(fixture_names, list) or any(
            not (manifest_path.parent / str(name)).is_file() for name in fixture_names
        ):
            print("FAIL: Cutaway/Finish behavior fixture missing")
            return 1
        print(json.dumps({"status": "pass", "references": len(seen), "fixtures": len(fixture_names)}, sort_keys=True))
        return 0
    fixtures = manifest.get("fixtures")
    if manifest.get("schema") != "cutright.macos-native-fixtures.v1" or not isinstance(fixtures, list) or not fixtures:
        print("FAIL: invalid or empty Mac-native fixture manifest")
        return 1
    seen: set[str] = set()
    for item in fixtures:
        if not isinstance(item, dict):
            print("FAIL: fixture entry is not an object")
            return 1
        relative = item.get("path")
        expected = item.get("sha256")
        if not isinstance(relative, str) or relative in seen or "\\" in relative:
            print(f"FAIL: invalid or duplicate fixture path: {relative!r}")
            return 1
        candidate = Path(relative)
        if candidate.is_absolute() or ".." in candidate.parts or not re.fullmatch(r"[0-9a-f]{64}", str(expected)):
            print(f"FAIL: unsafe fixture entry: {relative!r}")
            return 1
        resolved = (repository / candidate).resolve()
        if repository not in resolved.parents or not resolved.is_file():
            print(f"FAIL: fixture is missing or escaped repository: {relative}")
            return 1
        actual = hashlib.sha256(resolved.read_bytes()).hexdigest()
        if actual != expected:
            print(f"FAIL: fixture hash mismatch: {relative}")
            return 1
        seen.add(relative)
    print(json.dumps({"status": "pass", "fixtures": len(seen), "promotionReady": bool(manifest.get("promotionReady"))}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
