#!/usr/bin/env python3
"""Run exact-RC clean-machine acceptance without host-runtime shortcuts.

Non-CLI applications must support this local protocol:
  APP --clean-machine-sample NAME --network-deny --lifecycle
      correction-undo,restart-resume,repair-rollback,uninstall-preservation
and write one JSON object to stdout.  A passing object names its sample and
lane, reports ``ready_review``, zero network attempts, every requested
lifecycle result, and exactly the installed pack ids. ``videoctl`` receives
its native ``clean-machine-sample`` command; its JSON must satisfy that strict
lifecycle contract before acceptance can pass.
"""

from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

EXPECTED_SAMPLES = (
    ("recorded-talking-head", "creator"),
    ("repurpose-podcast", "speech"),
    ("procedural-explainer", "creative"),
    ("anchored-product", "vision"),
)
LIFECYCLES = ("correction_undo", "restart_resume", "repair_rollback", "uninstall_preservation")
APP_CANDIDATES = ("app/videoctl", "app/cutright", "app/CutRight.app/Contents/MacOS/CutRight")
NETWORK_PROFILE = "(version 1) (deny network*) (allow default)"


def _record(name: str, ok: bool, evidence: dict) -> dict:
    return {"check": name, "passed": ok, "evidence": evidence}


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _relative_rc_path(value: str) -> str:
    parts = Path(value).parts
    return str(Path(*parts[parts.index("rc") + 1:])) if "rc" in parts else value


def _bundle_file(bundle: Path, value: object) -> Path | None:
    if not isinstance(value, str) or not value:
        return None
    candidate = (bundle / value).resolve()
    try:
        candidate.relative_to(bundle.resolve())
    except ValueError:
        return None
    return candidate


def _load_json(path: Path) -> tuple[dict | None, str | None]:
    try:
        return json.loads(path.read_text(encoding="utf-8")), None
    except (OSError, json.JSONDecodeError) as exc:
        return None, str(exc)


def _verify_exact_hashes(bundle: Path) -> tuple[bool, dict]:
    seal_path = bundle / "SEAL.json"
    seal, error = _load_json(seal_path)
    if error or not isinstance(seal, dict) or not isinstance(seal.get("items"), list):
        return False, {"seal": str(seal_path), "error": error or "missing items"}
    bundle_root = bundle.resolve()
    sealed_files = {
        path.relative_to(bundle_root).as_posix()
        for path in bundle.rglob("*")
        if path.is_file() and path.relative_to(bundle_root).as_posix() != "SEAL.json"
    }
    rows = []
    seen: set[str] = set()
    for item in seal["items"]:
        rel = item.get("path") if isinstance(item, dict) else None
        expected = item.get("sha256") if isinstance(item, dict) else None
        path = _bundle_file(bundle, rel)
        canonical_rel = None
        if path is not None:
            canonical_rel = path.relative_to(bundle_root).as_posix()
        actual = _sha256(path) if path is not None and path.is_file() else None
        malformed = not isinstance(item, dict) or not isinstance(rel, str) or not rel
        malformed = malformed or not isinstance(expected, str) or len(expected) != 64
        malformed = malformed or (isinstance(expected, str) and any(char not in "0123456789abcdefABCDEF" for char in expected))
        path_error = (path is None or canonical_rel != rel or canonical_rel not in sealed_files)
        duplicate = isinstance(rel, str) and rel in seen
        verified = not malformed and not path_error and not duplicate and actual == expected
        rows.append({"path": rel, "expected_sha256": expected, "actual_sha256": actual,
                     "verified": verified,
                     "error": ("malformed entry" if malformed else
                               "duplicate path" if duplicate else
                               "path is outside bundle or not a bundle file" if path_error else None)})
        if isinstance(rel, str):
            seen.add(rel)
    missing_files = sorted(sealed_files - seen)
    unexpected_files = sorted(seen - sealed_files)
    evidence = {"seal": str(seal_path), "files": rows,
                "bundle_files": sorted(sealed_files),
                "missing_files": missing_files,
                "unsealed_files": missing_files,
                "unexpected_files": unexpected_files}
    return bool(rows) and all(row["verified"] for row in rows) and not missing_files and not unexpected_files, evidence


