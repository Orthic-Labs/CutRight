import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("v2-audit.py")
SPEC = importlib.util.spec_from_file_location("v2_audit", MODULE_PATH)
AUDIT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(AUDIT)

SEAL_MODULE_PATH = Path(__file__).with_name("v2-seal.py")
SEAL_SPEC = importlib.util.spec_from_file_location("v2_seal", SEAL_MODULE_PATH)
SEAL = importlib.util.module_from_spec(SEAL_SPEC)
assert SEAL_SPEC.loader
SEAL_SPEC.loader.exec_module(SEAL)


class V2AuditTest(unittest.TestCase):
    def fixture(self) -> tuple[Path, Path]:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        bundle = root / "release/v2/rc"
        bundle.mkdir(parents=True)
        payload = bundle / "app"
        payload.write_text("verified", encoding="utf-8")
        (bundle / "SEAL.json").write_text(json.dumps({"items": [{"path": "app", "sha256": AUDIT._sha256(payload)}]}), encoding="utf-8")
        audit_dir = root / "release/v2/audit"
        audit_dir.mkdir(parents=True)
        names = AUDIT.REQUIRED_AUDIT_EVIDENCE
        audit = {
            "policy": {"pass_only_when_all_required_checks_pass": True, "skipped_is_never_coerced_to_pass": True, "policy_required_checks": ["secret_scan"]},
            "checks": [{"id": "secret_scan", "status": "pass"}], "skipped": [], "unproven": [],
            "summary": {"release_blocking_finding": False, "audit_status": "pass"},
        }
        for name in names:
            (audit_dir / name).write_text(json.dumps(audit if name == "audit.json" else {"status": "pass"}), encoding="utf-8")
        for name in AUDIT.REQUIRED_RELEASE_ARTIFACTS:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("present", encoding="utf-8")
        return root, bundle

    def test_passes_only_complete_passing_evidence(self):
        root, bundle = self.fixture()
        summary = AUDIT.audit(root, bundle, root / "release/v2/audit")
        self.assertEqual(summary["status"], "pass")

    def test_skipped_audit_check_fails(self):
        root, bundle = self.fixture()
        path = root / "release/v2/audit/audit.json"
        audit = json.loads(path.read_text(encoding="utf-8"))
        audit["skipped"] = ["secret_scan"]
        path.write_text(json.dumps(audit), encoding="utf-8")
        summary = AUDIT.audit(root, bundle, root / "release/v2/audit")
        self.assertEqual(summary["status"], "fail")
        self.assertEqual(summary["checks"][3]["status"], "fail")

    def test_rejects_empty_or_outside_seal_entries(self):
        root, bundle = self.fixture()
        seal = bundle / "SEAL.json"
        seal.write_text(json.dumps({"items": [{"path": "../outside", "sha256": "x"}]}), encoding="utf-8")
        self.assertFalse(AUDIT._verify_seal(bundle))
        seal.write_text(json.dumps({"items": []}), encoding="utf-8")
        self.assertFalse(AUDIT._verify_seal(bundle))

    def test_rejects_unlisted_bundle_file(self):
        root, bundle = self.fixture()
        extra = bundle / "extra"
        extra.write_text("not sealed", encoding="utf-8")
        self.assertFalse(AUDIT._verify_seal(bundle))

    def test_legacy_seal_commands_match_documented_plan_forms(self):
        with tempfile.TemporaryDirectory() as temporary:
            bundle = Path(temporary) / "rc"
            bundle.mkdir()
            (bundle / "app").write_text("payload", encoding="utf-8")
            bundled_manifest = bundle / "RC-MANIFEST.json"
            bundled_manifest.write_text('{"head":"abc123"}', encoding="utf-8")
            exported_manifest = Path(temporary) / "release" / "RC-MANIFEST.json"
            self.assertEqual(0, SEAL.main(["--seal", str(bundle), "--manifest", str(exported_manifest)]))
            self.assertEqual(bundled_manifest.read_text(), exported_manifest.read_text())
            self.assertTrue((bundle / "SEAL.json").is_file())
            self.assertEqual(0, SEAL.main(["--verify", str(bundle)]))
            checksums = Path(temporary) / "SHA256SUMS.txt"
            self.assertEqual(0, SEAL.main(["--checksums", str(bundle), "--out", str(checksums)]))
            self.assertTrue(checksums.is_file())

    def test_strict_verify_rejects_malformed_or_unsafe_entries(self):
        root, bundle = self.fixture()
        digest = AUDIT._sha256(bundle / "app")
        invalid_items = [
            [], [None],
            [{"path": "app", "sha256": digest}] * 2,
            [{"path": "/tmp/outside", "sha256": digest}],
            [{"path": "../outside", "sha256": digest}],
            [{"path": "app/child", "sha256": digest}],
            [{"path": "app", "sha256": "invalid"}],
        ]
        for items in invalid_items:
            (bundle / "SEAL.json").write_text(json.dumps({"items": items}), encoding="utf-8")
            self.assertEqual(1, SEAL.main(["--verify", str(bundle)]))

    def test_verify_requires_complete_bundle_coverage(self):
        root, bundle = self.fixture()
        omitted = bundle / "omitted"
        omitted.write_text("not sealed", encoding="utf-8")
        app = bundle / "app"
        seal = {"items": [{"path": "app", "sha256": AUDIT._sha256(app)}]}
        (bundle / "SEAL.json").write_text(json.dumps(seal), encoding="utf-8")
        self.assertEqual(1, SEAL.main(["--verify", str(bundle)]))

        provenance = root / "provenance.json"
        provenance.write_text(json.dumps({"seal_target": str(bundle)}), encoding="utf-8")
        self.assertEqual(1, SEAL.main(["verify-provenance", "--provenance", str(provenance), str(bundle)]))
        provenance.write_text("{malformed", encoding="utf-8")
        self.assertEqual(1, SEAL.main(["verify-provenance", "--provenance", str(provenance), str(bundle)]))

    def test_verify_marks_manifest_and_reverification_passes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "release" / "v2"
            bundle = root / "rc"
            bundle.mkdir(parents=True)
            (bundle / "app").write_text("payload", encoding="utf-8")
            manifest = {"schema_version": "v2", "verified": False}
            (bundle / "RC-MANIFEST.json").write_text(json.dumps(manifest), encoding="utf-8")
            exported = root / "RC-MANIFEST.json"
            self.assertEqual(0, SEAL.main(["--seal", str(bundle), "--manifest", str(exported)]))
            self.assertEqual(0, SEAL.main(["verify", str(bundle)]))
            self.assertTrue(json.loads((bundle / "RC-MANIFEST.json").read_text())["verified"])
            self.assertTrue(json.loads(exported.read_text())["verified"])
            self.assertEqual(0, SEAL.main(["verify", str(bundle)]))

    def test_failed_verify_keeps_manifest_unverified(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / "release" / "v2"
            bundle = root / "rc"
            bundle.mkdir(parents=True)
            payload = bundle / "app"
            payload.write_text("payload", encoding="utf-8")
            (bundle / "RC-MANIFEST.json").write_text(json.dumps({"verified": False}), encoding="utf-8")
            self.assertEqual(0, SEAL.main(["--seal", str(bundle), "--manifest", str(root / "RC-MANIFEST.json")]))
            payload.write_text("tampered", encoding="utf-8")
            self.assertEqual(1, SEAL.main(["verify", str(bundle)]))
            self.assertFalse(json.loads((bundle / "RC-MANIFEST.json").read_text())["verified"])
if __name__ == "__main__":
    unittest.main()
