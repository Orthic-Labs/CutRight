#!/usr/bin/env python3
"""scripts/gates/v2-runtime-boundary.py — CutRight v2 runtime boundary guard.

Release code may resolve executables only from signed CutRight runtime pack
paths. Bare executable resolution (ffmpeg, ffprobe, python, python3, node,
heardright-engine) in release Rust/TypeScript/JSON/TOML/shell sources is a
violation. Tests, generated files, and provenance paths are excluded through
config/v2-runtime-boundary-allowlist.txt.

Usage:
  python3 scripts/gates/v2-runtime-boundary.py --check [--root PATH]
  python3 scripts/gates/v2-runtime-boundary.py --self-test

Exit 0 when the tree passes, 1 on any violation, 2 on self-test failure.
Created by CR-V2-B1-005.
"""

from __future__ import annotations

import json
import os
import re
import sys
import tempfile
from pathlib import Path

FORBIDDEN = ("ffmpeg", "ffprobe", "python", "python3", "node", "heardright-engine")
EXEC_KEYS = {"executable", "command", "bin", "program", "cmd"}
SCAN_EXTENSIONS = {".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs", ".json", ".toml", ".sh", ".bash"}

RUST_BARE_EXEC = re.compile(
    r'Command::new\(\s*"(ffmpeg|ffprobe|python3?|node|heardright-engine)"'
)
JS_BARE_EXEC = re.compile(
    r"(?:spawn|spawnSync|exec|execSync|execFile|execFileSync)\(\s*[\"'](ffmpeg|ffprobe|python3?|node|heardright-engine)[\"']"
)
SHELL_BARE_EXEC = re.compile(
    r"(?:^|[|;&]\s*)(ffmpeg|ffprobe|python3?|node)\s"
)
KV_BARE_EXEC = re.compile(
    r'(?i)^\s*[\w-]*(' + "|".join(sorted(EXEC_KEYS)) + r')[\w-]*\s*[=:]\s*"(ffmpeg|ffprobe|python3?|node|heardright-engine)"'
)


def default_repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent


def load_allowlist(root: Path) -> tuple[list[str], set[str]]:
    path = root / "config" / "v2-runtime-boundary-allowlist.txt"
    prefixes: list[str] = []
    filenames: set[str] = set()
    if not path.exists():
        return prefixes, filenames
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.endswith("/"):
            prefixes.append(line)
        else:
            filenames.add(line)
    return prefixes, filenames


def is_allowlisted(rel: str, name: str, prefixes: list[str], filenames: set[str]) -> bool:
    if name in filenames:
        return True
    return any(rel.startswith(prefix) for prefix in prefixes)


def mask_comments_and_strings(text: str, strings_too: bool) -> str:
    """Blank out comments (and optionally string/char literal bodies) while
    preserving offsets and newlines, so line numbers stay stable."""
    out = list(text)
    i, n = 0, len(text)
    state = "code"
    while i < n:
        ch = text[i]
        nxt = text[i + 1] if i + 1 < n else ""
        if state == "code":
            if ch == "/" and nxt == "/":
                state = "line_comment"
                out[i] = out[i + 1] = " "
                i += 2
                continue
            if ch == "/" and nxt == "*":
                state = "block_comment"
                out[i] = out[i + 1] = " "
                i += 2
                continue
            if ch == '"':
                state = "string"
                i += 1
                continue
            if ch == "'":
                # Distinguish char literals ('x', '\n') from lifetimes
                # ('static): lifetimes stay code, otherwise every quote in
                # the rest of the file would be masked as one char literal.
                if i + 2 < n and text[i + 2] == "'":
                    state = "char"
                    i += 1
                    continue
                if nxt == "\\":
                    close = text.find("'", i + 2)
                    if close != -1 and close - i <= 5:
                        state = "char"
                        i += 1
                        continue
                # Lifetime: treat as ordinary code.
        elif state == "line_comment":
            if ch == "\n":
                state = "code"
            else:
                out[i] = " "
        elif state == "block_comment":
            if ch == "*" and nxt == "/":
                out[i] = out[i + 1] = " "
                state = "code"
                i += 2
                continue
            if ch != "\n":
                out[i] = " "
        elif state == "string":
            if ch == "\\" and i + 1 < n:
                if strings_too:
                    out[i] = out[i + 1] = " "
                i += 2
                continue
            if ch == '"':
                state = "code"
            elif strings_too and ch != "\n":
                out[i] = " "
        elif state == "char":
            if ch == "\\" and i + 1 < n:
                if strings_too:
                    out[i] = out[i + 1] = " "
                i += 2
                continue
            if ch == "'":
                state = "code"
            elif strings_too and ch != "\n":
                out[i] = " "
        i += 1
    return "".join(out)


