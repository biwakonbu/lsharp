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
    def run_producer(
        self,
        root,
        compiler,
        wasmtime,
        output,
        work_dir,
        extra=None,
        fixture_id="valid/syntax-basic",
        fixture_ids=None,
        wasm_tools=None,
    ):
        if wasm_tools is None:
            wasm_tools = root / "fake-wasm-tools.py"
            make_executable(wasm_tools, "#!/bin/sh\nexit 0\n")
        selected_fixture_ids = fixture_ids or [fixture_id]
        command = [
            sys.executable,
            str(PRODUCER),
            "--manifest",
            str(MANIFEST),
            "--root",
            str(ROOT),
            "--target",
            "aarch64-apple-darwin",
            "--source-commit",
            SOURCE_COMMIT,
            "--compiler",
            str(compiler),
            "--wasmtime",
            str(wasmtime),
            "--wasm-tools",
            str(wasm_tools),
            "--work-dir",
            str(work_dir),
            "--output",
            str(output),
        ]
        fixture_arguments = []
        for selected_fixture_id in selected_fixture_ids:
            fixture_arguments.extend(["--fixture-id", selected_fixture_id])
        command[command.index("--target"):command.index("--target")] = fixture_arguments
        if extra:
            command.extend(extra)
        return subprocess.run(command, cwd=root, capture_output=True, text=True, check=False)

    def test_writes_explicit_compilation_and_runtime_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "fake-compiler.py"
            wasmtime = root / "fake-wasmtime.py"
            wasm_tools = root / "fake-wasm-tools.py"
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
            make_executable(wasm_tools, "#!/bin/sh\nexit 0\n")
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
                    "--wasm-tools",
                    str(wasm_tools),
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

    def test_rejects_invalid_wasm_before_runtime(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "fake-compiler.py"
            wasm_tools = root / "rejecting-wasm-tools.py"
            wasmtime = root / "fake-wasmtime.py"
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            runtime_log = root / "runtime.log"
            make_executable(
                compiler,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'not-wasm')\n",
            )
            make_executable(
                wasm_tools,
                "#!/bin/sh\n"
                "echo invalid-wasm >&2\n"
                "exit 1\n",
            )
            make_executable(
                wasmtime,
                "#!/bin/sh\n"
                f"printf 'executed' > {runtime_log}\n"
                "exit 0\n",
            )

            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_id="valid/syntax-basic",
                wasm_tools=wasm_tools,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("wasm validation", result.stderr.lower())
            self.assertFalse(output.exists())
            self.assertFalse(runtime_log.exists())

    def test_compiler_cannot_mutate_manifest_source(self):
        source_path = ROOT / "examples/types.ls"
        original_source = source_path.read_bytes()
        try:
            with tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                compiler = root / "mutating-compiler.py"
                wasmtime = root / "fake-wasmtime.py"
                work_dir = root / "work"
                work_dir.mkdir()
                output = root / "report.json"
                make_executable(
                    compiler,
                    "#!/usr/bin/env python3\n"
                    "import pathlib, sys\n"
                    "source = pathlib.Path(sys.argv[2])\n"
                    "source.write_text('(defn main [] 99)\\n', encoding='utf-8')\n"
                    "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'fake-wasm')\n",
                )
                make_executable(wasmtime, "#!/bin/sh\nprintf '42\\n'\n")
                result = self.run_producer(
                    root,
                    compiler,
                    wasmtime,
                    output,
                    work_dir,
                    fixture_id="valid/adt-pattern",
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(source_path.read_bytes(), original_source)
        finally:
            source_path.write_bytes(original_source)

    def test_materializes_declared_runtime_input_snapshot(self):
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
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'fake-wasm')\n",
            )
            make_executable(
                wasmtime,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "from pathlib import Path\n"
                "if '--dir=.' not in sys.argv:\n"
                "    raise SystemExit('runtime input directory was not preopened')\n"
                "print(Path('input.txt').read_text(encoding='utf-8'))\n",
            )
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_id="valid/io-read-file",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(report["fixtures"][0]["runtime"]["stdout"], "payload\n")
            self.assertEqual((work_dir / "input.txt").read_text(encoding="utf-8"), "payload")

    def test_preopens_explicit_empty_runtime_directory_for_missing_file(self):
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
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'fake-wasm')\n",
            )
            make_executable(
                wasmtime,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "from pathlib import Path\n"
                "if '--dir=.' not in sys.argv:\n"
                "    raise SystemExit('explicit empty runtime directory was not preopened')\n"
                "if Path('input.txt').exists():\n"
                "    raise SystemExit('missing-file fixture unexpectedly materialized input.txt')\n",
            )
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_id="valid/io-read-file-missing",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(
                report["fixtures"][0]["runtime"],
                {"status": "observed", "exit_code": 0, "stdout": "", "stderr": ""},
            )
            self.assertFalse((work_dir / "input.txt").exists())

    def test_rejects_overwriting_declared_runtime_input_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "fake-compiler.py"
            wasmtime = root / "fake-wasmtime.py"
            work_dir = root / "work"
            work_dir.mkdir()
            (work_dir / "input.txt").write_text("existing", encoding="utf-8")
            output = root / "report.json"
            make_executable(
                compiler,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'fake-wasm')\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_id="valid/io-read-file",
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("runtime input", result.stderr.lower())
            self.assertFalse(output.exists())
            self.assertEqual((work_dir / "input.txt").read_text(encoding="utf-8"), "existing")

    def test_passes_declared_runtime_stdin_snapshot(self):
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
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'fake-wasm')\n",
            )
            make_executable(
                wasmtime,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "if '--dir=.' in sys.argv:\n"
                "    raise SystemExit('stdin-only fixture must not preopen a directory')\n"
                "print(sys.stdin.read(), end='')\n",
            )
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_id="valid/io-read-stdin",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(report["fixtures"][0]["runtime"]["stdout"], "payload")

    def test_writes_invalid_report_when_code_and_span_are_explicit(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "invalid-compiler.py"
            wasmtime = root / "fake-wasmtime.py"
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                compiler,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "sys.stderr.write('Error: [LS1001] undefined value (15..28)\\n')\n"
                "raise SystemExit(1)\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_id="invalid/type-undefined-value",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            fixture = report["fixtures"][0]
            self.assertEqual(
                fixture["diagnostics"],
                [
                    {
                        "code": "LS1001",
                        "span": {
                            "start": {"line": 1, "column": 16},
                            "end": {"line": 1, "column": 29},
                        },
                    }
                ],
            )
            self.assertEqual(fixture["exit_code"], 1)
            self.assertEqual(fixture["artifact"], {"status": "not-applicable"})
            self.assertEqual(
                fixture["runtime"],
                {"status": "not-run", "exit_code": None, "stdout": None, "stderr": None},
            )

    def test_writes_invalid_report_from_multiline_span_struct(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "invalid-span-struct-compiler.py"
            wasmtime = root / "fake-wasmtime.py"
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                compiler,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "sys.stderr.write('Error: [LS3001] unsupported literal pattern "
                "Span { start:\\n  │ 214, end: 216 }\\n')\n"
                "raise SystemExit(1)\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_id="invalid/record-field-pattern-literal",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(
                report["fixtures"][0]["diagnostics"],
                [
                    {
                        "code": "LS3001",
                        "span": {
                            "start": {"line": 8, "column": 19},
                            "end": {"line": 8, "column": 21},
                        },
                    }
                ],
            )

    def test_rejects_invalid_reports_without_explicit_code_or_span(self):
        cases = [
            (
                "invalid/lexer-unexpected-character",
                "Error: unexpected character '@' (0..1)\n",
                "diagnostic code",
            ),
            (
                "invalid/module-not-found",
                "Error: [LS3102] module 'MissingModule' was not found\n",
                "diagnostic span",
            ),
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            wasmtime = root / "fake-wasmtime.py"
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            for index, (fixture_id, diagnostic, expected_error) in enumerate(cases):
                compiler = root / f"invalid-compiler-{index}.py"
                work_dir = root / f"work-{index}"
                work_dir.mkdir()
                output = root / f"report-{index}.json"
                make_executable(
                    compiler,
                    "#!/usr/bin/env python3\n"
                    "import sys\n"
                    f"sys.stderr.write({diagnostic!r})\n"
                    "raise SystemExit(1)\n",
                )
                result = self.run_producer(
                    root,
                    compiler,
                    wasmtime,
                    output,
                    work_dir,
                    fixture_id=fixture_id,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_error, result.stderr.lower())
                self.assertFalse(output.exists())

    def test_rejects_invalid_commit(self):
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

    def test_writes_sorted_batch_report_with_isolated_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "batch-compiler.py"
            wasmtime = root / "fake-wasmtime.py"
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                compiler,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "source = pathlib.Path(sys.argv[2])\n"
                "if source.name == 'invalid-type.ls':\n"
                "    sys.stderr.write('Error: [LS1001] undefined value (15..28)\\n')\n"
                "    raise SystemExit(1)\n"
                "out = pathlib.Path(sys.argv[sys.argv.index('-o') + 1])\n"
                "out.write_bytes(source.name.encode('utf-8'))\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nprintf '42\\n'\n")
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_ids=["valid/syntax-basic", "invalid/type-undefined-value"],
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(
                [fixture["id"] for fixture in report["fixtures"]],
                ["invalid/type-undefined-value", "valid/syntax-basic"],
            )
            invalid_fixture, valid_fixture = report["fixtures"]
            self.assertEqual(invalid_fixture["artifact"], {"status": "not-applicable"})
            self.assertEqual(invalid_fixture["runtime"]["status"], "not-run")
            self.assertEqual(valid_fixture["artifact"]["status"], "observed")
            self.assertEqual(valid_fixture["runtime"]["stdout"], "42\n")
            self.assertFalse((work_dir / "0000" / "semantic-fixture.wasm").exists())
            self.assertEqual(
                (work_dir / "0001" / "semantic-fixture.wasm").read_bytes(),
                b"hello.ls",
            )

    def test_rejects_duplicate_fixture_ids(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            compiler = root / "compiler"
            wasmtime = root / "wasmtime"
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(compiler, "#!/bin/sh\nexit 0\n")
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                compiler,
                wasmtime,
                output,
                work_dir,
                fixture_ids=["valid/syntax-basic", "valid/syntax-basic"],
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("duplicate", result.stderr.lower())
            self.assertFalse(output.exists())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
