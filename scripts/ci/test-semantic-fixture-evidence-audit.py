#!/usr/bin/env python3

"""Contract tests for the V4 evidence-index audit boundary."""

from __future__ import annotations

from contextlib import contextmanager
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
ARTIFACT_ROOT = ROOT / "ci-artifacts"
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
AUDIT = SCRIPTS_DIR / "semantic_fixture_evidence_audit.py"
TARGET = "aarch64-apple-darwin"
SOURCE_COMMIT = subprocess.check_output(
    ["git", "-C", str(ROOT), "rev-parse", "--verify", "HEAD"], text=True
).strip()
SELECTED_IDS = ["invalid/type-undefined-value", "valid/syntax-basic"]
GATES = {
    "fallback-forbidden": "pass",
    "network-forbidden": "pass",
    "source-commit-bound": "pass",
    "target-declared": "pass",
}


@contextmanager
def temporary_bundle(target: str = TARGET, source_commit: str = SOURCE_COMMIT):
    namespace = ARTIFACT_ROOT / "v4-m1-01" / source_commit / target
    namespace.mkdir(parents=True, exist_ok=True)
    directory = pathlib.Path(tempfile.mkdtemp(dir=namespace, prefix=".semantic-evidence-"))
    try:
        yield directory
    finally:
        shutil.rmtree(directory)
        for parent in (namespace, namespace.parent, namespace.parent.parent):
            try:
                parent.rmdir()
            except OSError:
                break


def fixture_map() -> dict[str, dict]:
    return {item["id"]: item for item in json.loads(MANIFEST.read_text(encoding="utf-8"))["fixtures"]}


def report_for(producer: str, observed: bool = True) -> dict:
    fixtures = fixture_map()
    result = []
    for identifier in SELECTED_IDS:
        expected = fixtures[identifier]["expected"]
        if fixtures[identifier]["kind"] == "invalid":
            artifact = {"status": "not-applicable"}
            runtime = {"status": "not-run", "exit_code": None, "stdout": None, "stderr": None}
        elif observed:
            artifact = {"status": "observed", "sha256": "sha256:" + "b" * 64, "size": 4}
            runtime = {
                "status": "observed",
                "exit_code": expected["runtime"]["exit_code"],
                "stdout": expected["runtime"]["stdout"],
                "stderr": expected["runtime"]["stderr"],
            }
        else:
            artifact = {"status": "pending"}
            runtime = {"status": "pending", "exit_code": None, "stdout": None, "stderr": None}
        result.append(
            {
                "id": identifier,
                "diagnostics": expected["diagnostics"],
                "exit_code": expected["exit_code"],
                "artifact": artifact,
                "runtime": runtime,
            }
        )
    return {
        "schema_version": 1,
        "suite": "v4-m1-01",
        "producer": producer,
        "target": TARGET,
        "source_commit": SOURCE_COMMIT,
        "fixtures": result,
    }


def comparison_for(status: str = "pass") -> dict:
    pending = [] if status == "pass" else ["valid/syntax-basic.artifact", "valid/syntax-basic.runtime"]
    return {
        "schema_version": 1,
        "suite": "v4-m1-01",
        "target": TARGET,
        "source_commit": SOURCE_COMMIT,
        "fixture_count": len(SELECTED_IDS),
        "status": status,
        "pending_boundaries": pending,
        "mismatches": [],
    }


def index_for(root: pathlib.Path, status: str = "pass") -> dict:
    relative_root = root.relative_to(ROOT)
    return {
        "schema_version": 1,
        "suite": "v4-m1-06",
        "task": "V4-M1-01",
        "target": TARGET,
        "source_commit": SOURCE_COMMIT,
        "status": status,
        "adr": "docs/adr/decisions-v0.4-m1-01-semantic-fixture-matrix.md",
        "oracle_report": str(relative_root / "oracle.json"),
        "native_report": str(relative_root / "native.json"),
        "comparison": str(relative_root / "comparison.json"),
        "fixtures": [
            {"id": "invalid/type-undefined-value", "command": "check", "negative_gates": dict(GATES)},
            {"id": "valid/syntax-basic", "command": "compile", "negative_gates": dict(GATES)},
        ],
    }


class SemanticFixtureEvidenceAuditTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._created_artifact_root = not ARTIFACT_ROOT.exists()
        ARTIFACT_ROOT.mkdir(exist_ok=True)

    @classmethod
    def tearDownClass(cls):
        if cls._created_artifact_root:
            ARTIFACT_ROOT.rmdir()

    def run_audit(
        self,
        root: pathlib.Path,
        index: dict,
        index_path: pathlib.Path = None,
    ) -> subprocess.CompletedProcess[str]:
        index_path = index_path or root / "index.json"
        index_path.write_text(json.dumps(index), encoding="utf-8")
        return subprocess.run(
            [
                sys.executable,
                str(AUDIT),
                "--manifest",
                str(MANIFEST),
                "--root",
                str(ROOT),
                "--index",
                str(index_path),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )

    def write_bundle(self, root: pathlib.Path, observed: bool = True, comparison_status: str = "pass") -> None:
        (root / "oracle.json").write_text(json.dumps(report_for("rust-oracle", observed)), encoding="utf-8")
        (root / "native.json").write_text(json.dumps(report_for("native-stage0", observed)), encoding="utf-8")
        (root / "comparison.json").write_text(json.dumps(comparison_for(comparison_status)), encoding="utf-8")

    def test_emits_complete_evidence_index_for_passing_bundle(self):
        with temporary_bundle() as root:
            self.write_bundle(root)
            result = self.run_audit(root, index_for(root))
            self.assertEqual(result.returncode, 0, result.stderr)
            evidence = json.loads(result.stdout)
            self.assertEqual(evidence["status"], "pass")
            self.assertEqual(evidence["fixture_count"], 2)
            self.assertEqual(
                evidence["fixtures"][1]["artifact"],
                {
                    "oracle": {"status": "observed", "sha256": "sha256:" + "b" * 64, "size": 4},
                    "native": {"status": "observed", "sha256": "sha256:" + "b" * 64, "size": 4},
                },
            )

    def test_pending_bundle_returns_pending_without_claiming_verified(self):
        with temporary_bundle() as root:
            self.write_bundle(root, observed=False, comparison_status="pending")
            result = self.run_audit(root, index_for(root, status="pending"))
            self.assertEqual(result.returncode, 2, result.stderr)
            evidence = json.loads(result.stdout)
            self.assertEqual(evidence["status"], "pending")
            self.assertIn("valid/syntax-basic.artifact", evidence["pending_boundaries"])

    def test_rejects_verified_claim_for_pending_comparison(self):
        with temporary_bundle() as root:
            self.write_bundle(root, observed=False, comparison_status="pending")
            result = self.run_audit(root, index_for(root, status="pass"))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("status", result.stderr.lower())

    def test_rejects_shared_stale_source_commit(self):
        with temporary_bundle() as root:
            stale = "b" * 40
            oracle = report_for("rust-oracle")
            native = report_for("native-stage0")
            oracle["source_commit"] = stale
            native["source_commit"] = stale
            comparison = comparison_for()
            comparison["source_commit"] = stale
            (root / "oracle.json").write_text(json.dumps(oracle), encoding="utf-8")
            (root / "native.json").write_text(json.dumps(native), encoding="utf-8")
            (root / "comparison.json").write_text(json.dumps(comparison), encoding="utf-8")
            index = index_for(root)
            index["source_commit"] = stale

            result = self.run_audit(root, index)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("current", result.stderr.lower())

    def test_rejects_swapped_report_producer_roles(self):
        with temporary_bundle() as root:
            (root / "oracle.json").write_text(
                json.dumps(report_for("native-stage0")), encoding="utf-8"
            )
            (root / "native.json").write_text(
                json.dumps(report_for("rust-oracle")), encoding="utf-8"
            )
            (root / "comparison.json").write_text(
                json.dumps(comparison_for()), encoding="utf-8"
            )

            result = self.run_audit(root, index_for(root))

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("producer", result.stderr.lower())

    def test_rejects_scope_mismatch_missing_gate_and_unsafe_path(self):
        with temporary_bundle() as root:
            self.write_bundle(root)
            cases = []
            missing_fixture = index_for(root)
            missing_fixture["fixtures"] = missing_fixture["fixtures"][:1]
            cases.append(("fixture", missing_fixture, "fixture"))
            missing_gate = index_for(root)
            del missing_gate["fixtures"][0]["negative_gates"]["network-forbidden"]
            cases.append(("gate", missing_gate, "network-forbidden"))
            mismatched_task = index_for(root)
            mismatched_task["task"] = "V4-M1-03"
            cases.append(("task", mismatched_task, "task"))
            check_for_artifact = index_for(root)
            check_for_artifact["fixtures"][1]["command"] = "check"
            cases.append(("command", check_for_artifact, "artifact command"))
            adr_outside_scope = index_for(root)
            adr_outside_scope["adr"] = "docs/README.md"
            cases.append(("adr", adr_outside_scope, "docs/adr"))
            unsafe_path = index_for(root)
            unsafe_path["oracle_report"] = "../outside.json"
            cases.append(("path", unsafe_path, "relative"))
            for label, index, expected_error in cases:
                with self.subTest(label=label):
                    result = self.run_audit(root, index)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(expected_error, result.stderr.lower())

    def test_rejects_report_path_through_symlink(self):
        with temporary_bundle() as root, tempfile.TemporaryDirectory() as outside_directory:
            outside = pathlib.Path(outside_directory)
            self.write_bundle(root)
            (outside / "oracle.json").write_text(
                (root / "oracle.json").read_text(encoding="utf-8"), encoding="utf-8"
            )
            (root / "linked-reports").symlink_to(outside, target_is_directory=True)
            index = index_for(root)
            index["oracle_report"] = str(root.relative_to(ROOT) / "linked-reports" / "oracle.json")

            result = self.run_audit(root, index)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symlink", result.stderr.lower())

    def test_rejects_report_bundle_outside_ci_artifacts_namespace(self):
        with tempfile.TemporaryDirectory(dir=ROOT, prefix=".semantic-evidence-") as directory:
            root = pathlib.Path(directory)
            self.write_bundle(root)

            result = self.run_audit(root, index_for(root))

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("ci-artifacts", result.stderr.lower())

    def test_rejects_report_bundle_for_different_target_namespace(self):
        with temporary_bundle("x86_64-unknown-linux-gnu") as root:
            self.write_bundle(root)

            result = self.run_audit(root, index_for(root))

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("target", result.stderr.lower())

    def test_rejects_report_bundle_for_different_source_namespace(self):
        with temporary_bundle(source_commit="b" * 40) as root:
            self.write_bundle(root)

            result = self.run_audit(root, index_for(root))

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("source_commit", result.stderr.lower())

    def test_rejects_index_outside_target_namespace(self):
        with temporary_bundle() as root, tempfile.TemporaryDirectory(
            dir=ROOT, prefix=".semantic-index-"
        ) as outside_directory:
            self.write_bundle(root)
            outside_index = pathlib.Path(outside_directory) / "index.json"

            result = self.run_audit(root, index_for(root), outside_index)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("evidence index", result.stderr.lower())

    def test_rejects_index_symlink(self):
        with temporary_bundle() as root, tempfile.TemporaryDirectory() as outside_directory:
            self.write_bundle(root)
            outside_index = pathlib.Path(outside_directory) / "index.json"
            linked_index = root / "index.json"
            linked_index.symlink_to(outside_index)

            result = self.run_audit(root, index_for(root), linked_index)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("symlink", result.stderr.lower())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
