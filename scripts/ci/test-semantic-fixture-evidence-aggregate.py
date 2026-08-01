#!/usr/bin/env python3

"""Contract tests for the two-target V4 evidence aggregate audit."""

from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys
import unittest
from contextlib import contextmanager


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
ARTIFACT_ROOT = ROOT / "ci-artifacts"
SOURCE_COMMIT = subprocess.check_output(
    ["git", "-C", str(ROOT), "rev-parse", "--verify", "HEAD"], text=True
).strip()
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
AGGREGATE = SCRIPTS_DIR / "semantic_fixture_evidence_aggregate.py"
TARGETS = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]
SELECTED_IDS = ["invalid/type-undefined-value", "valid/syntax-basic"]
GATES = {
    "fallback-forbidden": "pass",
    "network-forbidden": "pass",
    "source-commit-bound": "pass",
    "target-declared": "pass",
}


def fixture_map() -> dict[str, dict]:
    return {item["id"]: item for item in json.loads(MANIFEST.read_text(encoding="utf-8"))["fixtures"]}


def report_for(producer: str, target: str, observed: bool) -> dict:
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
        "target": target,
        "source_commit": SOURCE_COMMIT,
        "fixtures": result,
    }


def comparison_for(target: str, status: str) -> dict:
    pending = [] if status == "pass" else ["valid/syntax-basic.artifact", "valid/syntax-basic.runtime"]
    return {
        "schema_version": 1,
        "suite": "v4-m1-01",
        "target": target,
        "source_commit": SOURCE_COMMIT,
        "fixture_count": len(SELECTED_IDS),
        "status": status,
        "pending_boundaries": pending,
        "mismatches": [],
    }


def target_index(target: str, observed: bool) -> dict:
    prefix = f"ci-artifacts/v4-m1-01/{SOURCE_COMMIT}/{target}"
    status = "pass" if observed else "pending"
    return {
        "schema_version": 1,
        "suite": "v4-m1-06",
        "task": "V4-M1-01",
        "target": target,
        "source_commit": SOURCE_COMMIT,
        "status": status,
        "adr": "docs/adr/decisions-v0.4-m1-01-semantic-fixture-matrix.md",
        "oracle_report": f"{prefix}/oracle.json",
        "native_report": f"{prefix}/native.json",
        "comparison": f"{prefix}/comparison.json",
        "fixtures": [
            {"id": "invalid/type-undefined-value", "command": "check", "negative_gates": dict(GATES)},
            {"id": "valid/syntax-basic", "command": "compile", "negative_gates": dict(GATES)},
        ],
    }


@contextmanager
def scenario(observed: tuple[bool, bool] = (True, True), status: str = "pass"):
    source_root = ARTIFACT_ROOT / "v4-m1-01" / SOURCE_COMMIT
    source_root.mkdir(parents=True, exist_ok=True)
    index_paths = []
    try:
        for target, target_observed in zip(TARGETS, observed):
            target_root = source_root / target
            target_root.mkdir()
            (target_root / "oracle.json").write_text(
                json.dumps(report_for("rust-oracle", target, target_observed)), encoding="utf-8"
            )
            (target_root / "native.json").write_text(
                json.dumps(report_for("native-stage0", target, target_observed)), encoding="utf-8"
            )
            (target_root / "comparison.json").write_text(
                json.dumps(comparison_for(target, "pass" if target_observed else "pending")),
                encoding="utf-8",
            )
            (target_root / "index.json").write_text(
                json.dumps(target_index(target, target_observed)), encoding="utf-8"
            )
            index_paths.append(f"ci-artifacts/v4-m1-01/{SOURCE_COMMIT}/{target}/index.json")

        aggregate_root = source_root / "aggregate"
        aggregate_root.mkdir()
        aggregate_path = aggregate_root / "index.json"
        aggregate_path.write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "suite": "v4-m1-06-aggregate",
                    "task": "V4-M1-01",
                    "source_commit": SOURCE_COMMIT,
                    "status": status,
                    "indexes": [
                        {"target": target, "index": index_path}
                        for target, index_path in zip(TARGETS, index_paths)
                    ],
                }
            ),
            encoding="utf-8",
        )
        yield aggregate_path, index_paths
    finally:
        shutil.rmtree(source_root, ignore_errors=True)
        for parent in (source_root.parent,):
            try:
                parent.rmdir()
            except OSError:
                break


class SemanticFixtureEvidenceAggregateTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._created_artifact_root = not ARTIFACT_ROOT.exists()
        ARTIFACT_ROOT.mkdir(exist_ok=True)

    @classmethod
    def tearDownClass(cls):
        if cls._created_artifact_root:
            ARTIFACT_ROOT.rmdir()

    def run_aggregate(self, index_path: pathlib.Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [
                sys.executable,
                str(AGGREGATE),
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

    def test_pass_requires_both_supported_targets(self):
        with scenario() as (index_path, _):
            result = self.run_aggregate(index_path)
            self.assertEqual(result.returncode, 0, result.stderr)
            output = json.loads(result.stdout)
            self.assertEqual(output["status"], "pass")
            self.assertEqual([item["target"] for item in output["targets"]], TARGETS)

    def test_pending_target_keeps_aggregate_pending(self):
        with scenario((True, False), status="pending") as (index_path, _):
            result = self.run_aggregate(index_path)
            self.assertEqual(result.returncode, 2, result.stderr)
            output = json.loads(result.stdout)
            self.assertEqual(output["status"], "pending")
            self.assertEqual(output["targets"][1]["status"], "pending")

    def test_rejects_single_target_aggregate(self):
        with scenario() as (index_path, _):
            value = json.loads(index_path.read_text(encoding="utf-8"))
            value["indexes"] = value["indexes"][:1]
            index_path.write_text(json.dumps(value), encoding="utf-8")
            result = self.run_aggregate(index_path)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("target", result.stderr.lower())

    def test_rejects_verified_claim_when_one_target_is_pending(self):
        with scenario((True, False), status="pass") as (index_path, _):
            result = self.run_aggregate(index_path)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("status", result.stderr.lower())

    def test_rejects_cross_target_index_reference(self):
        with scenario() as (index_path, index_paths):
            value = json.loads(index_path.read_text(encoding="utf-8"))
            value["indexes"][0]["index"] = index_paths[1]
            index_path.write_text(json.dumps(value), encoding="utf-8")
            result = self.run_aggregate(index_path)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("target", result.stderr.lower())

    def test_rejects_stale_source_commit_before_namespace_resolution(self):
        with scenario() as (index_path, _):
            value = json.loads(index_path.read_text(encoding="utf-8"))
            value["source_commit"] = "b" * 40
            index_path.write_text(json.dumps(value), encoding="utf-8")
            result = self.run_aggregate(index_path)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("current", result.stderr.lower())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
