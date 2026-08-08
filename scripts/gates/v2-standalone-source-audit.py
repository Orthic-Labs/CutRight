#!/usr/bin/env python3
"""scripts/gates/v2-standalone-source-audit.py — CutRight v2 standalone source audit.

Proves the source tree has no runtime dependency on another checkout. The
whole repository is scanned for path references, manifest dependencies,
environment variable names, and executable lookups that would only resolve
against a sibling checkout (the parent workspace, HeardRight, AutoShorts,
Vox, or Palmier), a user home directory, or a git submodule.

Findings are classified:
  release_code          severity FAIL — a runtime reference in shipping code
  provenance_citation   severity INFO — provenance/legal files may cite
                        external repositories, source URLs, and commit IDs
  test_code             severity INFO — references confined to test regions
                        and test trees are not runtime dependencies

Exit 0 iff there are zero release_code findings, 1 otherwise, 2 on
self-test failure. Created by CR-V2-B1-026.

Usage:
  python3 scripts/gates/v2-standalone-source-audit.py --root . [--json OUT|--out OUT]
  python3 scripts/gates/v2-standalone-source-audit.py --self-test

JSON report shape:
  {"schema_version": 1, "root": ..., "findings":
   [{"file", "line", "rule_id", "severity", "classification", "snippet"}],
   "summary": {...}}
"""

from __future__ import annotations

import json
import os
import re
import sys
import tempfile
from pathlib import Path

# ---------------------------------------------------------------------------
# Rule definitions
# ---------------------------------------------------------------------------

SIBLING_CHECKOUTS = ("heardright", "autoshorts", "vox-director", "vox", "palmier")
# Bare tool names whose resolution on PATH implies another checkout's build.
SIBLING_TOOLS = ("heardright-engine", "autoshorts", "vox-director", "palmier")

# Environment variable names that point at an external checkout / workspace.
FORBIDDEN_ENV_NAMES = ("CUTRIGHT_HEARDRIGHT_ENGINE", "HEARDRIGHT_ENGINE_BIN")
WORKSPACE_ROOT_ENV = re.compile(r"\b[A-Z][A-Z0-9]*_WORKSPACE_ROOT\b")

# R01 parent workspace; R02 sibling checkout absolute paths.
ABS_PARENT_WORKSPACE = re.compile(r"/Volumes/D/claude\b")
ABS_SIBLING = re.compile(
    r"/Volumes/[A-Za-z0-9._-]+/(heardright|autoshorts|vox-director|vox|palmier|claude)\b"
)
# R03 any other volume mount; R04 home-relative references.
ABS_VOLUMES = re.compile(r"/Volumes/")
HOME_REF = re.compile(r"(?:~|\$HOME|\$\{HOME\})/")
# R07 sibling checkout relative references ("../heardright", "/heardright/",
# "../../claude/x"). An occurrence preceded by "vendor/" is an intra-repo
# vendored tree and is allowed (resolved/validated by R09 separately).
SIBLING_REL = re.compile(
    r"(?:\.\./)+(" + "|".join(SIBLING_CHECKOUTS) + r")\b|/(" + "|".join(SIBLING_CHECKOUTS) + r")/"
)
# Contexts that mark a sibling-name path segment as a provenance/vendored
# citation rather than a live checkout reference.
PROVENANCE_CONTEXT = re.compile(r"(?:imports|vendor|third_party|app|provenance)/$")
# R05 quoted relative ".." paths in code-bearing sources; each is resolved
# against its file and only flagged when it actually escapes the repository
# root. Comment-heavy formats (deny.toml, docs) are excluded from this rule.
QUOTED_DOTDOT = re.compile(r'["\']((?:\.\./)+[^"\']*)["\']')
QUOTED_DOTDOT_EXTENSIONS = {
    ".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs",
    ".json", ".sh", ".bash", ".py", ".yaml", ".yml",
}
# R08 user skill directories cited as live references.
USER_SKILL_DIR = re.compile(r"/tools/skills/")
# R11 a PATH-lookup mention next to a sibling tool name.
PATH_MENTION = re.compile(r"\bPATH\b")

