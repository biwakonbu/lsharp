#!/usr/bin/env python3

"""Contract tests for the v0.4 Rust/native fixture diff helper."""

from __future__ import annotations

import copy
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
DIFF = SCRIPTS_DIR / "semantic_fixture_diff.py"
TARGET = "aarch64-apple-darwin"
SOURCE_COMMIT = subprocess.check_output(
    ["git", "-C", str(ROOT), "rev-parse", "--verify", "HEAD"], text=True
).strip()
ARTIFACT_DIGEST = "sha256:" + "b" * 64


def report_for(producer):
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    fixtures = []
    for fixture in manifest["fixtures"]:
        expected = fixture["expected"]
        if fixture["kind"] == "invalid":
            artifact = {"status": "not-applicable"}
            runtime = {
                "status": "not-run",
                "exit_code": None,
                "stdout": None,
                "stderr": None,
                "artifact_sha256": None,
            }
        else:
            artifact = {"status": "pending"}
            runtime = {
                "status": "pending",
                "exit_code": None,
                "stdout": None,
                "stderr": None,
                "artifact_sha256": None,
            }
        fixtures.append(
            {
                "id": fixture["id"],
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
        "fixtures": fixtures,
    }


class SemanticFixtureDiffTest(unittest.TestCase):
    def run_diff(self, oracle, native, fixture_ids=None):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            oracle_path = root / "oracle.json"
            native_path = root / "native.json"
            oracle_path.write_text(json.dumps(oracle), encoding="utf-8")
            native_path.write_text(json.dumps(native), encoding="utf-8")
            command = [
                    sys.executable,
                    str(DIFF),
                    "--manifest",
                    str(MANIFEST),
                    "--root",
                    str(ROOT),
                    "--oracle",
                    str(oracle_path),
                    "--native",
                    str(native_path),
                ]
            for fixture_id in fixture_ids or []:
                command.extend(["--fixture-id", fixture_id])
            return subprocess.run(
                command,
                capture_output=True,
                text=True,
                check=False,
            )

    def test_pending_reports_never_promote_to_pass(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        result = self.run_diff(oracle, native)
        self.assertEqual(result.returncode, 2, result.stderr)
        projected = json.loads(result.stdout)
        self.assertEqual(projected["status"], "pending")
        self.assertTrue(projected["pending_boundaries"])
        self.assertFalse(projected["mismatches"])

        for report in (oracle, native):
            for fixture in report["fixtures"]:
                if fixture["artifact"]["status"] == "pending":
                    fixture["artifact"] = {"status": "observed", "sha256": "sha256:" + "b" * 64, "size": 1}
                if fixture["runtime"]["status"] == "pending":
                    expected = next(
                        item for item in json.loads(MANIFEST.read_text(encoding="utf-8"))["fixtures"]
                        if item["id"] == fixture["id"]
                    )["expected"]["runtime"]
                    fixture["runtime"] = {
                        "status": "observed",
                        "exit_code": expected["exit_code"],
                        "stdout": expected["stdout"],
                        "stderr": expected["stderr"],
                        "artifact_sha256": ARTIFACT_DIGEST,
                    }
        result = self.run_diff(oracle, native)
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        self.assertEqual(projected["status"], "pass")
        self.assertFalse(projected["pending_boundaries"])
        self.assertFalse(projected["mismatches"])

    def test_rejects_observed_empty_artifact(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
        fixture_id = next(item["id"] for item in manifest["fixtures"] if item["kind"] == "valid")
        for report in (oracle, native):
            fixture = next(item for item in report["fixtures"] if item["id"] == fixture_id)
            fixture["artifact"] = {
                "status": "observed",
                "sha256": "sha256:" + "b" * 64,
                "size": 0,
            }

        result = self.run_diff(oracle, native)

        self.assertEqual(result.returncode, 1)
        self.assertIn("positive", result.stderr.lower())

    def test_rejects_runtime_bound_to_different_artifact(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        for report in (oracle, native):
            fixture = next(
                item for item in report["fixtures"] if item["id"] == "valid/syntax-basic"
            )
            fixture["artifact"] = {
                "status": "observed",
                "sha256": ARTIFACT_DIGEST,
                "size": 1,
            }
            fixture["runtime"] = {
                "status": "observed",
                "exit_code": 0,
                "stdout": "42\n",
                "stderr": "",
                "artifact_sha256": "sha256:" + "c" * 64,
            }

        result = self.run_diff(oracle, native, ["valid/syntax-basic"])

        self.assertEqual(result.returncode, 1)
        self.assertIn("artifact_sha256", result.stderr)

    def test_reports_expose_observable_mismatch(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        native["fixtures"][-1]["exit_code"] = 7
        native["fixtures"][-1]["artifact"] = {
            "status": "observed",
            "sha256": ARTIFACT_DIGEST,
            "size": 1,
        }
        native["fixtures"][-1]["runtime"] = {
            "status": "observed",
            "exit_code": 7,
            "stdout": "wrong\n",
            "stderr": "",
            "artifact_sha256": ARTIFACT_DIGEST,
        }
        result = self.run_diff(oracle, native)
        self.assertEqual(result.returncode, 1)
        projected = json.loads(result.stdout)
        self.assertEqual(projected["status"], "mismatch")
        self.assertIn("valid/syntax-basic", {item["fixture"] for item in projected["mismatches"]})

        native["fixtures"][-1]["exit_code"] = 0
        oracle["fixtures"][-1]["artifact"] = {
            "status": "observed",
            "sha256": ARTIFACT_DIGEST,
            "size": 1,
        }
        oracle["fixtures"][-1]["runtime"] = {
            "status": "observed",
            "exit_code": 0,
            "stdout": "42\n",
            "stderr": "",
            "artifact_sha256": ARTIFACT_DIGEST,
        }
        native["fixtures"][-1]["runtime"] = {
            "status": "observed",
            "exit_code": 0,
            "stdout": "wrong\n",
            "stderr": "",
            "artifact_sha256": ARTIFACT_DIGEST,
        }
        result = self.run_diff(oracle, native)
        self.assertEqual(result.returncode, 1)
        projected = json.loads(result.stdout)
        self.assertIn(
            "runtime.stdout",
            {item["field"] for item in projected["mismatches"]},
        )

    def test_rejects_stale_source_or_target_before_comparison(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        cases = (
            ("source_commit", "source_commit", "c" * 40),
            ("target", "target", "x86_64-unknown-linux-gnu"),
        )
        for label, field, value in cases:
            with self.subTest(label=label):
                changed = copy.deepcopy(native)
                changed[field] = value
                result = self.run_diff(oracle, changed)
                self.assertEqual(result.returncode, 1)
                self.assertIn(label, result.stderr.lower())

    def test_rejects_shared_stale_source_commit(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        oracle["source_commit"] = "b" * 40
        native["source_commit"] = "b" * 40

        result = self.run_diff(oracle, native)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("current", result.stderr.lower())

    def test_rejects_swapped_report_producer_roles(self):
        oracle = report_for("native-stage0")
        native = report_for("rust-oracle")

        result = self.run_diff(oracle, native)

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("producer", result.stderr.lower())

    def test_selected_fixture_can_close_one_boundary_without_false_pending(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        for report in (oracle, native):
            report["fixtures"] = [report["fixtures"][-1]]
            fixture = report["fixtures"][-1]
            fixture["artifact"] = {"status": "observed", "sha256": "sha256:" + "b" * 64, "size": 1}
            fixture["runtime"] = {
                "status": "observed",
                "exit_code": 0,
                "stdout": "42\n",
                "stderr": "",
                "artifact_sha256": ARTIFACT_DIGEST,
            }
        result = self.run_diff(oracle, native, ["valid/syntax-basic"])
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        self.assertEqual(projected["fixture_count"], 1)
        self.assertEqual(projected["status"], "pass")


if __name__ == "__main__":
    raise SystemExit(unittest.main())
