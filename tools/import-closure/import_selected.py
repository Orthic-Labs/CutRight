#!/usr/bin/env python3
"""import_selected.py — selection-driven pinned import into CutRight.

Reads a selection JSON, extracts the pinned paths from the source git
repository via `git archive`, copies them into the destination root, and
writes the closure graph plus the frozen import receipt.

Selection format:
{
  "schema_version": 1,
  "selection_id": "designer",
  "source_id": "workspace-capabilities",
  "revision_type": "commit",
  "source_repo": "/Volumes/D/claude",
  "revision": "6ee21f03...",
  "include_paths": ["tools/skills/designer"],
  "strip_prefix": "tools/skills",
  "dest": "skills/",
  "graph": "imports/v2/graphs/designer.json",
  "receipt": "imports/v2/receipts/designer.json",
  "imported_by_task": "CR-V2-B1-007"
}
"""

from __future__ import annotations

import datetime
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import fail, hash_tree, read_json, repo_root, write_json  # noqa: E402


def git(source_repo: Path, *args: str) -> bytes:
    return subprocess.run(
        ["git", "-C", str(source_repo), *args],
        check=True,
        capture_output=True,
    ).stdout


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    selection = read_json(Path(argv[1]))
    for field in ("selection_id", "source_id", "revision_type", "source_repo", "revision", "include_paths", "dest", "graph", "receipt", "imported_by_task"):
        if field not in selection:
            fail(f"selection {argv[1]} missing field {field}")

    root = repo_root()
    source_repo = Path(selection["source_repo"]).resolve()
    revision = selection["revision"]
    if not source_repo.is_dir():
        fail(f"source repository not found: {source_repo}")
    try:
        git(source_repo, "cat-file", "-e", f"{revision}^{{commit}}")
    except subprocess.CalledProcessError:
        fail(f"revision {revision} not present in {source_repo}")

    strip_prefix = selection.get("strip_prefix", "").strip("/")
    dest_root = (root / selection["dest"].strip("/")).resolve()
    if root not in dest_root.parents and dest_root != root:
        fail(f"destination escapes the repository: {dest_root}")

    with tempfile.TemporaryDirectory(prefix="cr-v2-import-") as staging:
        staging_path = Path(staging)
        tar = subprocess.run(
            ["git", "-C", str(source_repo), "archive", "--format=tar", revision, *selection["include_paths"]],
            check=True,
            capture_output=True,
        )
        subprocess.run(["tar", "-x", "-C", str(staging_path)], input=tar.stdout, check=True)

        written = 0
        for include in selection["include_paths"]:
            src = staging_path / include
            if not src.exists():
                fail(f"pinned revision lacks expected path: {include}")
            rel = include[len(strip_prefix) + 1 :] if strip_prefix and include.startswith(strip_prefix + "/") else include
            target = dest_root / rel if selection.get("merge_under_dest", True) else dest_root
            if src.is_dir():
                target.parent.mkdir(parents=True, exist_ok=True)
                if target.exists():
                    shutil.rmtree(target)
                shutil.copytree(src, target, symlinks=False)
                written += sum(1 for _ in target.rglob("*") if _.is_file())
            else:
                target.mkdir(parents=True, exist_ok=True)
                shutil.copy2(src, target / src.name)
                written += 1

    graph = hash_tree(dest_root)
    graph["selection_id"] = selection["selection_id"]
    graph["source_id"] = selection["source_id"]
    graph["revision"] = revision
    write_json(root / selection["graph"], graph)

    receipt = {
        "schema_version": 1,
        "source_id": selection["source_id"],
        "revision_type": selection["revision_type"],
        "revision": revision,
        "destination": selection["dest"],
        "file_count": graph["file_count"],
        "total_bytes": graph["total_bytes"],
        "sha256_of_sorted_path_list": graph["sha256_of_sorted_path_list"],
        "imported_by_task": selection["imported_by_task"],
        "imported_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "selection": {
            "selection_id": selection["selection_id"],
            "source_repo": str(source_repo),
            "include_paths": selection["include_paths"],
            "strip_prefix": strip_prefix,
        },
    }
    write_json(root / selection["receipt"], receipt)
    print(f"imported {graph['file_count']} files into {selection['dest']}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
