#!/usr/bin/env python3
"""Enforce CutRight's crate dependency direction from Cargo manifests."""

from __future__ import annotations

import sys
import tomllib
from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[1]

# A package may depend only on these local packages. This is intentionally
# explicit: adding a local edge requires updating this contract and review.
ALLOWED = {
    "video-core": set(),
    "video-actions": {"video-core"},
    "video-capabilities": {"video-core"},
    "video-state": {"video-core"},
    "video-sessions": {"video-core"},
    "video-security": {"video-core"},
    "video-media": {"video-core"},
    "video-providers": {"video-core", "video-media"},
    "video-runtime": {"video-core"},
    "video-jobs": {"video-core"},
    "video-editorial": {"video-benchmarks"},
    "video-services": {"video-core", "video-jobs", "video-runtime", "video-state"},
    "video-project": {
        "video-actions", "video-capabilities", "video-core", "video-editorial",
        "video-jobs", "video-media", "video-providers", "video-runtime",
        "video-security", "video-sessions", "video-state",
    },
    "video-cli": {"video-capabilities", "video-core", "video-project", "video-providers", "video-sessions"},
    "video-agent": {"video-actions", "video-capabilities", "video-project", "video-sessions"},
    "video-daemon": {"video-services"},
    "cutright-mcp": {"video-daemon"},
    "video-driver-host": set(),
    "video-protocol": set(),
}

FORBIDDEN = {
    "video-driver-host": {"video-project", "video-state", "video-actions", "project-storage"},
    "video-protocol": {"video-project", "video-state", "video-actions", "video-services"},
}


def local_dependencies(manifest: Path) -> tuple[str, set[str]]:
    data = tomllib.loads(manifest.read_text(encoding="utf-8"))
    package = data.get("package", {})
    name = package.get("name")
    if not name:
        return "", set()
    deps: set[str] = set()
    for section in ("dependencies", "dev-dependencies", "build-dependencies"):
        for dep, value in data.get(section, {}).items():
            if isinstance(value, dict) and "path" in value:
                deps.add(dep)
    return name, deps


def main() -> int:
    workspace = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    manifests = [ROOT / member / "Cargo.toml" for member in workspace["workspace"]["members"]]
    studio = ROOT / "apps/studio/src-tauri/Cargo.toml"
    if studio.exists():
        manifests.append(studio)
    violations: list[str] = []
    for manifest in manifests:
        name, deps = local_dependencies(manifest)
        if not name or name not in ALLOWED:
            continue
        forbidden = deps & FORBIDDEN.get(name, set())
        for dep in sorted(forbidden):
            violations.append(f"{manifest.relative_to(ROOT)}: {name} -> {dep}: forbidden dependency")
        unexpected = deps - ALLOWED[name]
        for dep in sorted(unexpected - forbidden):
            violations.append(f"{manifest.relative_to(ROOT)}: {name} -> {dep}: forbidden dependency (edge not in enforced DAG)")
        if name in {"video-driver-host", "video-protocol"}:
            source_root = manifest.parent / "src"
            forbidden_symbols = re.compile(r"ActionExecutor|video_project|video_state|video_services|project.storage")
            for source in source_root.rglob("*.rs") if source_root.exists() else ():
                if forbidden_symbols.search(source.read_text(encoding="utf-8")):
                    violations.append(f"{source.relative_to(ROOT)}: {name}: forbidden project/mutation symbol")
    if violations:
        print("CRATE DAG FAIL", file=sys.stderr)
        print("\n".join(f"- {item}" for item in violations), file=sys.stderr)
        return 1
    print("CRATE DAG PASS: enforced Studio/videoctl/cutright-mcp -> video-daemon -> video-services -> video-project -> domain crates")
    return 0


if __name__ == "__main__":
    sys.exit(main())
