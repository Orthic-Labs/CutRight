#!/usr/bin/env python3
"""Fail-closed catalogue-integrity gate for the CutRight skill catalogue.

Adapted from the workspace concept tools/evals/catalog_integrity.py at pin
6ee21f03a787e7b57dc412760a8996ea7a235302 (source_id "workspace-capabilities");
rewritten for CutRight roots and schema names. The build fails for any of five
classes:

1. malformed, duplicate, or directory-mismatched skill identity;
2. capability-registry divergence (when capabilities/registry.json exists);
3. stale eval-case references (fixtures/evals/cases/**.json);
4. stale MAY_CALL_SKILLS edges;
5. dangling frontmatter `depends:` edges.

Skills planned but not yet materialised are listed in
fixtures/evals/known-missing-skills.json so the gate stays usable while the
tree is being built concurrently; the Book 1 join task prunes the list.

Deterministic: every report line is sorted; no timestamps. Exit codes:
0 clean, 1 errors found, 2 skills root missing.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _lib as lib


def identity_errors(skills: list[dict]) -> list[str]:
    errors: list[str] = []
    identities: dict[str, str] = {}
    for row in skills:
        name = row["name"]
        if not row["has_skill_md"]:
            errors.append(f"skill directory has no SKILL.md: skills/{name}")
            continue
        declared = row["metadata"].get("name", "").strip()
        description = row["metadata"].get("description", "").strip()
        if not declared:
            errors.append(f"skill has no frontmatter name: skills/{name}/SKILL.md")
            continue
        if declared != name:
            errors.append(
                f"skill name differs from directory: skills/{name}/SKILL.md declares "
                f"{declared!r}, expected {name!r}"
            )
        if not description:
            errors.append(f"skill has no frontmatter description: skills/{name}/SKILL.md")
        prior = identities.get(declared)
        if prior is not None:
            errors.append(
                f"duplicate skill identity {declared!r}: skills/{prior} and skills/{name}"
            )
        identities[declared] = name
    return errors


def registry_errors(
    skills: list[dict], capabilities: set[str], registry_present: bool
) -> tuple[list[str], list[str]]:
    if not registry_present:
        return [], ["capabilities/registry.json absent; registry-divergence check skipped"]
    known = {row["name"] for row in skills if row["has_skill_md"]}
    errors: list[str] = []
    # Registry capabilities are permission names, not skill identities; divergence
    # is checked through permissions declared by skills.
    for row in skills:
        for perm in row["metadata"].get("permissions", "").split(","):
            perm = perm.strip()
            if perm and perm.lower() != "none" and perm not in capabilities:
                errors.append(
                    f"skill {row['name']!r} declares capability not in registry: {perm!r}"
                )
    _ = known
    return errors, []


def eval_case_references(repo_root: Path) -> list[tuple[str, str]]:
    refs: list[tuple[str, str]] = []
    cases_dir = repo_root / "fixtures" / "evals" / "cases"
    if not cases_dir.is_dir():
        return refs
    for path in sorted(cases_dir.rglob("*.json")):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        rel = str(path.relative_to(repo_root))
        for case in data.get("cases", []) if isinstance(data, dict) else []:
            for key in ("skill", "expected_skill"):
                value = case.get(key)
                if isinstance(value, str) and value:
                    refs.append((rel, value))
            expected = case.get("expected")
            if isinstance(expected, dict):
                value = expected.get("skill")
                if isinstance(value, str) and value:
                    refs.append((rel, value))
            forbidden = case.get("forbidden_skills")
            if isinstance(forbidden, list):
                for item in forbidden:
                    if isinstance(item, str) and item:
                        refs.append((rel, item))
    return refs


def validate(repo_root: Path, skills_root: Path) -> tuple[list[str], list[str]]:
    notes: list[str] = []
    skills = lib.load_skills(skills_root)
    known = {row["name"] for row in skills if row["has_skill_md"]}
    known_missing = lib.load_known_missing(
        repo_root / "fixtures" / "evals" / "known-missing-skills.json"
    )
    allowed = known | known_missing

    errors = identity_errors(skills)

    capabilities, registry_present = lib.load_registry(repo_root)
    reg_errors, reg_notes = registry_errors(skills, capabilities, registry_present)
    errors.extend(reg_errors)
    notes.extend(reg_notes)

    for rel, ref in eval_case_references(repo_root):
        if ref not in allowed:
            errors.append(f"eval case references absent skill {ref!r}: {rel}")

    for row in skills:
        if not row["has_skill_md"]:
            continue
        for ref in lib.call_edges(row["text"]):
            if ref not in allowed:
                errors.append(
                    f"MAY_CALL_SKILLS references absent skill {ref!r}: "
                    f"skills/{row['name']}/SKILL.md"
                )
        for dep in lib.detect_dangling_dependencies(row["metadata"], allowed):
            errors.append(
                f"frontmatter depends references absent skill {dep!r}: "
                f"skills/{row['name']}/SKILL.md"
            )

    return errors, notes


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path("skills"))
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    repo_root = args.repo_root.resolve()
    skills_root = (repo_root / args.root).resolve() if not args.root.is_absolute() else args.root
    if not skills_root.is_dir():
        print(f"NOTE: skills root missing: {skills_root}")
        print("catalog_integrity: SKIPPED (missing root)")
        return lib.EXIT_MISSING_ROOT
    errors, notes = validate(repo_root, skills_root)
    lib.print_report("catalog_integrity", errors, notes)
    return lib.EXIT_OK if not errors else lib.EXIT_ERRORS


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
