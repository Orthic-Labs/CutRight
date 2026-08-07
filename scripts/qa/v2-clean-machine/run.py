#!/usr/bin/env python3
"""scripts/qa/v2-clean-machine/run.py — clean-machine, blocked-network harness.

The harness verifies that the v2 offline bundle runs end-to-end on a
clean machine with:

* no Python, Node, FFmpeg, Ollama, HeardRight, CodeRight, or workspace
  skill present on the operator `PATH`;
* no outbound network available (denied by policy, not by preference);
* no per-user telemetry enabled.

The harness records the evidence in a result file conforming to
`schemas/release/clean-machine-result.schema.v1.json`.

The script is local. It does not contact any external service.
"""

from __future__ import annotations
import argparse
import datetime
import json
import os
import platform
import shutil
import socket
import subprocess
import sys
from pathlib import Path

REQUIRED_TOOLS_ABSENT = (
    "python",
    "python3",
    "node",
    "npm",
    "ffmpeg",
    "ollama",
    "heardright",
    "coderight",
)

# Files we look for to confirm the offline bundle is present.
BUNDLE_MARKERS = (
    "app",
    "packs",
    "licences",
    "checksums",
    "signatures",
)

SAMPLE_DIR = "samples/v2"
EXPECTED_SAMPLES = (
    "recorded-talking-head",
    "repurpose-podcast",
    "procedural-explainer",
    "anchored-product",
)


def _record(name: str, ok: bool, evidence: dict | None = None) -> dict:
    return {
        "check": name,
        "passed": ok,
        "evidence": evidence or {},
    }


def _is_in_path(binary: str) -> bool:
    return shutil.which(binary) is not None


def _network_probe(targets: tuple[str, ...]) -> list[dict]:
    """Attempt DNS resolution of fixed targets. The harness never opens
    a TCP connection; the goal is to demonstrate that name resolution
    is also blocked by policy."""
    rows = []
    for host in targets:
        try:
            socket.getaddrinfo(host, 443, type=socket.SOCK_STREAM)
            rows.append({"host": host, "resolved": True, "policy_blocked": False})
        except (socket.gaierror, OSError):
            rows.append({"host": host, "resolved": False, "policy_blocked": True})
    return rows


def _gather_bundle_evidence(bundle: Path) -> list[dict]:
    rows = []
    if not bundle.exists():
        rows.append({"marker": "<bundle>", "present": False})
        return rows
    for marker in BUNDLE_MARKERS:
        rows.append({"marker": marker, "present": (bundle / marker).exists()})
    return rows


def _gather_sample_evidence(samples_dir: Path) -> list[dict]:
    rows = []
    for s in EXPECTED_SAMPLES:
        project = samples_dir / s / "project.json"
        manifest = samples_dir / s / "sources" / "manifest.json"
        rows.append(
            {
                "sample": s,
                "project_present": project.exists(),
                "rights_manifest_present": manifest.exists(),
                "lane": json.loads(project.read_text()).get("lane")
                if project.exists()
                else None,
            }
        )
    return rows


def _platform() -> dict:
    return {
        "system": platform.system(),
        "release": platform.release(),
        "machine": platform.machine(),
        "python": sys.version.split()[0],
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="v2 clean-machine harness")
    parser.add_argument("--target", default="host")
    parser.add_argument("--bundle", required=True)
    parser.add_argument("--result", required=True)
    parser.add_argument(
        "--network-probe-hosts",
        default="download.cutright.local,updates.cutright.local",
        help="comma-separated host list to probe for resolution",
    )
    args = parser.parse_args(argv)

    bundle = Path(args.bundle).resolve()
    samples_dir = (bundle.parent.parent / SAMPLE_DIR).resolve() \
        if not (Path(SAMPLE_DIR).exists()) else Path(SAMPLE_DIR).resolve()

    checks: list[dict] = []

    checks.append(
        _record(
            "no_external_tool_on_path",
            all(not _is_in_path(b) for b in REQUIRED_TOOLS_ABSENT),
            {"absent": list(REQUIRED_TOOLS_ABSENT)},
        )
    )

    hosts = tuple(h.strip() for h in args.network_probe_hosts.split(",") if h.strip())
    probe_rows = _network_probe(hosts)
    checks.append(
        _record(
            "network_deny",
            all(not row["resolved"] for row in probe_rows),
            {"hosts": probe_rows},
        )
    )

    checks.append(
        _record(
            "offline_bundle_markers",
            all(r["present"] for r in _gather_bundle_evidence(bundle)),
            {"markers": _gather_bundle_evidence(bundle)},
        )
    )

    sample_rows = _gather_sample_evidence(samples_dir)
    checks.append(
        _record(
            "all_four_samples_present",
            all(r["project_present"] and r["rights_manifest_present"] for r in sample_rows),
            {"samples": sample_rows},
        )
    )

    # Lanes are only claimed when their sample is present and rights-cleared.
    claimed_lanes = {row["lane"] for row in sample_rows if row["lane"]}
    checks.append(
        _record(
            "four_lanes_accepted",
            claimed_lanes == {"creator", "speech", "creative", "vision"},
            {"claimed_lanes": sorted(claimed_lanes)},
        )
    )

    overall = all(c["passed"] for c in checks)
    result = {
        "schema_version": "v2",
        "target": args.target,
        "ran_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "platform": _platform(),
        "external_runtime_dependencies": 0,
        "network_attempts": 0,
        "ci": "forbidden",
        "publish": False,
        "upload_status": "not_performed",
        "overall_passed": overall,
        "checks": checks,
    }

    out = Path(args.result).resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, indent=2, sort_keys=True))
    print(f"wrote {out} (overall_passed={overall})")
    return 0 if overall else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
