#!/usr/bin/env python3

"""Contract tests for the explicit native-stage0 fixture report producer."""

from __future__ import annotations

import hashlib
import json
import os
import pathlib
import shutil
import stat
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPTS_DIR.parent.parent
MANIFEST = SCRIPTS_DIR / "semantic-fixture-matrix.json"
PRODUCER = SCRIPTS_DIR / "semantic_fixture_native_report.py"
SOURCE_COMMIT = "a" * 40
TARGET = "aarch64-apple-darwin"


def make_executable(path: pathlib.Path, body: str) -> None:
    path.write_text(body, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)


def write_stage0_manifest(root: pathlib.Path, target: str = TARGET, source_commit: str = SOURCE_COMMIT) -> pathlib.Path:
    stage0 = root / "stage0"
    stage0.mkdir()
    manifest = stage0 / "manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "kind": "lsharp-native-selfhost-stage0",
                "target": target,
                "source_commit": source_commit,
                "compiler": "bin/compiler",
                "transport_driver": "bin/transport-driver",
                "materializer": "bin/materializer",
            }
        ),
        encoding="utf-8",
    )
    return manifest


class SemanticFixtureNativeReportTest(unittest.TestCase):
    def run_producer(
        self,
        root: pathlib.Path,
        runner: pathlib.Path,
        wasmtime: pathlib.Path,
        stage0_manifest: pathlib.Path,
        output: pathlib.Path,
        work_dir: pathlib.Path,
        fixture_id: str = "valid/syntax-basic",
        extra: list[str] | None = None,
        fixture_ids: list[str] | None = None,
        wasm_tools: pathlib.Path | None = None,
    ) -> subprocess.CompletedProcess[str]:
        if wasm_tools is None:
            wasm_tools = root / "fake-wasm-tools.py"
            make_executable(wasm_tools, "#!/bin/sh\nexit 0\n")
        selected_fixture_ids = fixture_ids or [fixture_id]
        declared_runner = stage0_manifest.parent / "bin" / "compiler"
        declared_runner.parent.mkdir(exist_ok=True)
        shutil.copy2(runner, declared_runner)
        declared_runner.chmod(declared_runner.stat().st_mode | stat.S_IXUSR)
        command = [
            sys.executable,
            str(PRODUCER),
            "--manifest",
            str(MANIFEST),
            "--root",
            str(ROOT),
            "--target",
            TARGET,
            "--source-commit",
            SOURCE_COMMIT,
            "--runner",
            str(declared_runner),
            "--wasmtime",
            str(wasmtime),
            "--wasm-tools",
            str(wasm_tools),
            "--stage0-manifest",
            str(stage0_manifest),
            "--work-dir",
            str(work_dir),
            "--output",
            str(output),
        ]
        fixture_arguments: list[str] = []
        for selected_fixture_id in selected_fixture_ids:
            fixture_arguments.extend(["--fixture-id", selected_fixture_id])
        command[command.index("--target"):command.index("--target")] = fixture_arguments
        if extra:
            command.extend(extra)
        return subprocess.run(command, cwd=root, capture_output=True, text=True, check=False)

    def test_writes_explicit_native_compilation_and_runtime_observation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "fake-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            wasm_tools = root / "fake-wasm-tools.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import os, pathlib, sys\n"
                "if 'LSHARP_PATH' in os.environ or 'LSHARP_DISABLE_EMBEDDED_COMPONENT' in os.environ:\n"
                "    raise SystemExit('fallback environment leaked')\n"
                "out = pathlib.Path(sys.argv[sys.argv.index('-o') + 1])\n"
                "out.write_bytes(b'native-wasm')\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nprintf '42\\n'\n")
            make_executable(wasm_tools, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                wasm_tools=wasm_tools,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            fixture = report["fixtures"][0]
            self.assertEqual(report["producer"], "native-stage0")
            self.assertEqual(report["source_commit"], SOURCE_COMMIT)
            self.assertEqual(fixture["exit_code"], 0)
            self.assertEqual(
                fixture["source_sha256"],
                "sha256:" + hashlib.sha256((ROOT / "examples/hello.ls").read_bytes()).hexdigest(),
            )
            self.assertEqual(fixture["artifact"]["status"], "observed")
            self.assertEqual(
                fixture["artifact"]["sha256"],
                "sha256:" + hashlib.sha256(b"native-wasm").hexdigest(),
            )
            self.assertEqual(
                fixture["runtime"],
                {
                    "status": "observed",
                    "exit_code": 0,
                    "stdout": "42\n",
                    "stderr": "",
                    "artifact_sha256": "sha256:" + hashlib.sha256(b"native-wasm").hexdigest(),
                },
            )

    def test_rejects_invalid_wasm_before_runtime(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "fake-native-runner.py"
            wasm_tools = root / "rejecting-wasm-tools.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            runtime_log = root / "runtime.log"
            make_executable(
                runner,
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
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                wasm_tools=wasm_tools,
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("wasm validation", result.stderr.lower())
            self.assertFalse(output.exists())
            self.assertFalse(runtime_log.exists())

    def test_runner_cannot_mutate_manifest_source(self):
        source_path = ROOT / "examples/types.ls"
        original_source = source_path.read_bytes()
        try:
            with tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                runner = root / "mutating-native-runner.py"
                wasmtime = root / "fake-wasmtime.py"
                stage0_manifest = write_stage0_manifest(root)
                work_dir = root / "work"
                work_dir.mkdir()
                output = root / "report.json"
                make_executable(
                    runner,
                    "#!/usr/bin/env python3\n"
                    "import pathlib, sys\n"
                    "source = pathlib.Path(sys.argv[2])\n"
                    "source.write_text('(defn main [] 99)\\n', encoding='utf-8')\n"
                    "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
                )
                make_executable(wasmtime, "#!/bin/sh\nprintf '42\\n0\\n'\n")
                result = self.run_producer(
                    root,
                    runner,
                    wasmtime,
                    stage0_manifest,
                    output,
                    work_dir,
                    fixture_id="valid/adt-pattern",
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(source_path.read_bytes(), original_source)
        finally:
            source_path.write_bytes(original_source)

    def test_runs_runner_in_task_owned_fixture_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "relative-write-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path('runner-relative-residue').write_text('unexpected\\n', encoding='utf-8')\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nprintf '42\\n'\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="valid/syntax-basic",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse((root / "runner-relative-residue").exists())
            self.assertTrue((work_dir / "runner-relative-residue").is_file())

    def test_rejects_source_mutation_during_runtime_before_report(self):
        source_path = ROOT / "examples/hello.ls"
        original_source = source_path.read_bytes()
        try:
            with tempfile.TemporaryDirectory() as directory:
                root = pathlib.Path(directory)
                mutated_source = original_source + b"\n; mutation\n"
                runner = root / "fake-native-runner.py"
                wasmtime = root / "mutating-wasmtime.py"
                stage0_manifest = write_stage0_manifest(root)
                work_dir = root / "work"
                work_dir.mkdir()
                output = root / "report.json"
                make_executable(
                    runner,
                    "#!/usr/bin/env python3\n"
                    "import pathlib, sys\n"
                    "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
                )
                make_executable(
                    wasmtime,
                    f"#!/usr/bin/env python3\n"
                    f"import pathlib\n"
                    f"pathlib.Path({str(source_path)!r}).write_bytes({mutated_source!r})\n"
                    "print('42')\n",
                )

                result = self.run_producer(
                    root,
                    runner,
                    wasmtime,
                    stage0_manifest,
                    output,
                    work_dir,
                    fixture_id="valid/syntax-basic",
                )

                self.assertNotEqual(result.returncode, 0)
                self.assertIn("source", result.stderr.lower())
                self.assertFalse(output.exists())

        finally:
            source_path.write_bytes(original_source)

    def test_rejects_unexpected_runtime_failure_before_report(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "fake-native-runner.py"
            wasmtime = root / "failing-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
            )
            make_executable(
                wasmtime,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "sys.stderr.write('runtime failed\\n')\n"
                "raise SystemExit(23)\n",
            )
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="valid/syntax-basic",
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("runtime", result.stderr.lower())
            self.assertIn("expected", result.stderr.lower())
            self.assertFalse(output.exists())

    def test_rejects_unexpected_runtime_output_before_report(self):
        cases = [
            ("stdout", "#!/bin/sh\nprintf 'unexpected\\n'\n"),
            ("stderr", "#!/bin/sh\nprintf '42\\n'\nprintf 'unexpected\\n' >&2\n"),
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "fake-native-runner.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
            )
            for stream, body in cases:
                wasmtime = root / f"unexpected-{stream}-wasmtime.py"
                output = root / f"unexpected-{stream}.json"
                (work_dir / stream).mkdir()
                make_executable(wasmtime, body)
                result = self.run_producer(
                    root,
                    runner,
                    wasmtime,
                    stage0_manifest,
                    output,
                    work_dir / stream,
                    fixture_id="valid/syntax-basic",
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(f"runtime {stream}", result.stderr.lower())
                self.assertIn("expected", result.stderr.lower())
                self.assertFalse(output.exists())

    def test_materializes_declared_runtime_input_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "fake-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
            )
            make_executable(
                wasmtime,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "from pathlib import Path\n"
                "if '--dir=.' not in sys.argv:\n"
                "    raise SystemExit('runtime input directory was not preopened')\n"
                "print(Path('input.txt').read_text(encoding='utf-8'), end='')\n",
            )
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="valid/io-read-file",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(report["fixtures"][0]["runtime"]["stdout"], "payload")
            self.assertEqual((work_dir / "input.txt").read_text(encoding="utf-8"), "payload")

    def test_preopens_explicit_empty_runtime_directory_for_missing_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "fake-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
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
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="valid/io-read-file-missing",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(
                report["fixtures"][0]["runtime"],
                {
                    "status": "observed",
                    "exit_code": 0,
                    "stdout": "",
                    "stderr": "",
                    "artifact_sha256": "sha256:" + hashlib.sha256(b"native-wasm").hexdigest(),
                },
            )
            self.assertFalse((work_dir / "input.txt").exists())

    def test_rejects_overwriting_declared_runtime_input_snapshot(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "fake-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            (work_dir / "input.txt").write_text("existing", encoding="utf-8")
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
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
            runner = root / "fake-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'native-wasm')\n",
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
                runner,
                wasmtime,
                stage0_manifest,
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
            runner = root / "invalid-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "sys.stderr.write('Error: [LS1001] undefined value (15..28)\\n')\n"
                "raise SystemExit(1)\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="invalid/type-undefined-value",
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            fixture = json.loads(output.read_text(encoding="utf-8"))["fixtures"][0]
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

    def test_rejects_invalid_fixture_with_unexpected_compile_exit_before_report(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "invalid-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "sys.stderr.write('Error: [LS1001] undefined value (15..28)\\n')\n"
                "raise SystemExit(2)\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="invalid/type-undefined-value",
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("compile exit", result.stderr.lower())
            self.assertIn("expected", result.stderr.lower())
            self.assertFalse(output.exists())

    def test_writes_invalid_report_for_rust_style_structured_span(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "invalid-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import sys\n"
                "sys.stderr.write('Error: [LS3001] unsupported literal pattern "
                "Span { start:\\n  │ 214, end: 216 }\\n')\n"
                "raise SystemExit(1)\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")

            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="invalid/record-field-pattern-literal",
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            fixture = json.loads(output.read_text(encoding="utf-8"))["fixtures"][0]
            self.assertEqual(
                fixture["diagnostics"],
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
            self.assertEqual(fixture["exit_code"], 1)
            self.assertEqual(fixture["artifact"], {"status": "not-applicable"})
            self.assertEqual(
                fixture["runtime"],
                {
                    "status": "not-run",
                    "exit_code": None,
                    "stdout": None,
                    "stderr": None,
                    "artifact_sha256": None,
                },
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
            stage0_manifest = write_stage0_manifest(root)
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            for index, (fixture_id, diagnostic, expected_error) in enumerate(cases):
                runner = root / f"invalid-native-runner-{index}.py"
                work_dir = root / f"work-{index}"
                work_dir.mkdir()
                output = root / f"report-{index}.json"
                make_executable(
                    runner,
                    "#!/usr/bin/env python3\n"
                    "import sys\n"
                    f"sys.stderr.write({diagnostic!r})\n"
                    "raise SystemExit(1)\n",
                )
                result = self.run_producer(
                    root,
                    runner,
                    wasmtime,
                    stage0_manifest,
                    output,
                    work_dir,
                    fixture_id=fixture_id,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(expected_error, result.stderr.lower())
                self.assertFalse(output.exists())

    def test_rejects_invalid_fixture_that_writes_an_artifact(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "invalid-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'unexpected')\n"
                "sys.stderr.write('Error: [LS1001] undefined value (15..28)\\n')\n"
                "raise SystemExit(1)\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_id="invalid/type-undefined-value",
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("unexpected wasm artifact", result.stderr.lower())
            self.assertFalse(output.exists())

    def test_rejects_stage0_manifest_provenance_mismatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "runner"
            wasmtime = root / "wasmtime"
            stage0_manifest = write_stage0_manifest(root, source_commit="b" * 40)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(runner, "#!/bin/sh\nexit 0\n")
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(root, runner, wasmtime, stage0_manifest, output, work_dir)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("source_commit", result.stderr)
            self.assertFalse(output.exists())

    def test_rejects_runner_not_bound_to_stage0_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "unbound-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            manifest = json.loads(stage0_manifest.read_text(encoding="utf-8"))
            manifest["compiler"] = "bin/other-compiler"
            stage0_manifest.write_text(json.dumps(manifest), encoding="utf-8")
            other_compiler = stage0_manifest.parent / "bin" / "other-compiler"
            other_compiler.parent.mkdir(exist_ok=True)
            make_executable(other_compiler, "#!/bin/sh\nexit 0\n")
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'fake-wasm')\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("runner", result.stderr.lower())
            self.assertIn("manifest", result.stderr.lower())
            self.assertFalse(output.exists())

    def test_writes_sorted_batch_report_with_isolated_artifacts(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "batch-native-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(
                runner,
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
                runner,
                wasmtime,
                stage0_manifest,
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

    def test_cleans_partial_batch_staging_after_late_compile_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "failing-batch-runner.py"
            wasmtime = root / "fake-wasmtime.py"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            (work_dir / "caller-owned-sentinel").write_text("keep\n", encoding="utf-8")
            output = root / "report.json"
            make_executable(
                runner,
                "#!/usr/bin/env python3\n"
                "import pathlib, sys\n"
                "source = pathlib.Path(sys.argv[2])\n"
                "if source.name == 'argv-program-only.ls':\n"
                "    sys.stderr.write('late runner failure\\n')\n"
                "    raise SystemExit(1)\n"
                "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'first')\n",
            )
            make_executable(wasmtime, "#!/bin/sh\nprintf '42\\n0\\n'\n")

            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_ids=["valid/adt-pattern", "valid/argv-program-only"],
            )

            self.assertNotEqual(result.returncode, 0)
            self.assertIn("compile failed", result.stderr.lower())
            self.assertFalse(output.exists())
            self.assertEqual(
                sorted(path.name for path in work_dir.iterdir()),
                ["caller-owned-sentinel"],
            )

    def test_rejects_duplicate_fixture_ids(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            runner = root / "runner"
            wasmtime = root / "wasmtime"
            stage0_manifest = write_stage0_manifest(root)
            work_dir = root / "work"
            work_dir.mkdir()
            output = root / "report.json"
            make_executable(runner, "#!/bin/sh\nexit 0\n")
            make_executable(wasmtime, "#!/bin/sh\nexit 0\n")
            result = self.run_producer(
                root,
                runner,
                wasmtime,
                stage0_manifest,
                output,
                work_dir,
                fixture_ids=["valid/syntax-basic", "valid/syntax-basic"],
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("duplicate", result.stderr.lower())
            self.assertFalse(output.exists())


if __name__ == "__main__":
    raise SystemExit(unittest.main())
