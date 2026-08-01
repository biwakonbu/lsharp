#!/usr/bin/env python3

"""Contract tests for the V4 evidence-index audit boundary."""

from __future__ import annotations

import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
AUDIT = SCRIPTS_DIR / "semantic_fixture_evidence_audit.py"
TARGET = "aarch64-apple-darwin"
SOURCE_COMMIT = "a" * 40
SELECTED_IDS = ["invalid/type-undefined-value", "valid/syntax-basic"]
GATES = {
    "fallback-forbidden": "pass",
    "network-forbidden": "pass",
    "source-commit-bound": "pass",
    "target-declared": "pass",
}


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
    def run_audit(self, root: pathlib.Path, index: dict) -> subprocess.CompletedProcess[str]:
        index_path = root / "index.json"
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
        with tempfile.TemporaryDirectory(dir=ROOT, prefix=".semantic-evidence-") as directory:
            root = pathlib.Path(directory)
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
        with tempfile.TemporaryDirectory(dir=ROOT, prefix=".semantic-evidence-") as directory:
            root = pathlib.Path(directory)
            self.write_bundle(root, observed=False, comparison_status="pending")
            result = self.run_audit(root, index_for(root, status="pending"))
            self.assertEqual(result.returncode, 2, result.stderr)
            evidence = json.loads(result.stdout)
            self.assertEqual(evidence["status"], "pending")
            self.assertIn("valid/syntax-basic.artifact", evidence["pending_boundaries"])

    def test_rejects_verified_claim_for_pending_comparison(self):
        with tempfile.TemporaryDirectory(dir=ROOT, prefix=".semantic-evidence-") as directory:
            root = pathlib.Path(directory)
            self.write_bundle(root, observed=False, comparison_status="pending")
            result = self.run_audit(root, index_for(root, status="pass"))
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("status", result.stderr.lower())

    def test_rejects_scope_mismatch_missing_gate_and_unsafe_path(self):
        with tempfile.TemporaryDirectory(dir=ROOT, prefix=".semantic-evidence-") as directory:
            root = pathlib.Path(directory)
            self.write_bundle(root)
            cases = []
            missing_fixture = index_for(root)
            missing_fixture["fixtures"] = missing_fixture["fixtures"][:1]
            cases.append(("fixture", missing_fixture, "fixture"))
            missing_gate = index_for(root)
            del missing_gate["fixtures"][0]["negative_gates"]["network-forbidden"]
            cases.append(("gate", missing_gate, "network-forbidden"))
            unsafe_path = index_for(root)
            unsafe_path["oracle_report"] = "../outside.json"
            cases.append(("path", unsafe_path, "relative"))
            for label, index, expected_error in cases:
                with self.subTest(label=label):
                    result = self.run_audit(root, index)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(expected_error, result.stderr.lower())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
