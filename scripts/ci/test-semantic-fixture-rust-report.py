#!/usr/bin/env python3

"""Contract tests for the explicit Rust-oracle fixture report producer."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import stat
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
PRODUCER = SCRIPTS_DIR / "semantic_fixture_rust_report.py"
SOURCE_COMMIT = "a" * 40


def make_executable(path, body):
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


class SemanticFixtureRustReportTest(unittest.TestCase):
    def run_producer(self, root, compiler, wasmtime, output, work_dir, extra=None):
        command = [
            sys.executable,
            str(PRODUCER),
            "--manifest",
            str(MANIFEST),
            "--root",
            str(ROOT),
            "--fixture-id",
            "valid/syntax-basic",
            "--target",
            "aarch64-apple-darwin",
            "--source-commit",
            SOURCE_COMMIT,
            "--compiler",
            str(compiler),
            "--wasmtime",
            str(wasmtime),
            "--work-dir",
            str(work_dir),
            "--output",
            str(output),
        ]
        if extra:
            command.extend(extra)
        return subprocess.run(command, cwd=root, capture_output=True, text=True, check=False)

    def test_writes_explicit_compilation_and_runtime_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "fake-compiler.py"
            wasmtime = root / "fake-wasmtime.py"
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                compiler,
                "#!/usr/bin/env python3\n"
                "import os, pathlib, sys\n"
                "pathlib.Path(os.environ['FAKE_LOG']).write_text(os.environ['LSHARP_DISABLE_EMBEDDED_COMPONENT'], encoding='utf-8')\n"
                "out = pathlib.Path(sys.argv[sys.argv.index('-o') + 1])\n"
                "out.write_bytes(b'fake-wasm')\n",
            )
            make_executable(
                wasmtime,
                "#!/usr/bin/env python3\n"
                "print('42')\n",
            )
            environment = os.environ.copy()
            environment["FAKE_LOG"] = str(root / "compiler.log")
            result = subprocess.run(
                [
                    sys.executable,
                    str(PRODUCER),
                    "--manifest",
                    str(MANIFEST),
                    "--root",
                    str(ROOT),
                    "--fixture-id",
                    "valid/syntax-basic",
                    "--target",
                    "aarch64-apple-darwin",
                    "--source-commit",
                    SOURCE_COMMIT,
                    "--compiler",
                    str(compiler),
                    "--wasmtime",
                    str(wasmtime),
                    "--work-dir",
                    str(work_dir),
                    "--output",
                    str(output),
                ],
                cwd=root,
                env=environment,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            fixture = report["fixtures"][0]
            self.assertEqual(report["producer"], "rust-oracle")
            self.assertEqual(report["source_commit"], SOURCE_COMMIT)
            self.assertEqual(fixture["exit_code"], 0)
            self.assertEqual(fixture["artifact"]["status"], "observed")
            self.assertEqual(
                fixture["artifact"]["sha256"],
                "sha256:" + hashlib.sha256(b"fake-wasm").hexdigest(),
            )
            self.assertEqual(fixture["runtime"], {"status": "observed", "exit_code": 0, "stdout": "42\n", "stderr": ""})
            self.assertEqual((root / "compiler.log").read_text(encoding="utf-8"), "1")

    def test_rejects_invalid_commit_and_non_valid_fixture_scope(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "compiler"
            wasmtime = root / "wasmtime"
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(compiler, "#!/bin/sh\nexit 0\n")
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            invalid_commit = self.run_producer(root, compiler, wasmtime, output, work_dir, ["--source-commit", "bad"])
            self.assertNotEqual(invalid_commit.returncode, 0)
            self.assertIn("source_commit", invalid_commit.stderr)
            invalid_fixture = subprocess.run(
                [
                    sys.executable,
                    str(PRODUCER),
                    "--manifest",
                    str(MANIFEST),
                    "--root",
                    str(ROOT),
                    "--fixture-id",
                    "invalid/lexer-unexpected-character",
                    "--target",
                    "aarch64-apple-darwin",
                    "--source-commit",
                    SOURCE_COMMIT,
                    "--compiler",
                    str(compiler),
                    "--wasmtime",
                    str(wasmtime),
                    "--work-dir",
                    str(work_dir),
                    "--output",
                    str(output),
                ],
                cwd=root,
                capture_output=True,
                text=True,
                check=False,
            )
            self.assertNotEqual(invalid_fixture.returncode, 0)
            self.assertIn("valid", invalid_fixture.stderr.lower())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
