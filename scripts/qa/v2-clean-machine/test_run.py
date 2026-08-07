"""Focused checks for exact-RC clean-machine evidence."""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

MODULE = Path(__file__).with_name("run.py")
SPEC = importlib.util.spec_from_file_location("clean_machine_run", MODULE)
run = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(run)


class CleanMachineHarnessTest(unittest.TestCase):
    def _bundle(self, root: Path) -> Path:
        bundle = root / "rc"
        app = bundle / "app" / "cutright"
        app.parent.mkdir(parents=True)
        app.write_text("#!/bin/sh\nprintf '%s\\n' '{\"sample\":\"repurpose-podcast\",\"lane\":\"creator\",\"state\":\"ready_review\",\"network_attempt_total\":0,\"pack_ids\":[\"v2-capability-core\",\"v2-skill-runtime\"],\"lifecycle\":{\"correction_undo\":true,\"restart_resume\":true,\"repair_rollback\":true,\"uninstall_preservation\":true}}'\n")
        # Each sample's lane is adjusted by its tiny protocol executable below.
        os.chmod(app, 0o755)
        packs = []
        for name in ("v2-capability-core", "v2-skill-runtime"):
            path = bundle / "packs" / name / "PACK.json"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("{}")
            packs.append({"name": name, "location": f"release/v2/rc/packs/{name}/"})
        for sample, _lane in run.EXPECTED_SAMPLES:
            (bundle / "samples" / "v2" / sample / "sources").mkdir(parents=True)
            (bundle / "samples" / "v2" / sample / "project.json").write_text("{}")
            (bundle / "samples" / "v2" / sample / "sources" / "manifest.json").write_text("{}")
        (bundle / "RC-MANIFEST.json").write_text(json.dumps({"packs": packs}))
        items = []
        for path in sorted(bundle.rglob("*")):
            if path.is_file():
                items.append({"path": str(path.relative_to(bundle)), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()})
        (bundle / "SEAL.json").write_text(json.dumps({"items": items}))
        return bundle

    def test_missing_app_is_explicit_and_non_passing(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = Path(temporary) / "rc"
            bundle.mkdir()
            ok, evidence = run._verify_exact_hashes(bundle)
            self.assertFalse(ok)
            self.assertIn("error", evidence)
            self.assertIsNone(run._find_app(bundle))

    def test_hash_verification_rejects_empty_or_outside_entries(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = Path(temporary) / "rc"
            bundle.mkdir()
            (bundle / "SEAL.json").write_text(json.dumps({"items": [{}]}))
            self.assertFalse(run._verify_exact_hashes(bundle)[0])
            (bundle / "SEAL.json").write_text(json.dumps({"items": [{"path": "../outside", "sha256": "x"}]}))
            self.assertFalse(run._verify_exact_hashes(bundle)[0])

    def test_hash_verification_rejects_extra_unsealed_file(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = self._bundle(Path(temporary))
            extra = bundle / "unsealed.txt"
            extra.write_text("not covered by SEAL.json")
            ok, evidence = run._verify_exact_hashes(bundle)
            self.assertFalse(ok)
            self.assertEqual(evidence["unsealed_files"], ["unsealed.txt"])

    def test_sample_protocol_rejects_lane_mismatch(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = self._bundle(Path(temporary))
            app = bundle / "app" / "cutright"
            result = run._run_sample(app, [], "repurpose-podcast", "speech", ["v2-capability-core", "v2-skill-runtime"])
            self.assertEqual(result["outcome"], "failed")
            self.assertEqual(result["network_attempt_total"], 0)

    def test_videoctl_uses_lifecycle_acceptance_command(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = self._bundle(Path(temporary))
            videoctl = bundle / "app" / "videoctl"
            videoctl.write_text("#!/bin/sh\n")
            os.chmod(videoctl, 0o755)
            completed = __import__("subprocess").CompletedProcess([], 1, '{}', "")
            with patch.object(run.subprocess, "run", return_value=completed) as invoke:
                result = run._run_sample(videoctl, [], "recorded-talking-head", "creator", [])
            self.assertEqual(invoke.call_args.args[0][1:3], ["clean-machine-sample", str(bundle / "samples" / "v2" / "recorded-talking-head" / "project.json")])
            self.assertIn("--network-deny", invoke.call_args.args[0])
            self.assertEqual(result["outcome"], "failed")

    def test_sample_protocol_requires_every_lifecycle_result(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = self._bundle(Path(temporary))
            payload = {
                "sample": "recorded-talking-head", "lane": "creator",
                "state": "ready_review", "network_attempt_total": 0,
                "pack_ids": ["v2-capability-core", "v2-skill-runtime"],
                "lifecycle": {"correction_undo": True, "restart_resume": False,
                              "repair_rollback": True, "uninstall_preservation": True},
            }
            completed = __import__("subprocess").CompletedProcess([], 0, json.dumps(payload), "")
            with patch.object(run.subprocess, "run", return_value=completed):
                result = run._run_sample(
                    bundle / "app" / "cutright", [], "recorded-talking-head", "creator",
                    ["v2-capability-core", "v2-skill-runtime"],
                )
            self.assertEqual(result["outcome"], "failed")
            self.assertFalse(result["lifecycle"]["restart_resume"])


if __name__ == "__main__":
    unittest.main()