# R12 explicit invocation constructs resolving a bare sibling tool name.
RUST_INVOKE = re.compile(r'Command::new\(\s*"(' + "|".join(SIBLING_TOOLS) + r')"')
JS_INVOKE = re.compile(
    r"(?:spawn|spawnSync|exec|execFile|execSync|execFileSync)\(\s*[\"']("
    + "|".join(SIBLING_TOOLS)
    + r")[\"']"
)
SHELL_INVOKE = re.compile(
    r"(?:^|[|;&]\s*)(" + "|".join(SIBLING_TOOLS) + r")(?:\s|$)"
)

# R09 Cargo path dependencies.
CARGO_PATH_DEP = re.compile(r'\bpath\s*=\s*"([^"]+)"')
# R10 pnpm workspace entries.
PNPM_ENTRY = re.compile(r"^\s*-\s*['\"]?([^#'\"\n]+?)['\"]?\s*$")

SCAN_EXTENSIONS = {
    ".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs",
    ".json", ".toml", ".sh", ".bash", ".py", ".yaml", ".yml",
}
PROVENANCE_PREFIXES = (
    "imports/", "docs/", "third_party/",
)
# Development/QA tooling is part of the repository boundary machinery, not
# the shipped runtime; suspicious references there are reported as INFO.
DEV_TOOLING_PREFIXES = ("tools/",)
TEST_TREE_SEGMENTS = {"tests", "__tests__"}
DEV_TREE_SEGMENTS = {"examples", "benches", "gen"}
SKIP_DIRS = {
    "target", "node_modules", "__pycache__", ".venv", "venv",
    ".pnpm-store", "dist", ".pytest_cache", ".mypy_cache", ".cache",
}
# Guard tools necessarily cite the forbidden patterns they enforce; auditing
# them would report the detector instead of the detected. The legal ledger
# validator likewise names the vendored and third-party trees it polices.
EXEMPT_FILES = {
    "tools/import-closure/assert_no_external_refs.py",
    "scripts/gates/v2-standalone-source-audit.py",
}
EXEMPT_PREFIXES = ("scripts/gates/", "scripts/legal/")

RULE_INFO = {
    "R01-parent-workspace": "absolute reference to the parent workspace /Volumes/D/claude",
    "R02-sibling-checkout-abs": "absolute path into a sibling checkout",
    "R03-volume-mount": "reference to an external volume mount (/Volumes/)",
    "R04-home-ref": "home-relative path reference (~/ or $HOME)",
    "R05-dotdot-escape": "relative '..' path escaping the repository root",
    "R07-sibling-checkout-rel": "relative reference to a sibling checkout",
    "R08-user-skill-dir": "live reference to a user skill directory (/tools/skills/)",
    "R09-cargo-path-escape": "Cargo path dependency escaping the repository root",
    "R10-workspace-escape": "package/workspace entry escaping the repository root",
    "R11-path-lookup": "sibling tool resolved via PATH lookup",
    "R12-invoke-bare": "invocation of a bare sibling tool executable",
    "R13-gitmodules": "git submodule definition or presence",
    "R14-hosted-ci": "hosted CI workflow file",
    "R15-symlink": "symlink under skills/ or vendor/",
    "R16-env-checkout": "environment variable name implying an external checkout",
}


def default_repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent


# ---------------------------------------------------------------------------
# Comment masking (line numbers stay stable) — same convention as
# scripts/gates/v2-runtime-boundary.py.
# ---------------------------------------------------------------------------

def mask_comments(text: str) -> str:
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
            elif ch == "'":
                state = "char_or_lifetime"
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
                i += 2
                continue
            if ch == '"':
                state = "code"
        elif state == "char_or_lifetime":
            # Lifetimes ('static) fall back to code on the next iteration.
            state = "code"
        i += 1
    return "".join(out)


CFG_TEST_ATTR = re.compile(r"#\[cfg\([^\]]*\btest\b[^\]]*\)\]")


def rust_test_region_lines(text: str) -> set[int]:
    """Line numbers inside cfg(test) modules (brace-tracked on the
    comment-masked text). Matches plain #[cfg(test)] and compound forms such
    as #[cfg(all(test, target_os = \"macos\"))]."""
    masked = mask_comments(text)
    test_lines: set[int] = set()
    depth = 0
    pending: int | None = None
    test_depth: int | None = None
    for lineno, line in enumerate(masked.splitlines(), start=1):
        stripped = line.strip()
        is_cfg_test = bool(CFG_TEST_ATTR.search(stripped))
        if is_cfg_test:
            pending = depth
        for ch in line:
            if ch == "{":
                depth += 1
                if pending is not None and test_depth is None:
                    test_depth = depth
            elif ch == "}":
                if test_depth is not None and depth == test_depth:
                    test_depth = None
                    pending = None
                depth -= 1
        if test_depth is not None or (pending is not None and is_cfg_test):
            test_lines.add(lineno)
        elif pending is not None and stripped and not is_cfg_test and not stripped.startswith("#"):
            pending = None
    return test_lines


