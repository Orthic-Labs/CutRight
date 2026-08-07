#!/usr/bin/env python3
"""build-corresponding-source.py — deterministic corresponding-source archives.

CutRight v2 Book 1 (CR-V2-B1-022). Builds corresponding-source archives for
binary-runtime components (FFmpeg and any future reciprocal-licence row) from
PINNED LOCAL source snapshots. This script never touches the network: every
input is a path inside the repository.

Archive layout (see runtime/source/README.md):

    runtime-source/<component>/<version>/<target>.tar.zst   (or .tar fallback)
    runtime-source/<component>/<version>/<target>.manifest.json

Manifest contract — every binary-runtime row MUST carry all of:

    source_revision   immutable revision the bytes were built from
    configure_args    exact build configuration used for the shipped binary
    patches           list of local patch files applied (may be empty, must exist)
    source_sha256     SHA-256 of the generated corresponding-source archive
    binary_sha256     SHA-256 of the shipped binary the archive corresponds to
    notice_path       path to the THIRD_PARTY.yml notice governing the component

A row missing any of those fields fails. Determinism: tar entries are sorted,
mtime/uid/gid are fixed, and compression (when available) is single-threaded
at a fixed level.

Compression: the stdlib `compression.zstd` module (Python >= 3.14) is used
when importable, producing `.tar.zst`. When it is unavailable offline, the
documented deterministic fallback is an uncompressed `.tar` with the same
entry order and metadata; the manifest contract is unchanged either way.
"""

from __future__ import annotations

import argparse
import hashlib
import io
import json
import sys
import tarfile
import tempfile
from pathlib import Path

try:  # Python 3.14+ stdlib zstd
    from compression import zstd as _zstd  # type: ignore
except ImportError:  # pragma: no cover - offline fallback
    try:
        import zstandard as _zstd  # type: ignore
    except ImportError:
        _zstd = None

ZSTD_LEVEL = 3
FIXED_MTIME = 0
FIXED_UID_GID = 0

# Every binary-runtime row must carry these fields; absence of any is fatal.
REQUIRED_ROW_FIELDS = (
    "component",
    "version",
    "target",
    "source_root",
    "source_revision",
    "configure_args",
    "patches",
    "source_sha256",
    "binary_sha256",
    "notice_path",
)


class RowError(Exception):
    """A binary-runtime row violates the manifest contract."""


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def validate_row(row: dict, repo_root: Path) -> None:
    """Reject a binary-runtime row missing any contract field."""
    if not isinstance(row, dict):
        raise RowError("row is not a JSON object")
    name = row.get("component", "<unnamed row>")
    for field in REQUIRED_ROW_FIELDS:
        if field not in row or row[field] is None:
            raise RowError(f"{name}: missing required field '{field}'")
        value = row[field]
        if isinstance(value, str) and not value.strip():
            raise RowError(f"{name}: required field '{field}' is empty")
    if not isinstance(row["configure_args"], list):
        raise RowError(f"{name}: configure_args must be a list")
    if not isinstance(row["patches"], list):
        raise RowError(f"{name}: patches must be a list")
    for field in ("source_sha256", "binary_sha256"):
        if not (isinstance(row[field], str) and len(row[field]) == 64
                and all(c in "0123456789abcdef" for c in row[field])):
            raise RowError(f"{name}: {field} must be 64 lowercase hex chars")
    source_root = (repo_root / row["source_root"]).resolve()
    if repo_root.resolve() not in source_root.parents and source_root != repo_root.resolve():
        raise RowError(f"{name}: source_root escapes the repository")
    if not source_root.is_dir():
        raise RowError(f"{name}: source_root does not exist: {row['source_root']}")
    notice = (repo_root / row["notice_path"]).resolve()
    if not notice.is_file():
        raise RowError(f"{name}: notice_path does not exist: {row['notice_path']}")
    for patch in row["patches"]:
        patch_path = (repo_root / patch).resolve()
        if not patch_path.is_file():
            raise RowError(f"{name}: patch does not exist: {patch}")


def build_deterministic_tar(source_root: Path) -> bytes:
    """Tar the pinned local snapshot with sorted entries and fixed metadata."""
    source_root = source_root.resolve()
    entries: list[tuple[str, Path]] = []
    for path in source_root.rglob("*"):
        rel = path.relative_to(source_root).as_posix()
        entries.append((rel, path))
    entries.sort(key=lambda item: item[0])
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.USTAR_FORMAT) as tar:
        for rel, path in entries:
            info = tar.gettarinfo(name=str(path), arcname=rel)
            info.uid = FIXED_UID_GID
            info.gid = FIXED_UID_GID
            info.uname = ""
            info.gname = ""
            info.mtime = FIXED_MTIME
            if info.isdir():
                info.mode = 0o755
                tar.addfile(info)
            elif info.isreg():
                info.mode = 0o644
                with open(path, "rb") as fh:
                    tar.addfile(info, fh)
            else:
                raise RowError(f"unsupported entry type in snapshot: {rel}")
    return buf.getvalue()


def compress_archive(tar_bytes: bytes) -> tuple[bytes, str]:
    """Compress with stdlib zstd when available; else deterministic .tar fallback."""
    if _zstd is not None:
        data = _zstd.compress(tar_bytes, level=ZSTD_LEVEL)
        return data, ".tar.zst"
    return tar_bytes, ".tar"


