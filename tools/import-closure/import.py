#!/usr/bin/env python3
"""import.py — import a whole directory from a local source git repository.

Usage:
  python3 tools/import-closure/import.py --source SOURCE_ID --root SRC_PATH \
      --dest DEST [--strip-prefix PREFIX] [--revision REV] [--task TASK_ID]

Extracts SRC_PATH (a path inside the source git repository given by
--repo) at the pinned revision and copies it to DEST inside the CutRight
repository, writing the graph and receipt.
"""

from __future__ import annotations

import argparse
import json
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from _common import repo_root  # noqa: E402
from import_selected import main as selected_main  # noqa: E402


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", required=True)
    parser.add_argument("--root", required=True, help="path inside the source repository")
    parser.add_argument("--dest", required=True)
    parser.add_argument("--repo", required=True, help="source git repository containing --root")
    parser.add_argument("--revision", default="HEAD")
    parser.add_argument("--revision-type", default="commit")
    parser.add_argument("--strip-prefix", default=None)
    parser.add_argument("--task", default="unknown")
    parser.add_argument("--selection-out", default=None)
    args = parser.parse_args(argv)

    root = repo_root()
    name = Path(args.dest.rstrip("/")).name
    graph = f"imports/v2/graphs/{name}.json"
    receipt = f"imports/v2/receipts/{name}.json"
    strip = args.strip_prefix if args.strip_prefix is not None else str(Path(args.root).parent)
    selection = {
        "schema_version": 1,
        "selection_id": name,
        "source_id": args.source,
        "revision_type": args.revision_type,
        "source_repo": args.repo,
        "revision": args.revision,
        "include_paths": [args.root.strip("/")],
        "strip_prefix": strip.strip("/"),
        "dest": args.dest,
        "graph": graph,
        "receipt": receipt,
        "imported_by_task": args.task,
    }
    if args.selection_out:
        sel_path = root / args.selection_out
        sel_path.parent.mkdir(parents=True, exist_ok=True)
        sel_path.write_text(json.dumps(selection, indent=2) + "\n", encoding="utf-8")
        print(f"wrote {sel_path}")
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as fh:
        json.dump(selection, fh)
        tmp = fh.name
    return selected_main(["import_selected.py", tmp])


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
