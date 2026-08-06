#!/usr/bin/env python3
"""Validate the CutRight public-skill topology.

Adapted from the workspace concept tools/evals/validate_skill_topology.py at
pin 6ee21f03a787e7b57dc412760a8996ea7a235302 (source_id "workspace-capabilities");
the workspace variant encoded one research-consolidation rule, re-expressed here
as the generic CutRight topology contract:

1. every top-level skill directory ships exactly one SKILL.md at its root;
2. nested SKILL.md files are forbidden (branches must not go accidentally
   public);
3. skill identity equals directory name and carries a description;
4. MAY_CALL_SKILLS edges are acyclic over the materialised catalogue;
5. no two skill directories declare the same identity.

Deterministic: every report line is sorted; no timestamps. Exit codes:
0 clean, 1 errors found, 2 skills root missing.
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _lib as lib


def nested_skill_md_errors(skills: list[dict]) -> list[str]:
    errors: list[str] = []
    for row in skills:
        for path in sorted(row["path"].rglob("SKILL.md")):
            if path != row["skill_md"]:
                errors.append(
                    f"nested branch is accidentally public: "
                    f"skills/{row['name']}/{path.relative_to(row['path'])}"
                )
    return errors


def cycle_errors(skills: list[dict], allowed: set[str]) -> list[str]:
    """Kahn's algorithm over MAY_CALL_SKILLS edges, lexicographic tie-break."""
    graph: dict[str, set[str]] = {}
    for row in skills:
        if row["has_skill_md"]:
            graph[row["name"]] = set()
    for row in skills:
        if not row["has_skill_md"]:
            continue
        for ref in lib.call_edges(row["text"]):
            if ref in graph:
                graph[row["name"]].add(ref)
    in_degree = {node: 0 for node in graph}
    for node, edges in graph.items():
        for target in edges:
            in_degree[target] += 1
    ready = sorted(node for node, degree in in_degree.items() if degree == 0)
    visited = 0
    while ready:
        node = ready.pop(0)
        visited += 1
        for target in sorted(graph[node]):
            in_degree[target] -= 1
            if in_degree[target] == 0:
                ready.append(target)
        ready.sort()
    if visited == len(graph):
        return []
    members = sorted(node for node, degree in in_degree.items() if degree > 0)
    return [f"MAY_CALL_SKILLS dependency cycle involves: {', '.join(members)}"]


def validate(repo_root: Path, skills_root: Path) -> tuple[list[str], list[str]]:
    notes: list[str] = []
    skills = lib.load_skills(skills_root)
    errors: list[str] = []
    identities: dict[str, str] = {}

    for row in skills:
        name = row["name"]
        if not row["has_skill_md"]:
            errors.append(f"skill directory has no root SKILL.md: skills/{name}")
            continue
        declared = row["metadata"].get("name", "").strip()
        if declared and declared != name:
            errors.append(
                f"skill identity differs from directory: skills/{name} declares {declared!r}"
            )
        if not row["metadata"].get("description", "").strip():
            errors.append(f"skill has no description: skills/{name}/SKILL.md")
        key = declared or name
        if key in identities:
            errors.append(
                f"duplicate skill identity {key!r}: skills/{identities[key]} and skills/{name}"
            )
        identities[key] = name

    errors.extend(nested_skill_md_errors(skills))

    known_missing = lib.load_known_missing(
        repo_root / "fixtures" / "evals" / "known-missing-skills.json"
    )
    errors.extend(cycle_errors(skills, known_missing))
    if known_missing:
        notes.append(
            f"known-missing skills excluded from cycle check: {', '.join(sorted(known_missing))}"
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
        print("validate_skill_topology: SKIPPED (missing root)")
        return lib.EXIT_MISSING_ROOT
    errors, notes = validate(repo_root, skills_root)
    lib.print_report("validate_skill_topology", errors, notes)
    return lib.EXIT_OK if not errors else lib.EXIT_ERRORS


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
