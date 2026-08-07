#!/usr/bin/env python3
"""Verify local v2 RC audit evidence without contacting any service."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from pathlib import Path


REQUIRED_AUDIT_EVIDENCE = (
    "audit.json",
    "secret-scan.json",
    "pack-tamper.json",
    "project-tamper.json",
    "source-corpus-leakage.json",
    "installer-permissions.json",
)
REQUIRED_RELEASE_ARTIFACTS = (
    "release/v2/SBOM.spdx.json",
    "release/v2/provenance.json",
    "release/v2/THIRD-PARTY-NOTICES.md",
)


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _load_json(path: Path) -> object:
    # Audit evidence permits explanatory comment lines before its JSON object.
    text = "\n".join(line for line in path.read_text(encoding="utf-8").splitlines()
                     if not line.lstrip().startswith("#"))
    return json.loads(text)


def _all_statuses_pass(value: object) -> bool:
    if isinstance(value, dict):
        status = value.get("status")
        if status is not None and status != "pass":
            return False
        return all(_all_statuses_pass(item) for item in value.values())
    if isinstance(value, list):
        return all(_all_statuses_pass(item) for item in value)
    return True


def _check(condition: bool, name: str) -> dict[str, str]:
    return {"id": name, "status": "pass" if condition else "fail"}


def _bundle_file(bundle: Path, item_path: object) -> Path | None:
    if not isinstance(item_path, str) or not item_path:
        return None
    candidate = (bundle / item_path).resolve()
    try:
        candidate.relative_to(bundle.resolve())
    except ValueError:
        return None
    return candidate


def _verify_seal(bundle: Path) -> bool:
    seal_path = bundle / "SEAL.json"
    if not bundle.is_dir() or not seal_path.is_file():
        return False
    try:
        seal = _load_json(seal_path)
        items = seal["items"] if isinstance(seal, dict) else None
        if not isinstance(items, list):
            return False
        if not items:
            return False
        root = bundle.resolve()
        expected = {
            path.resolve()
            for path in root.rglob("*")
            if path.is_file() and path.resolve() != (root / "SEAL.json").resolve()
        }
        seen: set[Path] = set()
        for item in items:
            name = item.get("path") if isinstance(item, dict) else None
            digest = item.get("sha256") if isinstance(item, dict) else None
            if (not isinstance(name, str) or not name or not isinstance(digest, str)
                    or re.fullmatch(r"[0-9a-fA-F]{64}", digest) is None):
                return False
            relative = Path(name)
            if relative.is_absolute() or ".." in relative.parts or "\\" in name:
                return False
            path = (root / relative).resolve()
            try:
                path.relative_to(root)
            except ValueError:
                return False
            if path in seen or path not in expected or not path.is_file() or _sha256(path) != digest:
                return False
            seen.add(path)
        return seen == expected
    except (OSError, ValueError, UnicodeError, TypeError, KeyError):
        return False


def _verify_audit(audit_dir: Path) -> tuple[bool, bool]:
    paths = [audit_dir / name for name in REQUIRED_AUDIT_EVIDENCE]
    present = all(path.is_file() for path in paths)
    if not present:
        return False, False
    try:
        evidence = {path.name: _load_json(path) for path in paths}
        audit = evidence["audit.json"]
        if not isinstance(audit, dict):
            return True, False
        policy = audit.get("policy")
        checks = audit.get("checks")
        required = policy.get("policy_required_checks") if isinstance(policy, dict) else None
        checked = {item.get("id"): item.get("status") for item in checks if isinstance(item, dict)} \
            if isinstance(checks, list) else {}
        audit_passes = (
            policy.get("pass_only_when_all_required_checks_pass") is True
            and policy.get("skipped_is_never_coerced_to_pass") is True
            and isinstance(required, list)
            and all(isinstance(check_id, str) and checked.get(check_id) == "pass" for check_id in required)
            and not audit.get("skipped")
            and not audit.get("unproven")
            and audit.get("summary", {}).get("release_blocking_finding") is False
            and audit.get("summary", {}).get("audit_status") == "pass"
            and all(_all_statuses_pass(item) for item in evidence.values())
        )
        return True, audit_passes
    except (OSError, ValueError, TypeError):
        return True, False


def audit(root: Path, bundle: Path, audit_dir: Path) -> dict[str, object]:
    audit_present, audit_passes = _verify_audit(audit_dir)
    checks = [
        _check(bundle.is_dir(), "bundle_exists"),
        _check(_verify_seal(bundle), "seal_verifies"),
        _check(audit_present, "audit_evidence_present"),
        _check(audit_passes, "audit_evidence_passing"),
        *[_check((root / path).is_file() and (root / path).stat().st_size > 0, path)
          for path in REQUIRED_RELEASE_ARTIFACTS],
    ]
    return {
        "schema_version": 1,
        "bundle": str(bundle.relative_to(root)),
        "audit_dir": str(audit_dir.relative_to(root)),
        "checks": checks,
        "status": "pass" if all(check["status"] == "pass" for check in checks) else "fail",
    }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="verify local v2 release audit evidence")
    parser.add_argument("--source", default=".")
    parser.add_argument("--bundle", default="release/v2/rc")
    parser.add_argument("--audit-dir", default="release/v2/audit")
    parser.add_argument("--out", required=True, help="directory for release-audit.json")
    args = parser.parse_args(argv)
    root = Path(args.source).resolve()
    bundle = (root / args.bundle).resolve()
    audit_dir = (root / args.audit_dir).resolve()
    summary = audit(root, bundle, audit_dir)
    out = Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    (out / "release-audit.json").write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(summary, sort_keys=True))
    return 0 if summary["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