def build_archive(row: dict, repo_root: Path, out_dir: Path) -> dict:
    """Generate the corresponding-source archive + manifest for one row."""
    validate_row(row, repo_root)
    source_root = (repo_root / row["source_root"]).resolve()
    tar_bytes = build_deterministic_tar(source_root)
    archive_bytes, suffix = compress_archive(tar_bytes)
    source_sha256 = sha256_bytes(archive_bytes)
    out_dir.mkdir(parents=True, exist_ok=True)
    stem = f"{row['component']}-{row['version']}-{row['target']}"
    archive_path = out_dir / f"{stem}{suffix}"
    archive_path.write_bytes(archive_bytes)
    manifest = {
        "schema_version": 1,
        "component": row["component"],
        "version": row["version"],
        "target": row["target"],
        "source_root": row["source_root"],
        "source_revision": row["source_revision"],
        "configure_args": row["configure_args"],
        "patches": row["patches"],
        "source_sha256": source_sha256,
        "binary_sha256": row["binary_sha256"],
        "notice_path": row["notice_path"],
        "archive": archive_path.name,
        "compression": "zstd-level-3" if suffix == ".tar.zst" else "tar-only-fallback",
        "generator": "scripts/legal/build-corresponding-source.py",
    }
    manifest_path = out_dir / f"{stem}.manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return manifest


def run_manifest(manifest_path: Path, out_root: Path, repo_root: Path, only: str | None) -> int:
    data = json.loads(manifest_path.read_text(encoding="utf-8"))
    rows = data.get("rows", [])
    if not rows:
        print("FAIL: manifest contains no binary-runtime rows", file=sys.stderr)
        return 1
    built = 0
    for row in rows:
        if only and row.get("component") != only:
            continue
        try:
            out_dir = out_root / row["component"] / row["version"]
            manifest = build_archive(row, repo_root, out_dir)
        except RowError as exc:
            print(f"FAIL: {exc}", file=sys.stderr)
            return 1
        print(f"OK: {manifest['archive']} source_sha256={manifest['source_sha256']}")
        built += 1
    if only and built == 0:
        print(f"FAIL: no row matched component '{only}'", file=sys.stderr)
        return 1
    return 0


def self_test() -> int:
    """Fixtures in a temp dir: byte-identical archives, hard row failures."""
    with tempfile.TemporaryDirectory(prefix="cr-corresponding-source-") as tmp:
        repo = Path(tmp)
        src = repo / "runtime/source/demo"
        (src / "sub").mkdir(parents=True)
        (src / "README").write_text("demo source snapshot\n", encoding="utf-8")
        (src / "sub" / "main.c").write_text("int main(void){return 0;}\n", encoding="utf-8")
        notice = repo / "notices/demo/THIRD_PARTY.yml"
        notice.parent.mkdir(parents=True)
        notice.write_text("schema_version: 1\n", encoding="utf-8")

        binary_hash = "ab" * 32
        good_row = {
            "component": "demo",
            "version": "1.0",
            "target": "self-test",
            "source_root": "runtime/source/demo",
            "source_revision": "0" * 40,
            "configure_args": ["--disable-network"],
            "patches": [],
            "source_sha256": "0" * 64,  # recomputed from the generated archive
            "binary_sha256": binary_hash,
            "notice_path": "notices/demo/THIRD_PARTY.yml",
        }

        # Two independent builds must be byte-identical.
        first = build_archive(dict(good_row), repo, repo / "out-a")
        second = build_archive(dict(good_row), repo, repo / "out-b")
        a = (repo / "out-a" / first["archive"]).read_bytes()
        b = (repo / "out-b" / second["archive"]).read_bytes()
        assert a == b, "archive bytes differ between identical builds"
        assert first["source_sha256"] == second["source_sha256"] == sha256_bytes(a)
        assert first["binary_sha256"] == binary_hash
        ma = (repo / "out-a" / "demo-1.0-self-test.manifest.json").read_text(encoding="utf-8")
        mb = (repo / "out-b" / "demo-1.0-self-test.manifest.json").read_text(encoding="utf-8")
        assert ma == mb, "manifest bytes differ between identical builds"

        # A row missing source_revision must fail.
        missing_source = dict(good_row)
        del missing_source["source_revision"]
        try:
            validate_row(missing_source, repo)
        except RowError:
            pass
        else:
            print("FAIL: row without source_revision was accepted", file=sys.stderr)
            return 1

        # A row missing its notice must fail.
        missing_notice = dict(good_row)
        missing_notice["notice_path"] = "notices/absent/THIRD_PARTY.yml"
        try:
            validate_row(missing_notice, repo)
        except RowError:
            pass
        else:
            print("FAIL: row without a real notice_path was accepted", file=sys.stderr)
            return 1

        # Every other required field, removed one at a time, must also fail.
        for field in REQUIRED_ROW_FIELDS:
            broken = dict(good_row)
            del broken[field]
            try:
                validate_row(broken, repo)
            except RowError:
                continue
            print(f"FAIL: row without '{field}' was accepted", file=sys.stderr)
            return 1

    print(f"OK: self-test passed (compression={'zstd' if _zstd is not None else 'tar-only fallback'})")
    return 0


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--self-test", action="store_true",
                        help="run deterministic fixture self-test and exit")
    parser.add_argument("--manifest", type=Path,
                        help="JSON manifest with binary-runtime rows")
    parser.add_argument("--out", type=Path, default=Path("runtime-source"),
                        help="output root (default: runtime-source/)")
    parser.add_argument("--repo-root", type=Path, default=Path("."),
                        help="repository root all row paths resolve against")
    parser.add_argument("--component", help="build only this component")
    args = parser.parse_args(argv[1:])
    if args.self_test:
        return self_test()
    if not args.manifest:
        parser.error("--manifest is required unless --self-test is given")
    return run_manifest(args.manifest, args.out, args.repo_root, args.component)


if __name__ == "__main__":
    sys.exit(main(sys.argv))