def rust_test_region_lines(masked: str) -> set[int]:
    """Line numbers inside #[cfg(test)] modules, using the string-and-comment
    masked text for brace tracking."""
    test_lines: set[int] = set()
    depth = 0
    pending_test_start: int | None = None
    test_depth: int | None = None
    for lineno, line in enumerate(masked.splitlines(), start=1):
        stripped = line.strip()
        if "#[cfg(test)]" in stripped:
            pending_test_start = depth
        for ch in line:
            if ch == "{":
                depth += 1
                if pending_test_start is not None and test_depth is None:
                    test_depth = depth
            elif ch == "}":
                if test_depth is not None and depth == test_depth:
                    test_depth = None
                    pending_test_start = None
                depth -= 1
        if test_depth is not None or (pending_test_start is not None and "#[cfg(test)]" in stripped):
            test_lines.add(lineno)
        elif test_depth is None and pending_test_start is not None and stripped and "#[cfg(test)]" not in stripped and not stripped.startswith("#["):
            # Attribute did not open a module within a couple of lines; drop it.
            pending_test_start = None
    return test_lines


def scan_rust(path: Path, text: str) -> list[str]:
    no_comments = mask_comments_and_strings(text, strings_too=False)
    masked = mask_comments_and_strings(text, strings_too=True)
    test_lines = rust_test_region_lines(masked)
    hits = []
    for lineno, line in enumerate(no_comments.splitlines(), start=1):
        if lineno in test_lines:
            continue
        for match in RUST_BARE_EXEC.finditer(line):
            hits.append(f"{path}:{lineno}: bare executable resolution {match.group(0)!r} (release code may resolve only signed CutRight pack paths)")
    return hits


def scan_script(path: Path, text: str, pattern: re.Pattern) -> list[str]:
    no_comments = mask_comments_and_strings(text, strings_too=False)
    hits = []
    for lineno, line in enumerate(no_comments.splitlines(), start=1):
        for match in pattern.finditer(line):
            hits.append(f"{path}:{lineno}: bare executable resolution of {match.group(1)!r} (release code may resolve only signed CutRight pack paths)")
    return hits


def scan_json(path: Path, text: str) -> list[str]:
    try:
        doc = json.loads(text)
    except json.JSONDecodeError:
        return []
    hits = []

    def walk(node) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key.lower() in EXEC_KEYS and isinstance(value, str) and value in FORBIDDEN:
                    hits.append(f"{path}: executable key {key!r} resolves bare {value!r} (release code may resolve only signed CutRight pack paths)")
                walk(value)
        elif isinstance(node, list):
            for item in node:
                walk(item)

    walk(doc)
    return hits


def scan_toml(path: Path, text: str) -> list[str]:
    hits = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        match = KV_BARE_EXEC.match(line)
        if match:
            hits.append(f"{path}:{lineno}: executable key resolves bare {match.group(2)!r} (release code may resolve only signed CutRight pack paths)")
    return hits


def scan_file(path: Path, rel: str) -> list[str]:
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        return []
    ext = path.suffix.lower()
    if ext == ".rs":
        return scan_rust(rel, text)
    if ext in {".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"}:
        return scan_script(rel, text, JS_BARE_EXEC)
    if ext == ".json":
        return scan_json(rel, text)
    if ext == ".toml":
        return scan_toml(rel, text)
    if ext in {".sh", ".bash"}:
        return scan_script(rel, text, SHELL_BARE_EXEC)
    return []