def _find_app(bundle: Path) -> Path | None:
    for candidate in APP_CANDIDATES:
        path = bundle / candidate
        if path.is_file() and os.access(path, os.X_OK):
            return path
    return None


def _installed_packs(bundle: Path) -> tuple[list[str], dict]:
    manifest, error = _load_json(bundle / "RC-MANIFEST.json")
    if error or not isinstance(manifest, dict):
        return [], {"error": error or "manifest missing"}
    rows, names = [], []
    packs = manifest.get("packs", [])
    if not isinstance(packs, list):
        return [], {"error": "packs must be an array"}
    for pack in packs:
        if not isinstance(pack, dict):
            rows.append({"name": None, "manifest": None, "present": False})
            continue
        name = pack.get("name")
        location = _relative_rc_path(str(pack.get("location", "")))
        pack_root = _bundle_file(bundle, location)
        pack_manifest = pack_root / "PACK.json" if pack_root else None
        present = pack_manifest is not None and pack_manifest.is_file()
        rows.append({"name": name, "manifest": str(pack_manifest) if pack_manifest else None, "present": present})
        if present and isinstance(name, str):
            names.append(name)
    return names, {"packs": rows}


def _samples(bundle: Path) -> tuple[bool, list[dict]]:
    rows = []
    for sample, lane in EXPECTED_SAMPLES:
        root = bundle / "samples" / "v2" / sample
        project, rights = root / "project.json", root / "sources" / "manifest.json"
        rows.append({"sample": sample, "lane": lane, "project": str(project),
                     "project_present": project.is_file(), "rights_manifest_present": rights.is_file()})
    return all(row["project_present"] and row["rights_manifest_present"] for row in rows), rows


def _network_runner() -> tuple[list[str], dict]:
    sandbox = Path("/usr/bin/sandbox-exec")
    if platform.system() == "Darwin" and sandbox.is_file():
        return [str(sandbox), "-p", NETWORK_PROFILE], {"enforced": True, "mechanism": "sandbox-exec",
                                                         "policy": NETWORK_PROFILE}
    return [], {"enforced": False, "mechanism": "unavailable",
                "policy": "no supported local network sandbox found"}


def _clean_env() -> dict[str, str]:
    # Deliberately do not inherit caller PATH or arbitrary developer settings.
    return {"PATH": "", "HOME": str(Path.cwd() / ".clean-machine-home"),
            "CUTRIGHT_NETWORK_POLICY": "deny", "CUTRIGHT_TELEMETRY": "off"}


def _not_run(sample: str, lane: str, reason: str) -> dict:
    return {"sample": sample, "lane": lane, "outcome": "not_run", "reason": reason,
            "command": [], "exit_code": None, "network_attempt_total": 0,
            "pack_ids": [], "lifecycle": {key: False for key in LIFECYCLES}}