# ---------------------------------------------------------------------------
# Classification
# ---------------------------------------------------------------------------

def classify_file(rel: str, name: str) -> str:
    if name.endswith((".md", ".markdown")):
        return "provenance_citation"
    if rel.startswith(PROVENANCE_PREFIXES):
        return "provenance_citation"
    if rel.startswith(DEV_TOOLING_PREFIXES):
        return "dev_tooling"
    parts = rel.split("/")
    base = name.lower()
    if any(seg in TEST_TREE_SEGMENTS for seg in parts[:-1]):
        return "test_code"
    if any(seg in DEV_TREE_SEGMENTS for seg in parts[:-1]):
        return "dev_tooling"
    if base.endswith(("_test.py", ".test.ts", ".test.tsx", ".spec.ts", ".spec.tsx")):
        return "test_code"
    return "release_code"


def severity_for(classification: str) -> str:
    return "FAIL" if classification == "release_code" else "INFO"


def is_exempt(rel: str) -> bool:
    if rel in EXEMPT_FILES:
        return True
    return any(rel.startswith(prefix) for prefix in EXEMPT_PREFIXES)


def escapes_root(root: Path, base_dir: Path, target: str) -> bool:
    """True when a manifest/relative path resolved from base_dir leaves root."""
    if target.startswith(("http://", "https://", "git+", "ssh://")):
        return False
    candidate = target[1:] if target.startswith("./") else target
    if os.path.isabs(candidate):
        resolved = Path(candidate)
    else:
        resolved = base_dir / candidate
    try:
        resolved.resolve().relative_to(root)
        return False
    except ValueError:
        return True


# ---------------------------------------------------------------------------
# Line scanners
# ---------------------------------------------------------------------------

def line_pattern_findings(root: Path, base_dir: Path, rel: str, lineno: int, line: str, ext: str) -> list[dict]:
    findings = []

    def add(rule: str, snippet: str) -> None:
        findings.append({
            "file": rel, "line": lineno, "rule_id": rule,
            "severity": "", "classification": "",
            "snippet": snippet.strip()[:120],
        })

    if ABS_PARENT_WORKSPACE.search(line):
        add("R01-parent-workspace", line)
    if ABS_SIBLING.search(line):
        add("R02-sibling-checkout-abs", line)
    if ABS_VOLUMES.search(line):
        add("R03-volume-mount", line)
    if HOME_REF.search(line):
        add("R04-home-ref", line)
    if ext in QUOTED_DOTDOT_EXTENSIONS:
        for match in QUOTED_DOTDOT.finditer(line):
            if escapes_root(root, base_dir, match.group(1)):
                add("R05-dotdot-escape", line)
                break
    for match in SIBLING_REL.finditer(line):
        # Intra-repo vendored trees (vendor/heardright/...) and provenance
        # citations (imports/provenance/..., app/... mirrors) are legitimate.
        name_start = match.start() + (1 if match.group(0).startswith("/") else 0)
        before = line[max(0, name_start - 80):name_start]
        if PROVENANCE_CONTEXT.search(before):
            continue
        add("R07-sibling-checkout-rel", line)
        break
    if USER_SKILL_DIR.search(line):
        add("R08-user-skill-dir", line)
    for name in FORBIDDEN_ENV_NAMES:
        if re.search(r"\b" + name + r"\b", line):
            add("R16-env-checkout", line)
            break
    else:
        if WORKSPACE_ROOT_ENV.search(line):
            add("R16-env-checkout", line)
    if PATH_MENTION.search(line) and re.search(
        r"\b(" + "|".join(SIBLING_TOOLS) + r")\b", line
    ):
        add("R11-path-lookup", line)
    return findings


def invocation_findings(rel: str, lineno: int, line: str, ext: str) -> list[dict]:
    pattern = None
    if ext == ".rs":
        pattern = RUST_INVOKE
    elif ext in {".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"}:
        pattern = JS_INVOKE
    elif ext in {".sh", ".bash"}:
        pattern = SHELL_INVOKE
    if pattern and pattern.search(line):
        return [{
            "file": rel, "line": lineno, "rule_id": "R12-invoke-bare",
            "severity": "", "classification": "",
            "snippet": line.strip()[:120],
        }]
    return []


