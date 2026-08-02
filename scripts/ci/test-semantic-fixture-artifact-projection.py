#!/usr/bin/env python3

"""Contract tests for the source-to-Wasm static projection boundary."""

from __future__ import annotations

import hashlib
import json
import pathlib
import stat
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
PROJECTION = SCRIPTS_DIR / "semantic_fixture_artifact_projection.py"
DIFF = SCRIPTS_DIR / "semantic_fixture_artifact_projection_diff.py"


def make_executable(path: pathlib.Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def run_projection(
    root: pathlib.Path,
    tool: pathlib.Path,
    artifact: pathlib.Path,
    output: pathlib.Path,
    source_commit: str | None = None,
):
    if source_commit is None:
        source_commit = subprocess.check_output(
            ["git", "-C", str(ROOT), "rev-parse", "--verify", "HEAD"], text=True
        ).strip()
    return subprocess.run(
        [
            sys.executable,
            str(PROJECTION),
            "--manifest",
            str(MANIFEST),
            "--root",
            str(ROOT),
            "--fixture-id",
            "valid/nested-record-pattern",
            "--target",
            "aarch64-apple-darwin",
            "--source-commit",
            source_commit,
            "--artifact",
            str(artifact),
            "--wasm-tools",
            str(tool),
            "--output",
            str(output),
        ],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )


class SemanticFixtureArtifactProjectionTest(unittest.TestCase):
    def test_projects_source_and_static_wasm_shape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            tool = root / "fake-wasm-tools.py"
            artifact = root / "artifact.wasm"
            output = root / "projection.json"
            artifact.write_bytes(b"\x00asm\x01\x00\x00\x00fake")
            make_executable(
                tool,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "assert sys.argv[1] == 'print'\n"
                "print('(module')\n"
                "print('  (import \\\"env\\\" \\\"print\\\" (func $print))')\n"
                "print('  (table 3 7 funcref)')\n"
                "print('  (export \\\"_start\\\" (func $start))')\n"
                "print(')')\n",
            )

            result = run_projection(root, tool, artifact, output)

            self.assertEqual(result.returncode, 0, result.stderr)
            projection = json.loads(output.read_text(encoding="utf-8"))
            fixture = projection["fixtures"][0]
            self.assertEqual(projection["suite"], "v4-m1-07")
            self.assertEqual(fixture["id"], "valid/nested-record-pattern")
            self.assertEqual(fixture["source_sha256"], "sha256:" + hashlib.sha256(
                (ROOT / "scripts/ci/semantic-fixtures/nested-record-pattern.ls").read_bytes()
            ).hexdigest())
            self.assertEqual(fixture["imports"], [{"module": "env", "name": "print", "kind": "func"}])
            self.assertEqual(fixture["tables"], [{"min": 3, "max": 7}])
            self.assertEqual(fixture["exports"], [{"name": "_start", "kind": "func"}])

    def test_rejects_static_projection_failure_before_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            tool = root / "rejecting-wasm-tools.py"
            artifact = root / "artifact.wasm"
            output = root / "projection.json"
            artifact.write_bytes(b"\x00asm\x01\x00\x00\x00fake")
            make_executable(tool, "#!/bin/sh\necho malformed projection >&2\nexit 9\n")

            result = run_projection(root, tool, artifact, output)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("projection", result.stderr.lower())
            self.assertFalse(output.exists())

    def test_rejects_stale_source_commit_before_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            tool = root / "fake-wasm-tools.py"
            artifact = root / "artifact.wasm"
            output = root / "projection.json"
            artifact.write_bytes(b"\x00asm\x01\x00\x00\x00fake")
            make_executable(
                tool,
                "#!/bin/sh\n"
                "echo should-not-run >&2\n"
                "exit 97\n",
            )

            result = run_projection(
                root,
                tool,
                artifact,
                output,
                source_commit="0" * 40,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("source_commit", result.stderr)
            self.assertFalse(output.exists())

    def test_diff_rejects_rust_native_import_or_table_mismatch(self):
        base = {
            "schema_version": 1,
            "suite": "v4-m1-07",
            "producer": "static-wasm-artifact",
            "target": "aarch64-apple-darwin",
            "source_commit": "a" * 40,
            "fixtures": [
                {
                    "id": "valid/nested-record-pattern",
                    "source": "scripts/ci/semantic-fixtures/nested-record-pattern.ls",
                    "source_sha256": "sha256:" + "b" * 64,
                    "artifact_sha256": "sha256:" + "c" * 64,
                    "required_observables": ["ftable", "imports"],
                    "imports": [{"module": "env", "name": "print", "kind": "func"}],
                    "tables": [{"min": 3, "max": 7}],
                    "exports": [{"name": "_start", "kind": "func"}],
                }
            ],
        }
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            oracle = root / "oracle.json"
            native = root / "native.json"
            oracle.write_text(json.dumps(base), encoding="utf-8")
            changed = json.loads(json.dumps(base))
            changed["fixtures"][0]["tables"] = [{"min": 4, "max": 7}]
            native.write_text(json.dumps(changed), encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(DIFF), "--oracle", str(oracle), "--native", str(native)],
                capture_output=True,
                text=True,
                check=False,
            )

            self.assertEqual(result.returncode, 1)
            self.assertIn("tables", result.stdout.lower())

    def test_diff_binds_projection_to_existing_runtime_reports(self):
        projection = {
            "schema_version": 1,
            "suite": "v4-m1-07",
            "producer": "static-wasm-artifact",
            "target": "aarch64-apple-darwin",
            "source_commit": "a" * 40,
            "fixtures": [
                {
                    "id": "valid/nested-record-pattern",
                    "source": "scripts/ci/semantic-fixtures/nested-record-pattern.ls",
                    "source_sha256": "sha256:" + "b" * 64,
                    "artifact_sha256": "sha256:" + "c" * 64,
                    "required_observables": ["ftable", "imports"],
                    "imports": [],
                    "tables": [{"min": 3, "max": 7}],
                    "exports": [],
                }
            ],
        }
        report = {
            "schema_version": 1,
            "suite": "v4-m1-01",
            "producer": "rust-oracle",
            "target": "aarch64-apple-darwin",
            "source_commit": "a" * 40,
            "fixtures": [
                {
                    "id": "valid/nested-record-pattern",
                    "artifact": {"status": "observed", "sha256": "sha256:" + "c" * 64, "size": 9},
                    "runtime": {
                        "status": "observed",
                        "artifact_sha256": "sha256:" + "c" * 64,
                    },
                }
            ],
        }
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            oracle = root / "oracle.json"
            native = root / "native.json"
            oracle_report = root / "oracle-report.json"
            native_report = root / "native-report.json"
            for path, value in ((oracle, projection), (native, projection), (oracle_report, report), (native_report, report)):
                path.write_text(json.dumps(value), encoding="utf-8")
            result = subprocess.run(
                [
                    sys.executable,
                    str(DIFF),
                    "--oracle",
                    str(oracle),
                    "--native",
                    str(native),
                    "--oracle-report",
                    str(oracle_report),
                    "--native-report",
                    str(native_report),
                ],
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout)["status"], "pass")


if __name__ == "__main__":
    unittest.main()
