#!/usr/bin/env python3
"""Self-tests for integrate_skill_catalogue.py.

Run as:
    python3 tools/v2-evals/test_integrate_skill_catalogue.py

No third-party deps; stdlib only. The tests build a synthetic catalogue and
registry in a tempdir, run the integration logic via `main(['--no-validate',
'--write'])`, and assert that:
- derived suites reference every skill that has the capability in its
  permissions;
- the resulting registry still loads through the same JSON shape the
  canonical loader accepts.
"""
from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import integrate_skill_catalogue as mod  # noqa: E402


def _write(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload))


def _synthetic_registry() -> dict:
    return {
        "schema": "cutright.capability/v1",
        "schema_version": 1,
        "registry_id": "synthetic",
        "note": "test registry",
        "capabilities": [
            {
                "schema": "cutright.capability/v1",
                "capability_id": "timeline.read",
                "version": 1,
                "kind": "read",
                "owner_component": "video-state",
                "permission_set": "pset.read_only",
                "inputs": {},
                "outputs": {"bounded": True, "windowed": True, "max_items": 100},
                "eval_suites": [],
                "degradation": "ok",
            },
            {
                "schema": "cutright.capability/v1",
                "capability_id": "asset.plan",
                "version": 1,
                "kind": "mutation",
                "owner_component": "video-actions",
                "permission_set": "pset.editor",
                "inputs": {},
                "outputs": {"bounded": False, "windowed": False},
                "eval_suites": [],
                "degradation": "ok",
            },
        ],
        "permission_sets": [
            {
                "schema": "cutright.permission_set/v1",
                "permission_set_id": "pset.read_only",
                "scope_grants": [{"capability_id": "timeline.read", "scope": "timeline_read"}],
            },
            {
                "schema": "cutright.permission_set/v1",
                "permission_set_id": "pset.editor",
                "scope_grants": [{"capability_id": "asset.plan", "scope": "asset_plan"}],
            },
        ],
    }


def _synthetic_catalogue() -> dict:
    return {
        "schema": "cutright.skill_catalogue/v1",
        "catalogue_id": "synthetic",
        "skills": [
            {
                "id": "brand",
                "calls": [],
                "dependencies": [],
                "permissions": ["timeline.read"],
                "eval_suites": ["skill.brand_roundtrip"],
            },
            {
                "id": "qa",
                "calls": [],
                "dependencies": [],
                "permissions": ["timeline.read", "asset.plan"],
                "eval_suites": [],
            },
            {
                "id": "ignored",
                "calls": [],
                "dependencies": [],
                "permissions": ["nonexistent.capability"],
                "eval_suites": [],
            },
        ],
    }


def test_derive_eval_suites_assigns_skills_to_their_capabilities() -> None:
    registry = _synthetic_registry()
    catalogue = _synthetic_catalogue()
    derived = mod.derive_eval_suites(catalogue, registry)
    assert "timeline.read" in derived
    assert "skill.brand" in derived["timeline.read"]
    assert "skill.brand_roundtrip" in derived["timeline.read"]
    assert "skill.qa" in derived["timeline.read"]
    assert "skill.qa" in derived["asset.plan"]
    # The orphan capability permission must NOT introduce a phantom capability
    assert "nonexistent.capability" not in derived
    # Stable ordering
    assert derived["timeline.read"] == sorted(set(derived["timeline.read"]))


def test_merge_eval_suites_preserves_existing_suites() -> None:
    registry = _synthetic_registry()
    registry["capabilities"][0]["eval_suites"] = ["eval.existing"]
    catalogue = _synthetic_catalogue()
    derived = mod.derive_eval_suites(catalogue, registry)
    touched = mod.merge_eval_suites(registry, derived)
    assert touched == 2  # both capabilities changed
    assert "eval.existing" in registry["capabilities"][0]["eval_suites"]
    assert "skill.brand" in registry["capabilities"][0]["eval_suites"]


def test_main_no_validate_writes_registry() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        tmp_path = Path(tmp)
        reg = tmp_path / "registry.json"
        cat = tmp_path / "catalogue.json"
        _write(reg, _synthetic_registry())
        _write(cat, _synthetic_catalogue())

        original_reg = mod.REGISTRY_PATH
        original_cat = mod.CATALOGUE_PATH
        mod.REGISTRY_PATH = reg
        mod.CATALOGUE_PATH = cat
        try:
            rc = mod.main(["--no-validate", "--write"])
            assert rc == 0
            after = json.loads(reg.read_text())
            # timeline.read got suites
            suites = after["capabilities"][0]["eval_suites"]
            assert "skill.brand" in suites
            assert "skill.qa" in suites
        finally:
            mod.REGISTRY_PATH = original_reg
            mod.CATALOGUE_PATH = original_cat


def test_real_catalogue_integration_validates() -> None:
    """Run the integration against the real catalogue + registry. Must pass
    the Rust validator."""
    rc = mod.main([])
    assert rc == 0, "integration against the real catalogue failed"


def main() -> int:
    tests = [
        test_derive_eval_suites_assigns_skills_to_their_capabilities,
        test_merge_eval_suites_preserves_existing_suites,
        test_main_no_validate_writes_registry,
        test_real_catalogue_integration_validates,
    ]
    failed = []
    for fn in tests:
        try:
            fn()
        except AssertionError as exc:
            failed.append((fn.__name__, str(exc)))
            print(f"FAIL  {fn.__name__}: {exc}")
        except subprocess.CalledProcessError as exc:
            failed.append((fn.__name__, str(exc)))
            print(f"FAIL  {fn.__name__}: subprocess error {exc}")
        else:
            print(f"PASS  {fn.__name__}")
    if failed:
        print(f"\n{len(failed)}/{len(tests)} tests failed")
        return 1
    print(f"\n{len(tests)}/{len(tests)} tests passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())