def scan_content(root: Path, path: Path, rel: str, text: str) -> list[dict]:
    ext = path.suffix.lower()
    classification = classify_file(rel, path.name)
    test_lines: set[int] = set()
    if ext == ".rs" and classification == "release_code":
        test_lines = rust_test_region_lines(text)

    findings: list[dict] = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        for finding in line_pattern_findings(root, path.parent, rel, lineno, line, ext):
            cls = classification
            if cls == "release_code" and lineno in test_lines:
                cls = "test_code"
            finding["classification"] = cls
            finding["severity"] = severity_for(cls)
            findings.append(finding)
        for finding in invocation_findings(rel, lineno, line, ext):
            cls = classification
            if cls == "release_code" and lineno in test_lines:
                cls = "test_code"
            finding["classification"] = cls
            finding["severity"] = severity_for(cls)
            findings.append(finding)

    if path.name == "Cargo.toml":
        base_dir = path.parent
        for lineno, line in enumerate(text.splitlines(), start=1):
            for match in CARGO_PATH_DEP.finditer(line):
                target = match.group(1)
                if target.startswith(".") or os.path.isabs(target):
                    if escapes_root(root, base_dir, target):
                        findings.append({
                            "file": rel, "line": lineno,
                            "rule_id": "R09-cargo-path-escape",
                            "severity": severity_for(classification),
                            "classification": classification,
                            "snippet": line.strip()[:120],
                        })
    if path.name == "pnpm-workspace.yaml":
        for lineno, line in enumerate(text.splitlines(), start=1):
            match = PNPM_ENTRY.match(line)
            if not match:
                continue
            entry = match.group(1).strip()
            if entry.startswith(".") and escapes_root(root, root, entry):
                findings.append({
                    "file": rel, "line": lineno,
                    "rule_id": "R10-workspace-escape",
                    "severity": severity_for(classification),
                    "classification": classification,
                    "snippet": line.strip()[:120],
                })
    return findings


# ---------------------------------------------------------------------------
# Tree traversal and structural checks
# ---------------------------------------------------------------------------

def structural_findings(root: Path) -> list[dict]:
    findings = []
    if (root / ".gitmodules").exists():
        findings.append({
            "file": ".gitmodules", "line": 1, "rule_id": "R13-gitmodules",
            "severity": "FAIL", "classification": "release_code",
            "snippet": "git submodule definitions are forbidden",
        })
    workflows = root / ".github" / "workflows"
    if workflows.is_dir():
        for entry in sorted(workflows.iterdir()):
            if entry.is_file():
                rel = ".github/workflows/" + entry.name
                findings.append({
                    "file": rel, "line": 1, "rule_id": "R14-hosted-ci",
                    "severity": "FAIL", "classification": "release_code",
                    "snippet": "hosted CI workflow files are forbidden",
                })
    for tree in ("skills", "vendor"):
        base = root / tree
        if not base.is_dir():
            continue
        for dirpath, dirnames, filenames in os.walk(base):
            for name in dirnames + filenames:
                full = Path(dirpath) / name
                if full.is_symlink():
                    findings.append({
                        "file": str(full.relative_to(root)).replace(os.sep, "/"),
                        "line": 1, "rule_id": "R15-symlink",
                        "severity": "FAIL", "classification": "release_code",
                        "snippet": "symlinks under " + tree + "/ are forbidden",
                    })
    return findings


def enumerate_files(root: Path) -> list[Path]:
    results = []
    for dirpath, dirnames, filenames in os.walk(root):
        rel_dir = os.path.relpath(dirpath, root)
        rel_dir = "" if rel_dir == "." else rel_dir.replace(os.sep, "/") + "/"
        # Vendored source is scanned; build/cache trees and dot-dirs are not
        # (.github stays visible so hosted-CI files are enumerable).
        dirnames[:] = sorted(
            d for d in dirnames
            if d not in SKIP_DIRS and (not d.startswith(".") or d == ".github")
        )
        for name in sorted(filenames):
            if name.startswith("."):
                continue
            results.append(Path(dirpath) / name)
    return sorted(results)


