"""Focused local tests for v2 RC payload assembly."""

from __future__ import annotations

import importlib.util
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

MODULE = Path(__file__).with_name("v2-build.py")
SPEC = importlib.util.spec_from_file_location("v2_build", MODULE)
build = importlib.util.module_from_spec(SPEC)
assert SPEC.loader
SPEC.loader.exec_module(build)


class V2BuildTest(unittest.TestCase):
    def _root(self, temporary: Path, profile: str = "release") -> Path:
        root = temporary / "source"
        executable = root / "target" / profile / "videoctl"
        executable.parent.mkdir(parents=True)
        executable.write_text("#!/bin/sh\n")
        executable.chmod(0o755)
        for name in build.SAMPLE_NAMES:
            sample = root / "samples" / "v2" / name
            sample.mkdir(parents=True)
            (sample / "project.json").write_text("{}")
        for name in build.PACK_NAMES:
            pack = root / "release" / "v2" / "rc" / "packs" / name
            pack.mkdir(parents=True)
            (pack / "PACK.json").write_text("{}")
        source = root / "release" / "v2" / "staging" / "corresponding-source"
        source.mkdir(parents=True)
        (source / "README.md").write_text("source")
        manifest = root / "release" / "v2" / "RC-MANIFEST.json"
        manifest.parent.mkdir(parents=True, exist_ok=True)
        manifest.write_text(json.dumps({"samples": [], "verified": True}))
        return root

    def test_assembles_executable_and_all_samples(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, out = self._root(Path(temporary)), Path(temporary) / "rc"
            environment = {"PATH": "/safe", "PUBLIC_FLAG": "retain", "API_TOKEN": "do-not-expose"}
            filtered_environment = {"PATH": "/safe", "PUBLIC_FLAG": "retain"}
            with patch.object(build.subprocess, "run") as run:
                meta = build.assemble(root, out, "host", "/fake/cargo", environment)
            run.assert_called_once_with(
                ["/fake/cargo", "build", "--profile", "release", "-p", "videoctl"],
                cwd=root, check=True, env=filtered_environment,
            )
            self.assertTrue((out / "app" / "videoctl").stat().st_mode & 0o111)
            self.assertEqual(set(build.SAMPLE_NAMES), {path.name for path in (out / "samples" / "v2").iterdir()})
            self.assertEqual(set(build.PACK_NAMES), {path.name for path in (out / "packs").iterdir()})
            self.assertTrue((out / "corresponding-source" / "README.md").is_file())
            self.assertEqual("target/release/videoctl", meta["executable"])

    def test_honors_nonrelease_profile_in_cargo_and_artifact_path(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, out = self._root(Path(temporary), "ci"), Path(temporary) / "rc"
            with patch.object(build.subprocess, "run") as run:
                meta = build.assemble(root, out, "host", "/fake/cargo", {"PATH": "/safe"}, "ci")
            run.assert_called_once_with(
                ["/fake/cargo", "build", "--profile", "ci", "-p", "videoctl"],
                cwd=root, check=True, env={"PATH": "/safe"},
            )
            self.assertEqual("target/ci/videoctl", meta["executable"])

    def test_replaces_only_builder_owned_staged_paths(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, out = self._root(Path(temporary)), Path(temporary) / "rc"
            (out / "app").mkdir(parents=True)
            (out / "app" / "stale").write_text("stale")
            (out / "BUILD.json").write_text("stale")
            (out / "SEAL.json").write_text("stale-seal")
            (out / "checksums.txt").write_text("stale-checksums")
            with patch.object(build.subprocess, "run"):
                build.assemble(root, out, "host", "/fake/cargo", {"PATH": "/safe"})
            self.assertFalse((out / "app" / "stale").exists())
            self.assertFalse((out / "BUILD.json").exists())
            self.assertFalse((out / "SEAL.json").exists())
            self.assertFalse((out / "checksums.txt").exists())

    def test_rebuild_snapshots_in_place_pack_inputs(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = self._root(Path(temporary))
            out = root / "release" / "v2" / "rc"
            with patch.object(build.subprocess, "run"):
                build.assemble(root, out, "host", "/fake/cargo", {"PATH": "/safe"})
            self.assertEqual(set(build.PACK_NAMES), {path.name for path in (out / "packs").iterdir()})

    def test_cargo_failure_stops_before_payload_copy(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, out = self._root(Path(temporary)), Path(temporary) / "rc"
            with patch.object(build.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "cargo")):
                with self.assertRaises(subprocess.CalledProcessError):
                    build.assemble(root, out, "host", "/fake/cargo", {"PATH": "/safe"})
            self.assertFalse((out / "app").exists())

    def test_filters_credential_shaped_names_from_cargo_environment(self):
        source = {
            "PATH": "/safe",
            "PUBLIC_FLAG": "retain",
            "API_TOKEN": "do-not-expose",
            "SERVICE_SECRET": "do-not-expose",
            "SIGNING_KEY": "do-not-expose",
        }
        self.assertEqual(
            {"PATH": "/safe", "PUBLIC_FLAG": "retain"},
            build._filtered_build_env(source),
        )

    def test_build_metadata_records_dirty_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root, out = self._root(Path(temporary)), Path(temporary) / "rc"
            assembly = {"executable": "target/release/videoctl"}
            with patch.object(build, "assemble", return_value=assembly), \
                 patch.object(build, "_git_head", return_value="abc123"), \
                 patch.object(build, "_git_dirty", return_value=True):
                self.assertEqual(0, build.main(["--source", str(root), "--out", str(out)]))
            metadata = json.loads((out / "BUILD.json").read_text())
            self.assertTrue(metadata["source_dirty"])
            self.assertEqual("abc123", metadata["head"])
            manifest = json.loads((out / "RC-MANIFEST.json").read_text())
            self.assertEqual("abc123", manifest["head_full"])
            self.assertFalse(manifest["verified"])
            self.assertEqual(4, len(manifest["samples"]))
            self.assertIn("RC-MANIFEST.json", metadata["staged_payload_hashes"])
            self.assertNotIn("BUILD.json", metadata["staged_payload_hashes"])
            self.assertEqual("BUILD.json", metadata["staged_payload_coverage"]["unhashed_self"])


if __name__ == "__main__":
    unittest.main()