def _run_sample(app: Path, prefix: list[str], sample: str, lane: str, pack_ids: list[str]) -> dict:
    if app.name in ("videoctl", "videoctl.exe"):
        command = prefix + [str(app), "clean-machine-sample",
                            str(app.parents[1] / "samples" / "v2" / sample / "project.json"),
                            "--sample", sample, "--lane", lane,
                            "--pack-id", ",".join(pack_ids), "--network-deny", "--lifecycle",
                            "correction_undo,restart_resume,repair_rollback,uninstall_preservation"]
    else:
        command = prefix + [str(app), "--clean-machine-sample", sample, "--network-deny", "--lifecycle",
                            "correction-undo,restart-resume,repair-rollback,uninstall-preservation"]
    try:
        completed = subprocess.run(command, env=_clean_env(), capture_output=True, text=True, timeout=120)
        payload = json.loads(completed.stdout)
    except (OSError, subprocess.TimeoutExpired, json.JSONDecodeError) as exc:
        return _not_run(sample, lane, f"application protocol failed: {exc}")
    if not isinstance(payload, dict):
        return _not_run(sample, lane, "application protocol failed: response was not an object")
    lifecycle = payload.get("lifecycle") if isinstance(payload, dict) else {}
    lifecycle = lifecycle if isinstance(lifecycle, dict) else {}
    normalized = {key: lifecycle.get(key) is True for key in LIFECYCLES}
    reported_packs = payload.get("pack_ids")
    network_attempts = payload.get("network_attempt_total")
    valid_packs = isinstance(reported_packs, list) and all(isinstance(pack, str) for pack in reported_packs)
    valid_attempts = isinstance(network_attempts, int) and not isinstance(network_attempts, bool)
    passed = (completed.returncode == 0 and payload.get("sample") == sample and payload.get("lane") == lane
              and payload.get("state") == "ready_review" and valid_attempts and network_attempts == 0
              and valid_packs and sorted(reported_packs) == sorted(pack_ids) and all(normalized.values()))
    return {"sample": sample, "lane": lane, "outcome": "ready_review" if passed else "failed",
            "reason": "" if passed else "application result did not meet clean-machine protocol",
            "command": command, "exit_code": completed.returncode,
            "network_attempt_total": network_attempts if valid_attempts and network_attempts >= 0 else 0,
            "pack_ids": reported_packs if valid_packs else [], "lifecycle": normalized}


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="v2 exact-RC clean-machine harness")
    parser.add_argument("--target", required=True)
    parser.add_argument("--bundle", required=True)
    parser.add_argument("--result", required=True)
    parser.add_argument("--fresh-os-user", action="store_true",
                        help="set only from provisioned fresh-user runner")
    args = parser.parse_args(argv)
    bundle = Path(args.bundle).resolve()
    app = _find_app(bundle)
    hashes_ok, hash_evidence = _verify_exact_hashes(bundle)
    pack_ids, pack_evidence = _installed_packs(bundle)
    packs_ok = bool(pack_ids) and all(row["present"] for row in pack_evidence.get("packs", []))
    samples_ok, sample_evidence = _samples(bundle)
    prefix, network_evidence = _network_runner()
    preconditions = {"fresh_os_user": args.fresh_os_user, "isolated_runner": bool(prefix), "runtime_path": "",
                     "blocked_network": {**network_evidence, "network_attempt_total": 0}}
    checks = [
        _record("fresh_os_user", args.fresh_os_user,
                {"attested_by_runner": args.fresh_os_user}),
        _record("sanitized_runtime_path", _clean_env()["PATH"] == "", {"runtime_path": "", "host_path_read": False}),
        _record("blocked_network_policy", bool(network_evidence["enforced"]), network_evidence),
        _record("exact_bundle_hashes", hashes_ok, hash_evidence),
        _record("bundled_application", app is not None, {"candidates": list(APP_CANDIDATES), "selected": str(app) if app else None}),
        _record("bundled_packs", packs_ok, pack_evidence),
        _record("bundled_four_samples", samples_ok, {"samples": sample_evidence}),
    ]
    ready = all(check["passed"] for check in checks)
    if ready:
        results = [_run_sample(app, prefix, sample, lane, pack_ids) for sample, lane in EXPECTED_SAMPLES]
    else:
        reason = "preconditions failed; application lifecycle was not executed"
        results = [_not_run(sample, lane, reason) for sample, lane in EXPECTED_SAMPLES]
    lifecycle_ok = len(results) == 4 and all(row["outcome"] == "ready_review" for row in results)
    checks.append(_record("four_sample_lifecycles", lifecycle_ok, {"results": results}))
    overall = all(check["passed"] for check in checks)
    result = {"schema_version": "v2", "target": args.target,
              "ran_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "platform": {"system": platform.system(), "release": platform.release(),
                           "machine": platform.machine(), "python": sys.version.split()[0]},
              "preconditions": preconditions, "external_runtime_dependencies": 0,
              "network_attempts": sum(row["network_attempt_total"] for row in results),
              "ci": "forbidden", "publish": False, "upload_status": "not_performed",
              "overall_passed": overall, "checks": checks, "sample_results": results,
              "blocked_by": None if overall else "exact RC lacks required clean-machine payload or isolation evidence"}
    out = Path(args.result).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"wrote {out} (overall_passed={overall})")
    return 0 if overall else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