def scan_tree(root: Path, exclude: set[Path] | None = None) -> list[dict]:
    findings = structural_findings(root)
    excluded = {p.resolve() for p in exclude} if exclude else set()
    for path in enumerate_files(root):
        if excluded and path.resolve() in excluded:
            continue
        rel = str(path.relative_to(root)).replace(os.sep, "/")
        if is_exempt(rel):
            continue
        if path.suffix.lower() not in SCAN_EXTENSIONS:
            continue
        if path.is_symlink():
            continue
        try:
            text = path.read_text(encoding="utf-8")
        except (UnicodeDecodeError, OSError):
            continue
        findings.extend(scan_content(root, path, rel, text))
    findings.sort(key=lambda f: (f["file"], f["line"], f["rule_id"]))
    return findings


def build_report(root: Path, findings: list[dict]) -> dict:
    by_severity = {"FAIL": 0, "INFO": 0}
    by_classification: dict[str, int] = {}
    by_rule: dict[str, int] = {}
    for finding in findings:
        by_severity[finding["severity"]] += 1
        by_classification[finding["classification"]] = by_classification.get(finding["classification"], 0) + 1
        by_rule[finding["rule_id"]] = by_rule.get(finding["rule_id"], 0) + 1
    return {
        "schema_version": 1,
        "gate": "v2-standalone-source-audit",
        "task": "CR-V2-B1-026",
        "root": str(root),
        "findings": findings,
        "summary": {
            "total_findings": len(findings),
            "release_code_findings": by_classification.get("release_code", 0),
            "by_severity": by_severity,
            "by_classification": by_classification,
            "by_rule": by_rule,
            "passed": by_classification.get("release_code", 0) == 0,
        },
        "rules": RULE_INFO,
    }


# ---------------------------------------------------------------------------
# Self-test
# ---------------------------------------------------------------------------

def run_audit_on(case_dir: Path) -> dict:
    findings = scan_tree(case_dir)
    return build_report(case_dir, findings)