def scan_tree(root: Path) -> list[str]:
    prefixes, filenames = load_allowlist(root)
    hits: list[str] = []
    for dirpath, dirnames, filenames_here in os.walk(root):
        rel_dir = os.path.relpath(dirpath, root)
        rel_dir = "" if rel_dir == "." else rel_dir.replace(os.sep, "/") + "/"
        dirnames[:] = sorted(d for d in dirnames if not is_allowlisted(rel_dir + d + "/", d, prefixes, filenames) and d not in {".git", "target", "node_modules"})
        for name in sorted(filenames_here):
            rel = rel_dir + name
            if is_allowlisted(rel, name, prefixes, filenames):
                continue
            if Path(name).suffix.lower() not in SCAN_EXTENSIONS:
                continue
            hits.extend(scan_file(Path(dirpath) / name, rel))
    return hits


def self_test() -> int:
    failures = 0
    cases = {
        "rust-release": ('x.rs', 'use std::process::Command;\nfn main() { let _ = Command::new("ffmpeg").output(); }\n', True),
        "rust-test-region": ('x.rs', '#[cfg(test)]\nmod tests {\n    use std::process::Command;\n    #[test]\n    fn t() { let _ = Command::new("ffmpeg"); }\n}\n', False),
        "rust-doc-comment": ('x.rs', '/// Never write `Command::new("ffmpeg")` in release code.\nfn main() {}\n', False),
        "ts-spawn": ('x.ts', 'import { spawn } from "child_process";\nspawn("node", ["script"]);\n', True),
        "json-command-key": ('x.json', '{"executable": "python3"}\n', True),
        "toml-command-key": ('x.toml', '[toolchain]\nexecutable = "ffprobe"\n', True),
        "shell-bare": ('x.sh', '#!/bin/sh\nffmpeg -version\n', True),
        "clean": ('x.rs', 'fn main() { let resolved = pack_path(); spawn_resolved(resolved); }\n', False),
    }
    for name, (filename, content, expect_violation) in cases.items():
        with tempfile.TemporaryDirectory(prefix=f"v2-boundary-{name}-") as tmp:
            root = Path(tmp)
            (root / "config").mkdir()
            (root / "config" / "v2-runtime-boundary-allowlist.txt").write_text("imports/\n", encoding="utf-8")
            (root / filename).write_text(content, encoding="utf-8")
            hits = scan_tree(root)
            if bool(hits) != expect_violation:
                print(f"SELF-TEST FAIL: {name} (hits={hits})", file=sys.stderr)
                failures += 1
            else:
                print(f"self-test ok: {name} {'correctly rejected' if expect_violation else 'correctly accepted'}")
    if failures:
        print(f"v2-runtime-boundary self-test: {failures} failure(s)", file=sys.stderr)
        return 2
    print("v2-runtime-boundary self-test: all cases pass")
    return 0


def main(argv: list[str]) -> int:
    mode = None
    root = default_repo_root()
    i = 0
    while i < len(argv):
        if argv[i] == "--check":
            mode = "check"
        elif argv[i] == "--self-test":
            mode = "self-test"
        elif argv[i] == "--root" and i + 1 < len(argv):
            root = Path(argv[i + 1]).resolve()
            i += 1
        else:
            print(__doc__, file=sys.stderr)
            return 2
        i += 1
    if mode == "self-test":
        return self_test()
    if mode != "check":
        print(__doc__, file=sys.stderr)
        return 2
    hits = scan_tree(root)
    if hits:
        for hit in hits:
            print(f"VIOLATION: {hit}", file=sys.stderr)
        print(f"[FAIL] v2 runtime boundary: {len(hits)} violation(s)", file=sys.stderr)
        return 1
    print("[PASS] v2 runtime boundary")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
