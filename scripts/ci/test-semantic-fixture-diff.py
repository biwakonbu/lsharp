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
SOURCE_COMMIT = "a" * 40


def report_for(producer):
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    fixtures = []
    for fixture in manifest["fixtures"]:
        expected = fixture["expected"]
        if fixture["kind"] == "invalid":
            artifact = {"status": "not-applicable"}
            runtime = {"status": "not-run", "exit_code": None, "stdout": None, "stderr": None}
        else:
            artifact = {"status": "pending"}
            runtime = {"status": "pending", "exit_code": None, "stdout": None, "stderr": None}
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
    def run_diff(self, oracle, native):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            oracle_path = root / "oracle.json"
            native_path = root / "native.json"
            oracle_path.write_text(json.dumps(oracle), encoding="utf-8")
            native_path.write_text(json.dumps(native), encoding="utf-8")
            return subprocess.run(
                [
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
                ],
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
                    }
        result = self.run_diff(oracle, native)
        self.assertEqual(result.returncode, 0, result.stderr)
        projected = json.loads(result.stdout)
        self.assertEqual(projected["status"], "pass")
        self.assertFalse(projected["pending_boundaries"])
        self.assertFalse(projected["mismatches"])

    def test_reports_expose_observable_mismatch(self):
        oracle = report_for("rust-oracle")
        native = report_for("native-stage0")
        native["fixtures"][-1]["exit_code"] = 7
        native["fixtures"][-1]["runtime"] = {
            "status": "observed",
            "exit_code": 7,
            "stdout": "wrong\n",
            "stderr": "",
        }
        result = self.run_diff(oracle, native)
        self.assertEqual(result.returncode, 1)
        projected = json.loads(result.stdout)
        self.assertEqual(projected["status"], "mismatch")
        self.assertIn("valid/syntax-basic", {item["fixture"] for item in projected["mismatches"]})

        native["fixtures"][-1]["exit_code"] = 0
        oracle["fixtures"][-1]["runtime"] = {
            "status": "observed",
            "exit_code": 0,
            "stdout": "42\n",
            "stderr": "",
        }
        native["fixtures"][-1]["runtime"] = {
            "status": "observed",
            "exit_code": 0,
            "stdout": "wrong\n",
            "stderr": "",
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


if __name__ == "__main__":
    raise SystemExit(unittest.main())