def self_test() -> int:
    failures = 0

    def check(name: str, cond: bool, detail: str = "") -> None:
        nonlocal failures
        if cond:
            print(f"self-test ok: {name}")
        else:
            print(f"SELF-TEST FAIL: {name} {detail}", file=sys.stderr)
            failures += 1

    with tempfile.TemporaryDirectory(prefix="v2-standalone-audit-") as tmp:
        tmp_root = Path(tmp)

        # 1. Clean tree passes.
        clean = tmp_root / "clean"
        (clean / "crates" / "x" / "src").mkdir(parents=True)
        (clean / "crates" / "x" / "Cargo.toml").write_text(
            '[package]\nname = "x"\n[dependencies]\ncore = { path = "../core" }\n', encoding="utf-8")
        (clean / "crates" / "core").mkdir(parents=True)
        (clean / "crates" / "x" / "src" / "lib.rs").write_text(
            'pub fn f() -> &\'static str { "vendor/heardright is in-repo" }\n', encoding="utf-8")
        report = run_audit_on(clean)
        check("clean-tree-passes", report["summary"]["passed"] and report["summary"]["total_findings"] == 0,
              detail=str(report["findings"]))

        # 2. Planted sibling path dependency must FAIL.
        sibling = tmp_root / "sibling"
        (sibling / "crates" / "x").mkdir(parents=True)
        (sibling / "crates" / "x" / "Cargo.toml").write_text(
            '[dependencies]\nheardright_core = { path = "../../heardright/heardright_core" }\n',
            encoding="utf-8")
        report = run_audit_on(sibling)
        release = [f for f in report["findings"] if f["classification"] == "release_code"]
        check("planted-sibling-path-fails",
              not report["summary"]["passed"] and release
              and any(f["rule_id"] in {"R09-cargo-path-escape", "R07-sibling-checkout-rel"} for f in release),
              detail=str(report["findings"]))

        # 3. Provenance citation is allowed (INFO only, gate still passes).
        prov = tmp_root / "provenance"
        (prov / "imports" / "provenance" / "heardright").mkdir(parents=True)
        (prov / "imports" / "provenance" / "heardright" / "source.md").write_text(
            "Source: https://github.com/orthic/heardright commit 0123abc "
            "from /Volumes/D/claude/heardright\n", encoding="utf-8")
        (prov / "docs" / "legal").mkdir(parents=True)
        (prov / "docs" / "legal" / "notes.json").write_text(
            '{"note": "Vendored from ../heardright at commit deadbeef."}\n', encoding="utf-8")
        report = run_audit_on(prov)
        check("provenance-citation-allowed",
              report["summary"]["passed"] and report["summary"]["total_findings"] > 0
              and all(f["classification"] == "provenance_citation" and f["severity"] == "INFO"
                      for f in report["findings"]),
              detail=str(report["findings"]))

        # 4. Home-relative reference in release code FAILs.
        home = tmp_root / "homeref"
        home.mkdir(parents=True)
        (home / "tool.sh").write_text("#!/bin/sh\ncp config ~/.codex/config.toml\n", encoding="utf-8")
        report = run_audit_on(home)
        check("home-ref-fails", not report["summary"]["passed"]
              and any(f["rule_id"] == "R04-home-ref" for f in report["findings"]),
              detail=str(report["findings"]))

        # 5. Parent workspace escape chain FAILs.
        esc = tmp_root / "escape"
        (esc / "scripts").mkdir(parents=True)
        (esc / "scripts" / "build.sh").write_text(
            'SRC="../../../../Volumes/D/claude/heardright/engine"\n', encoding="utf-8")
        report = run_audit_on(esc)
        check("parent-workspace-escape-fails", not report["summary"]["passed"]
              and any(f["rule_id"] in {"R01-parent-workspace", "R07-sibling-checkout-rel"}
                      and f["classification"] == "release_code"
                      for f in report["findings"]),
              detail=str(report["findings"]))

        # 6. Checkout env var in release code FAILs.
        envcase = tmp_root / "envvar"
        (envcase / "src").mkdir(parents=True)
        (envcase / "src" / "main.rs").write_text(
            'fn main() { let _ = std::env::var_os("CUTRIGHT_HEARDRIGHT_ENGINE"); }\n',
            encoding="utf-8")
        report = run_audit_on(envcase)
        check("env-checkout-var-fails", not report["summary"]["passed"]
              and any(f["rule_id"] == "R16-env-checkout" and f["classification"] == "release_code"
                      for f in report["findings"]),
              detail=str(report["findings"]))

        # 7. Same env var inside a #[cfg(test)] region is test_code INFO.
        testenv = tmp_root / "testenv"
        (testenv / "src").mkdir(parents=True)
        (testenv / "src" / "lib.rs").write_text(
            '#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {\n'
            '        std::env::set_var("CUTRIGHT_HEARDRIGHT_ENGINE", "x");\n    }\n}\n',
            encoding="utf-8")
        report = run_audit_on(testenv)
        check("test-region-env-var-is-info", report["summary"]["passed"]
              and any(f["rule_id"] == "R16-env-checkout" and f["classification"] == "test_code"
                      for f in report["findings"]),
              detail=str(report["findings"]))

        # 7b. Compound cfg(all(test, ...)) modules are test regions too.
        testenv2 = tmp_root / "testenv2"
        (testenv2 / "src").mkdir(parents=True)
        (testenv2 / "src" / "lib.rs").write_text(
            'pub fn release_code() {}\n'
            '#[cfg(all(test, target_os = "macos"))]\nmod platform_tests {\n'
            '    #[test]\n    fn t() {\n'
            '        std::env::set_var("CUTRIGHT_HEARDRIGHT_ENGINE", "x");\n    }\n}\n',
            encoding="utf-8")
        report = run_audit_on(testenv2)
        check("compound-cfg-test-region-is-info", report["summary"]["passed"]
              and any(f["rule_id"] == "R16-env-checkout" and f["classification"] == "test_code"
                      for f in report["findings"]),
              detail=str(report["findings"]))

        # 8. Intra-repo vendor sibling path is allowed.
        vendor = tmp_root / "vendorcase"
        (vendor / "vendor" / "heardright" / "engine").mkdir(parents=True)
        (vendor / "vendor" / "heardright" / "heardright_core").mkdir(parents=True)
        (vendor / "vendor" / "heardright" / "engine" / "Cargo.toml").write_text(
            '[dependencies]\nheardright_core = { path = "../heardright_core" }\n',
            encoding="utf-8")
        report = run_audit_on(vendor)
        check("intra-repo-vendor-path-allowed",
              report["summary"]["passed"] and report["summary"]["total_findings"] == 0,
              detail=str(report["findings"]))

        # 9. .gitmodules presence FAILs.
        submod = tmp_root / "submod"
        submod.mkdir(parents=True)
        (submod / ".gitmodules").write_text('[submodule "x"]\npath = x\n', encoding="utf-8")
        report = run_audit_on(submod)
        check("gitmodules-fails", not report["summary"]["passed"]
              and any(f["rule_id"] == "R13-gitmodules" for f in report["findings"]),
              detail=str(report["findings"]))

        # 10. Symlink under vendor/ FAILs.
        sym = tmp_root / "symcase"
        (sym / "vendor" / "x").mkdir(parents=True)
        (sym / "vendor" / "x" / "real.txt").write_text("real\n", encoding="utf-8")
        os.symlink("real.txt", sym / "vendor" / "x" / "link.txt")
        report = run_audit_on(sym)
        check("vendor-symlink-fails", not report["summary"]["passed"]
              and any(f["rule_id"] == "R15-symlink" for f in report["findings"]),
              detail=str(report["findings"]))

        # 11. PATH-lookup invocation in release code FAILs.
        inv = tmp_root / "invoke"
        (inv / "src").mkdir(parents=True)
        (inv / "src" / "main.rs").write_text(
            'use std::process::Command;\n'
            'fn main() { let _ = Command::new("heardright-engine").spawn(); }\n',
            encoding="utf-8")
        report = run_audit_on(inv)
        check("invoke-bare-fails", not report["summary"]["passed"]
              and any(f["rule_id"] == "R12-invoke-bare" for f in report["findings"]),
              detail=str(report["findings"]))

        # 12. User skill directory live reference FAILs; .md citation is INFO.
        skill = tmp_root / "skillref"
        (skill / "src").mkdir(parents=True)
        (skill / "src" / "tool.py").write_text(
            'SKILL = "/Users/x/tools/skills/designer/engine"\n', encoding="utf-8")
        report = run_audit_on(skill)
        check("user-skill-dir-fails", not report["summary"]["passed"]
              and any(f["rule_id"] == "R08-user-skill-dir" and f["classification"] == "release_code"
                      for f in report["findings"]),
              detail=str(report["findings"]))

        # 13. Hosted CI workflow FAILs.
        ci = tmp_root / "cicase"
        (ci / ".github" / "workflows").mkdir(parents=True)
        (ci / ".github" / "workflows" / "x.yml").write_text("on: push\n", encoding="utf-8")
        report = run_audit_on(ci)
        check("hosted-ci-fails", not report["summary"]["passed"]
              and any(f["rule_id"] == "R14-hosted-ci" for f in report["findings"]),
              detail=str(report["findings"]))

    if failures:
        print(f"v2-standalone-source-audit self-test: {failures} failure(s)", file=sys.stderr)
        return 2
    print("v2-standalone-source-audit self-test: all cases pass")
    return 0


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main(argv: list[str]) -> int:
    root = default_repo_root()
    json_out: str | None = None
    mode = "audit"
    i = 0
    while i < len(argv):
        if argv[i] == "--self-test":
            mode = "self-test"
        elif argv[i] == "--root" and i + 1 < len(argv):
            root = Path(argv[i + 1]).resolve()
            i += 1
        elif argv[i] in {"--json", "--out"} and i + 1 < len(argv):
            json_out = argv[i + 1]
            i += 1
        else:
            print(__doc__, file=sys.stderr)
            return 2
        i += 1

    if mode == "self-test":
        return self_test()

    if not root.is_dir():
        print(f"[FAIL] standalone source audit: root not found: {root}", file=sys.stderr)
        return 2

    findings = scan_tree(root, exclude={Path(json_out)} if json_out else None)
    report = build_report(root, findings)

    if json_out:
        out_path = Path(json_out)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(
            json.dumps(report, separators=(",", ":")) + "\n", encoding="utf-8"
        )

    summary = report["summary"]
    for finding in findings:
        stream = sys.stderr if finding["severity"] == "FAIL" else sys.stdout
        print(
            f"{finding['severity']}: {finding['file']}:{finding['line']} "
            f"[{finding['rule_id']}] ({finding['classification']}) {finding['snippet']}",
            file=stream,
        )
    print(
        f"findings: total={summary['total_findings']} "
        f"release_code={summary['release_code_findings']} "
        f"by_severity={summary['by_severity']} by_rule={summary['by_rule']}"
    )
    if summary["passed"]:
        print("[PASS] v2 standalone source audit: zero release-code findings")
        return 0
    print(
        f"[FAIL] v2 standalone source audit: {summary['release_code_findings']} release-code finding(s)",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
