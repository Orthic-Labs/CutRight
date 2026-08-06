"""Shared helpers for the CutRight v2 eval tooling (stdlib only).

Adapted from workspace concepts at pin 6ee21f03a787e7b57dc412760a8996ea7a235302
(tools/evals/catalog_integrity.py, tools/evals/validate_skill_topology.py,
source_id "workspace-capabilities"); rewritten for CutRight roots, schema names,
and deterministic output. No workspace code was copied.

Conventions checked here:
- a skill is a directory directly under the skills root with a SKILL.md;
- SKILL.md frontmatter uses `name` and `description` keys;
- dependency edges are declared on MAY_CALL_SKILLS control lines as
  `cutright://skill/<name>` references (optionally with mode suffixes).
"""
from __future__ import annotations

import json
import re
from pathlib import Path

FRONTMATTER_RE = re.compile(r"\A---\s*\n(.*?)\n---(?:\s*\n|\Z)", re.S)
CONTROL_RE = re.compile(r"^\s*MAY_CALL_SKILLS\s*:\s*(.*?)\s*$", re.I)
SKILL_REF_RE = re.compile(r"cutright://skill/([a-z0-9][a-z0-9-]*)")

# Skill directories that hold shared material, not a public skill identity.
SHARED_DIR_PREFIXES = ("_",)

EXIT_OK = 0
EXIT_ERRORS = 1
EXIT_MISSING_ROOT = 2


def parse_frontmatter(text: str) -> dict[str, str]:
    match = FRONTMATTER_RE.match(text.replace("\r\n", "\n").replace("\r", "\n"))
    if not match:
        return {}
    values: dict[str, str] = {}
    for line in match.group(1).splitlines():
        if not line.strip() or line.lstrip().startswith("#") or ":" not in line:
            continue
        key, value = line.split(":", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def normalize(text: str) -> str:
    return text.replace("\r\n", "\n").replace("\r", "\n")


def skill_directories(root: Path) -> list[Path]:
    if not root.is_dir():
        return []
    rows = []
    for entry in sorted(root.iterdir()):
        if not entry.is_dir() or entry.name.startswith(SHARED_DIR_PREFIXES):
            continue
        rows.append(entry)
    return rows


def load_skills(root: Path) -> list[dict]:
    """One row per skill directory; metadata is empty when SKILL.md is absent."""
    rows = []
    for directory in skill_directories(root):
        skill_md = directory / "SKILL.md"
        text = ""
        metadata: dict[str, str] = {}
        if skill_md.is_file():
            text = skill_md.read_text(encoding="utf-8")
            metadata = parse_frontmatter(text)
        rows.append(
            {
                "name": directory.name,
                "path": directory,
                "skill_md": skill_md,
                "has_skill_md": skill_md.is_file(),
                "text": text,
                "metadata": metadata,
            }
        )
    return rows


def call_edges(text: str) -> list[str]:
    """Skill names referenced on the first MAY_CALL_SKILLS control line."""
    for line in normalize(text).splitlines():
        match = CONTROL_RE.match(line)
        if not match:
            continue
        value = match.group(1).strip()
        if not value or value.upper() == "NONE":
            return []
        seen: list[str] = []
        for ref in SKILL_REF_RE.findall(value):
            if ref not in seen:
                seen.append(ref)
        return seen
    return []


def load_registry(root: Path) -> tuple[set[str], bool]:
    """Capability names from capabilities/registry.json; tolerant when absent."""
    path = root / "capabilities" / "registry.json"
    if not path.is_file():
        return set(), False
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return set(), True
    names: set[str] = set()
    for key in ("capabilities", "skills"):
        rows = data.get(key)
        if isinstance(rows, list):
            for row in rows:
                if isinstance(row, dict) and isinstance(row.get("name"), str):
                    names.add(row["name"])
                elif isinstance(row, str):
                    names.add(row)
    return names, True


def load_known_missing(path: Path) -> set[str]:
    """Skills planned but not yet materialised in the tree (join task prunes)."""
    if not path.is_file():
        return set()
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return set()
    rows = data.get("skills", [])
    return {row for row in rows if isinstance(row, str)}


def detect_dangling_dependencies(metadata: dict[str, str], known: set[str]) -> list[str]:
    value = metadata.get("depends", "").strip()
    if not value or value.lower() == "none":
        return []
    missing = []
    for item in value.split(","):
        name = item.strip()
        if name and name not in known:
            missing.append(name)
    return missing


def detect_external_paths(metadata: dict[str, str]) -> list[str]:
    value = metadata.get("resources", "").strip()
    if not value or value.lower() == "none":
        return []
    bad = []
    for item in value.split(","):
        rel = item.strip()
        if not rel:
            continue
        if (
            rel.startswith("/")
            or rel.startswith("\\")
            or ".." in rel.split("/")
            or "\\" in rel
            or "://" in rel
            or "*" in rel
        ):
            bad.append(rel)
    return bad


def detect_undeclared_permissions(
    metadata: dict[str, str], capabilities: set[str], registry_present: bool
) -> list[str]:
    value = metadata.get("permissions", "").strip()
    if not value or value.lower() == "none" or not registry_present:
        return []
    return sorted(
        {item.strip() for item in value.split(",") if item.strip()} - capabilities
    )


MUTABLE_MODEL_VALUES = {"latest", "auto", "newest", "default", "any", "*"}


def detect_mutable_model_refs(metadata: dict[str, str]) -> list[str]:
    value = metadata.get("model", "").strip()
    if not value:
        return []
    if value.lower() in MUTABLE_MODEL_VALUES:
        return [value]
    return []


def detect_missing_notice(directory: Path) -> list[str]:
    notice = directory / "NOTICE.md"
    provenance = directory / "provenance.json"
    if notice.is_file() or provenance.is_file():
        return []
    return ["missing_notice"]


def print_report(title: str, errors: list[str], notes: list[str]) -> None:
    for note in sorted(set(notes)):
        print(f"NOTE: {note}")
    for error in sorted(set(errors)):
        print(f"FAIL: {error}")
    if errors:
        print(f"{title}: {len(set(errors))} error(s)")
    else:
        print(f"{title}: OK")
