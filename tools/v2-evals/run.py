#!/usr/bin/env python3
"""Deterministic runner for the CutRight v2 import eval suite.

Adapted from the workspace eval-runner concept (tools/evals/run_research_evals.py
at pin 6ee21f03a787e7b57dc412760a8996ea7a235302, source_id "workspace-capabilities");
rewritten as a static, stdlib-only gate over committed fixtures — no network, no
LLM calls, no timestamps, sorted output.

For `--suite import` it verifies:

1. every suite case file parses and matches schemas/evals/eval-case.schema.v1.json;
2. case ids are unique across the suite;
3. every included skill has at least one positive (routed) case and at least one
   refusal/degradation case;
4. every referenced skill exists in the catalogue or the known-missing list;
5. every negative fixture under fixtures/evals/negative/ triggers exactly the
   failure class declared in its expected.json (unclassified dependency, external
   path, missing permission, mutable model reference, absent notice);
6. every workspace eval source in the suite has either an import source notice
   or an exclusion row in fixtures/evals/exclusions.json.

Exit codes: 0 pass, 1 failures, 2 skills root or suite definition missing.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import _lib as lib

VALID_STATUSES = {"routed", "refused", "degraded"}
NEGATIVE_CLASSES = {
    "unclassified_dependency",
    "external_path",
    "missing_permission",
    "mutable_model_ref",
    "absent_notice",
}


def load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except json.JSONDecodeError as exc:
        return None, f"invalid JSON: {exc}"
    except OSError as exc:
        return None, f"unreadable: {exc}"


def check_case_file(
    repo_root: Path, rel: str, seen_ids: set[str], allowed: set[str]
) -> tuple[list[dict], list[str]]:
    """Returns (cases, errors) for one case file."""
    errors: list[str] = []
    data, err = load_json(repo_root / rel)
    if err:
        return [], [f"{rel}: {err}"]
    if not isinstance(data, dict) or data.get("schema_version") != 1:
        errors.append(f"{rel}: missing schema_version 1")
        return [], errors
    skill = data.get("skill")
    if not isinstance(skill, str) or not skill:
        errors.append(f"{rel}: missing top-level skill field")
    source = data.get("source")
    if not isinstance(source, dict) or not source.get("source_id") or not source.get("revision"):
        errors.append(f"{rel}: missing source notice (source_id + revision)")
    rows = data.get("cases")
    if not isinstance(rows, list) or not rows:
        errors.append(f"{rel}: missing non-empty cases list")
        return [], errors
    cases: list[dict] = []
    for index, case in enumerate(rows):
        where = f"{rel}:cases[{index}]"
        if not isinstance(case, dict):
            errors.append(f"{where}: not an object")
            continue
        case_id = case.get("case_id")
        if not isinstance(case_id, str) or not case_id:
            errors.append(f"{where}: missing case_id")
            continue
        if case_id in seen_ids:
            errors.append(f"{where}: duplicate case_id {case_id!r}")
        seen_ids.add(case_id)
        where = f"{rel}:{case_id}"
        request = case.get("input")
        if not isinstance(request, dict) or not isinstance(request.get("request"), str) or not request["request"]:
            errors.append(f"{where}: missing input.request")
        expected = case.get("expected")
        if not isinstance(expected, dict):
            errors.append(f"{where}: missing expected object")
            continue
        status = expected.get("status")
        if status not in VALID_STATUSES:
            errors.append(f"{where}: expected.status must be one of {sorted(VALID_STATUSES)}")
        elif status == "routed":
            target = expected.get("skill")
            if not isinstance(target, str) or not target:
                errors.append(f"{where}: routed case needs expected.skill")
            elif target not in allowed:
                errors.append(f"{where}: routes to absent skill {target!r}")
        else:
            reason = expected.get("reason_code")
            if not isinstance(reason, str) or not reason:
                errors.append(f"{where}: {status} case needs expected.reason_code")
        for ref in case.get("forbidden_skills", []) or []:
            if isinstance(ref, str) and ref and ref not in allowed:
                errors.append(f"{where}: forbids absent skill {ref!r}")
        cases.append({"case_id": case_id, "skill": skill, "status": status})
    return cases, errors


def check_negative_fixtures(repo_root: Path, negative_dir: Path) -> list[str]:
    errors: list[str] = []
    if not negative_dir.is_dir():
        return [f"negative fixture directory missing: {negative_dir.relative_to(repo_root)}"]
    found_classes: set[str] = set()
    for fixture_root in sorted(p for p in negative_dir.iterdir() if p.is_dir()):
        rel = fixture_root.relative_to(repo_root)
        expected, err = load_json(fixture_root / "expected.json")
        if err:
            errors.append(f"{rel}: expected.json {err}")
            continue
        declared = expected.get("expected_class") if isinstance(expected, dict) else None
        if declared not in NEGATIVE_CLASSES:
            errors.append(f"{rel}: expected_class must be one of {sorted(NEGATIVE_CLASSES)}")
            continue
        detected = detect_classes(fixture_root)
        if declared not in detected:
            errors.append(f"{rel}: expected class {declared!r} not detected (got {sorted(detected)})")
        else:
            found_classes.add(declared)
    for missing in sorted(NEGATIVE_CLASSES - found_classes):
        errors.append(f"no negative fixture demonstrates class {missing!r}")
    return errors


def detect_classes(fixture_root: Path) -> set[str]:
    detected: set[str] = set()
    skills = lib.load_skills(fixture_root / "skills")
    capabilities, registry_present = lib.load_registry(fixture_root)
    known = {row["name"] for row in skills}
    for row in skills:
        metadata = row["metadata"]
        if lib.detect_dangling_dependencies(metadata, known):
            detected.add("unclassified_dependency")
        if lib.detect_external_paths(metadata):
            detected.add("external_path")
        if lib.detect_undeclared_permissions(metadata, capabilities, registry_present):
            detected.add("missing_permission")
        if lib.detect_mutable_model_refs(metadata):
            detected.add("mutable_model_ref")
        if row["has_skill_md"] and lib.detect_missing_notice(row["path"]):
            detected.add("absent_notice")
    return detected


def check_exclusions(
    repo_root: Path, suite: dict, imported_sources: set[str]
) -> list[str]:
    errors: list[str] = []
    sources = suite.get("workspace_eval_sources", [])
    exclusions_rel = suite.get("exclusions_file", "fixtures/evals/exclusions.json")
    data, err = load_json(repo_root / exclusions_rel)
    rows = data.get("rows", []) if isinstance(data, dict) else []
    covered: dict[str, str] = {}
    for row in rows:
        if not isinstance(row, dict) or not row.get("source_path") or not row.get("reason"):
            errors.append(f"{exclusions_rel}: row needs source_path and reason")
            continue
        covered[row["source_path"]] = row["reason"]
    for source in sources:
        in_tree = source in imported_sources
        excluded = source in covered
        if not in_tree and not excluded:
            errors.append(f"omitted workspace eval has no exclusion row: {source}")
        if in_tree and excluded:
            errors.append(f"workspace eval is both imported and excluded: {source}")
    for path in covered:
        if path not in sources:
            errors.append(f"exclusion row references unknown workspace eval: {path}")
    return errors


def run_suite(repo_root: Path, suite_rel: str) -> tuple[int, dict]:
    suite_path = repo_root / suite_rel
    suite, err = load_json(suite_path)
    if err or not isinstance(suite, dict):
        return lib.EXIT_MISSING_ROOT, {"suite": suite_rel, "errors": [f"suite definition {err or 'invalid'}"]}

    skills_root = repo_root / suite.get("skills_root", "skills")
    report: dict = {"suite": suite.get("suite"), "checks": {}, "errors": []}
    if not skills_root.is_dir():
        report["errors"].append(f"skills root missing: {skills_root}")
        report["verdict"] = "SKIPPED (missing root)"
        return lib.EXIT_MISSING_ROOT, report

    known = {row["name"] for row in lib.load_skills(skills_root)}
    known_missing = lib.load_known_missing(
        repo_root / "fixtures" / "evals" / "known-missing-skills.json"
    )
    allowed = known | known_missing

    errors: list[str] = []
    seen_ids: set[str] = set()
    all_cases: list[dict] = []
    imported_sources: set[str] = set()
    for rel in sorted(suite.get("case_files", [])):
        cases, file_errors = check_case_file(repo_root, rel, seen_ids, allowed)
        errors.extend(file_errors)
        all_cases.extend(cases)
        data, _ = load_json(repo_root / rel)
        if isinstance(data, dict) and isinstance(data.get("source"), dict):
            source_file = data["source"].get("source_file")
            if isinstance(source_file, str) and source_file:
                imported_sources.add(source_file)

    included = sorted(suite.get("included_skills", []))
    coverage_errors: list[str] = []
    for skill in included:
        positives = [c for c in all_cases if c["skill"] == skill and c["status"] == "routed"]
        refusals = [c for c in all_cases if c["skill"] == skill and c["status"] in {"refused", "degraded"}]
        if not positives:
            coverage_errors.append(f"included skill {skill!r} has no positive (routed) case")
        if not refusals:
            coverage_errors.append(f"included skill {skill!r} has no refusal/degradation case")
    file_skills = {c["skill"] for c in all_cases if c["skill"]}
    if file_skills - set(included):
        coverage_errors.append(
            f"case files exist for skills outside included_skills: {sorted(file_skills - set(included))}"
        )
    errors.extend(coverage_errors)

    negative_dir = repo_root / suite.get("negative_dir", "fixtures/evals/negative")
    errors.extend(check_negative_fixtures(repo_root, negative_dir))
    errors.extend(check_exclusions(repo_root, suite, imported_sources))

    report["checks"] = {
        "case_files": len(suite.get("case_files", [])),
        "cases": len(all_cases),
        "included_skills": included,
        "negative_fixtures": len([p for p in negative_dir.iterdir() if p.is_dir()]) if negative_dir.is_dir() else 0,
        "workspace_eval_sources": len(suite.get("workspace_eval_sources", [])),
        "skills_in_tree": sorted(known),
    }
    report["errors"] = sorted(set(errors))
    report["verdict"] = "PASS" if not errors else f"FAIL ({len(report['errors'])} errors)"
    return (lib.EXIT_OK if not errors else lib.EXIT_ERRORS), report


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", default="import")
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    args = parser.parse_args(argv)
    repo_root = args.repo_root.resolve()
    suite_rel = f"fixtures/evals/suites/{args.suite}.json"
    code, report = run_suite(repo_root, suite_rel)
    print(json.dumps(report, indent=2, sort_keys=True))
    return code


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
