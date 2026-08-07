#!/usr/bin/env python3
"""Build and assemble a local, offline v2 release candidate.

This script only builds and stages local bytes. Signing, sealing, upload, and
publication remain separate operations.
"""

from __future__ import annotations

import argparse
from collections.abc import Mapping
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path

FORBIDDEN_ENV_PATTERNS = ("TOKEN", "SECRET", "KEY")
SAMPLE_NAMES = ("recorded-talking-head", "repurpose-podcast", "procedural-explainer", "anchored-product")
PACK_NAMES = ("v2-capability-core", "v2-skill-runtime")
BUILDER_STAGED_PATHS = ("app", "samples", "packs", "corresponding-source", "RC-MANIFEST.json", "BUILD.json")
# A fresh build invalidates artifacts produced by a prior seal.  Keep these
# outside BUILDER_STAGED_PATHS so payload hashing never treats them as inputs.
INVALIDATED_SEAL_PATHS = ("SEAL.json", "checksums.txt")


def _filtered_build_env(env: Mapping[str, str]) -> dict[str, str]:
    return {
        key: value for key, value in env.items()
        if not any(pattern in key.upper() for pattern in FORBIDDEN_ENV_PATTERNS)
    }


def _git_head(root: Path) -> str:
    return subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], check=True,
                          capture_output=True, text=True).stdout.strip()


def _git_dirty(root: Path) -> bool:
    return bool(subprocess.run(
        ["git", "-C", str(root), "status", "--porcelain"], check=True,
        capture_output=True, text=True,
    ).stdout.strip())


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8192), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _clean_staged_payload(out: Path) -> None:
    """Remove stale builder payload and seal artifacts before restaging."""
    for relative in (*BUILDER_STAGED_PATHS, *INVALIDATED_SEAL_PATHS):
        path = out / relative
        if path.is_dir():
            shutil.rmtree(path)
        elif path.exists() or path.is_symlink():
            path.unlink()


def _staged_payload_hashes(out: Path) -> dict[str, str]:
    """Hash final builder payload; BUILD.json describes this coverage, so omits itself."""
    hashes = {}
    for relative in BUILDER_STAGED_PATHS:
        if relative == "BUILD.json":
            continue
        path = out / relative
        if path.is_file():
            hashes[relative] = _sha256(path)
        elif path.is_dir():
            hashes.update({str(item.relative_to(out)): _sha256(item)
                           for item in sorted(path.rglob("*")) if item.is_file()})
    return hashes


def _artifact_dirs(root: Path, target: str, profile: str) -> tuple[Path, ...]:
    target_root = root / "target"
    return ((target_root / profile) if target == "host" else (target_root / target / profile),
            root / "apps" / "studio" / "src-tauri" / "target" / profile)


def _find_executable(root: Path, target: str, profile: str) -> Path:
    names = ("videoctl", "videoctl.exe", "cutright-studio", "cutright-studio.exe", "CutRight")
    tried = []
    for directory in _artifact_dirs(root, target, profile):
        for name in names:
            candidate = directory / name
            tried.append(str(candidate))
            if candidate.is_file() and os.access(candidate, os.X_OK):
                return candidate
    raise FileNotFoundError("no built videoctl/Studio executable; checked: " + ", ".join(tried))


def _copy_payload(root: Path, out: Path, executable: Path, packs_source: Path | None = None) -> None:
    app_dir = out / "app"
    app_dir.mkdir(parents=True, exist_ok=True)
    destination = app_dir / executable.name
    if destination.exists():
        destination.unlink()
    shutil.copy2(executable, destination)
    destination.chmod(destination.stat().st_mode | 0o111)

    samples_source, samples_destination = root / "samples" / "v2", out / "samples" / "v2"
    missing = [name for name in SAMPLE_NAMES if not (samples_source / name).is_dir()]
    if missing:
        raise FileNotFoundError(f"required samples missing: {', '.join(missing)}")
    if samples_destination.exists():
        shutil.rmtree(samples_destination)
    samples_destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(samples_source, samples_destination)

    packs_source = packs_source or root / "release" / "v2" / "rc" / "packs"
    packs_destination = out / "packs"
    missing_packs = [name for name in PACK_NAMES if not (packs_source / name / "PACK.json").is_file()]
    if missing_packs:
        raise FileNotFoundError(f"required packs missing: {', '.join(missing_packs)}")
    if packs_source.resolve() != packs_destination.resolve():
        if packs_destination.exists():
            shutil.rmtree(packs_destination)
        shutil.copytree(packs_source, packs_destination)

    source = root / "release" / "v2" / "staging" / "corresponding-source"
    destination = out / "corresponding-source"
    if not source.is_dir():
        raise FileNotFoundError("required corresponding-source bundle missing")
    if source.resolve() != destination.resolve():
        if destination.exists():
            shutil.rmtree(destination)
        shutil.copytree(source, destination)


def _write_candidate_manifest(root: Path, out: Path, head: str) -> None:
    template = root / "release" / "v2" / "RC-MANIFEST.json"
    manifest = json.loads(template.read_text(encoding="utf-8"))
    manifest.update({
        "head": head[:7],
        "head_full": head,
        "verified": False,
        "build_command": "python3 scripts/release/v2-build.py --profile release --target host --out release/v2/rc",
        "seal_command": "python3 scripts/release/v2-seal.py --seal release/v2/rc --manifest release/v2/RC-MANIFEST.json",
        "verify_command": "python3 scripts/release/v2-seal.py --verify release/v2/rc",
    })
    manifest["samples"] = [f"release/v2/rc/samples/v2/{name}/" for name in SAMPLE_NAMES]
    (out / "RC-MANIFEST.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def assemble(root: Path, out: Path, target: str, cargo: str = "cargo",
             env: Mapping[str, str] | None = None, profile: str = "release") -> dict:
    command = [cargo, "build", "--profile", profile, "-p", "videoctl"]
    if target != "host":
        command.extend(["--target", target])
    source_env = env if env is not None else os.environ
    subprocess.run(command, cwd=root, check=True, env=_filtered_build_env(source_env))
    executable = _find_executable(root, target, profile)
    packs_source = root / "release" / "v2" / "rc" / "packs"
    if packs_source.resolve() == (out / "packs").resolve():
        with tempfile.TemporaryDirectory() as temporary:
            snapshot = Path(temporary) / "packs"
            shutil.copytree(packs_source, snapshot)
            _clean_staged_payload(out)
            _copy_payload(root, out, executable, snapshot)
    else:
        _clean_staged_payload(out)
        _copy_payload(root, out, executable)
    return {"executable": str(executable.relative_to(root))}


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="v2 deterministic local RC build")
    parser.add_argument("--profile", default="release")
    parser.add_argument("--target", default="host")
    parser.add_argument("--out", required=True)
    parser.add_argument("--source", default=".")
    parser.add_argument("--cargo", default="cargo", help="cargo executable used only for local build")
    args = parser.parse_args(argv)
    root, out = Path(args.source).resolve(), Path(args.out).resolve()
    out.mkdir(parents=True, exist_ok=True)
    started_at = datetime.now(timezone.utc).isoformat()
    head = _git_head(root)
    assembly = assemble(root, out, args.target, args.cargo, _filtered_build_env(os.environ), args.profile)
    _write_candidate_manifest(root, out, head)
    build_meta = {"schema_version": "v2", "profile": args.profile, "target": args.target,
                  "head": head, "source_dirty": _git_dirty(root), "started_at": started_at, **assembly,
                  "staged_payload_hashes": _staged_payload_hashes(out),
                  "staged_payload_coverage": {
                      "builder_owned_paths": list(BUILDER_STAGED_PATHS),
                      "unhashed_self": "BUILD.json",
                      "reason": "BUILD.json contains these hashes and is covered by the subsequent seal",
                  },
                  "finished_at": datetime.now(timezone.utc).isoformat()}
    (out / "BUILD.json").write_text(json.dumps(build_meta, indent=2, sort_keys=True) + "\n")
    print(f"v2 build assembled at {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
