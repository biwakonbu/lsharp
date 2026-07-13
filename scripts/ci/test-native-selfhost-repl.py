#!/usr/bin/env python3

import json
import os
import pathlib
import pty
import select
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent.parent
REPL = SCRIPTS_DIR / "native-selfhost-repl.py"


class NativeSelfhostReplTests(unittest.TestCase):
    def setUp(self):
        temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(temporary_directory.cleanup)
        self.root = pathlib.Path(temporary_directory.name)
        self.bin_dir = self.root / "bin"
        self.bin_dir.mkdir()
        self.log_path = self.root / "commands.jsonl"
        self.forbidden_log_path = self.root / "forbidden.log"
        self.forbidden_log_path.write_text("", encoding="utf-8")
        self.program = self.root / "program.native"
        self.write_fake_program(self.program)
        self.write_fake_wasmtime(self.bin_dir / "wasmtime")
        self.write_forbidden_command(self.bin_dir / "cargo")
        self.write_forbidden_command(self.bin_dir / "lsharp")
        self.environment = os.environ.copy()
        self.environment.update(
            {
                "NATIVE_REPL_LOG": str(self.log_path),
                "NATIVE_REPL_FORBIDDEN_LOG": str(self.forbidden_log_path),
                "PATH": str(self.bin_dir)
                + os.pathsep
                + self.environment.get("PATH", ""),
            }
        )

    def write_executable(self, path, source):
        path.write_text(source, encoding="utf-8")
        path.chmod(0o755)

    def write_fake_program(self, path):
        self.write_executable(
            path,
            """#!__PYTHON__
import json
import os
import pathlib
import sys


def record(kind, **values):
    payload = {"kind": kind}
    payload.update(values)
    with open(os.environ["NATIVE_REPL_LOG"], "a", encoding="utf-8") as output:
        output.write(json.dumps(payload) + "\\n")


arguments = sys.argv[1:]
if len(arguments) != 4 or arguments[0] != "compile" or arguments[2] != "-o":
    print("fake native program received unexpected arguments", file=sys.stderr)
    raise SystemExit(99)

source_path = pathlib.Path(arguments[1])
wasm_path = pathlib.Path(arguments[3])
source_text = source_path.read_text(encoding="utf-8")
record(
    "compile",
    arguments=arguments,
    source=str(source_path),
    source_text=source_text,
    wasm=str(wasm_path),
)
if "compile-fail" in source_text:
    print("fake native compile failure", file=sys.stderr)
    raise SystemExit(17)

wasm_path.write_text(source_text, encoding="utf-8")
print("fake native compile output")
""".replace("__PYTHON__", sys.executable),
        )

    def write_fake_wasmtime(self, path):
        self.write_executable(
            path,
            """#!__PYTHON__
import json
import os
import pathlib
import sys


def record(kind, **values):
    payload = {"kind": kind}
    payload.update(values)
    with open(os.environ["NATIVE_REPL_LOG"], "a", encoding="utf-8") as output:
        output.write(json.dumps(payload) + "\\n")


arguments = sys.argv[1:]
if len(arguments) != 1:
    print("fake wasmtime received unexpected arguments", file=sys.stderr)
    raise SystemExit(98)

wasm_path = pathlib.Path(arguments[0])
source_text = wasm_path.read_text(encoding="utf-8")
record(
    "runtime",
    arguments=arguments,
    executable=str(pathlib.Path(sys.argv[0]).resolve()),
    source_text=source_text,
    wasm=str(wasm_path),
)
if "runtime-fail" in source_text:
    print("fake wasmtime runtime failure", file=sys.stderr)
    raise SystemExit(23)

print("fake wasmtime output: " + source_text)
""".replace("__PYTHON__", sys.executable),
        )

    def write_forbidden_command(self, path):
        self.write_executable(
            path,
            """#!/bin/sh
printf '%s\\n' "$0 $*" >> "$NATIVE_REPL_FORBIDDEN_LOG"
exit 97
""",
        )

    def run_repl(self, standard_input, wasmtime=None, environment=None):
        self.assertTrue(REPL.is_file(), "native selfhost REPL runner is missing")
        command = [sys.executable, str(REPL), "--program", str(self.program)]
        if wasmtime is not None:
            command.extend(["--wasmtime", str(wasmtime)])
        command.append("--stdin")
        return subprocess.run(
            command,
            input=standard_input,
            text=True,
            capture_output=True,
            check=False,
            env=environment or self.environment,
        )

    def records(self, kind):
        if not self.log_path.exists():
            return []
        return [
            json.loads(line)
            for line in self.log_path.read_text(encoding="utf-8").splitlines()
            if json.loads(line)["kind"] == kind
        ]

    def assert_no_forbidden_commands(self):
        self.assertEqual(self.forbidden_log_path.read_text(encoding="utf-8"), "")

    def test_stdin_evaluates_nonempty_lines_uses_path_wasmtime_and_cleans_artifacts(self):
        result = self.run_repl("(print (+ 1 2))\n\n(print 9)\n")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertNotIn("lsharp> ", result.stdout)
        self.assertEqual(result.stdout.count("fake native compile output"), 2)
        self.assertIn("fake wasmtime output: (defn main [] (print (+ 1 2)))", result.stdout)
        self.assertIn("fake wasmtime output: (defn main [] (print 9))", result.stdout)

        compile_records = self.records("compile")
        runtime_records = self.records("runtime")
        self.assertEqual(
            [record["source_text"] for record in compile_records],
            [
                "(defn main [] (print (+ 1 2)))",
                "(defn main [] (print 9))",
            ],
        )
        self.assertEqual(
            [record["source_text"] for record in runtime_records],
            [
                "(defn main [] (print (+ 1 2)))",
                "(defn main [] (print 9))",
            ],
        )
        self.assertEqual(len({record["wasm"] for record in compile_records}), 2)
        for record in compile_records:
            self.assertFalse(pathlib.Path(record["source"]).exists())
            self.assertFalse(pathlib.Path(record["wasm"]).exists())
        self.assert_no_forbidden_commands()

    def test_compile_failure_does_not_run_stale_wasm_and_keeps_later_line_failed(self):
        result = self.run_repl("(compile-fail)\n(print 7)\n")

        self.assertEqual(result.returncode, 1)
        self.assertIn("fake native compile failure", result.stderr)
        self.assertIn("compile failed with exit code 17", result.stderr)
        self.assertEqual(result.stdout.count("fake native compile output"), 1)
        self.assertIn("fake wasmtime output: (defn main [] (print 7))", result.stdout)

        compile_records = self.records("compile")
        runtime_records = self.records("runtime")
        self.assertEqual(len(compile_records), 2)
        self.assertEqual(len(runtime_records), 1)
        self.assertEqual(runtime_records[0]["wasm"], compile_records[1]["wasm"])
        self.assertNotEqual(compile_records[0]["wasm"], compile_records[1]["wasm"])
        self.assertEqual(
            runtime_records[0]["source_text"], "(defn main [] (print 7))"
        )
        self.assert_no_forbidden_commands()

    def test_runtime_failure_is_reported_and_preserves_failure_exit_status(self):
        result = self.run_repl("(runtime-fail)\n(print 8)\n")

        self.assertEqual(result.returncode, 1)
        self.assertIn("fake wasmtime runtime failure", result.stderr)
        self.assertIn("runtime failed with exit code 23", result.stderr)
        self.assertEqual(result.stdout.count("fake native compile output"), 2)
        self.assertIn("fake wasmtime output: (defn main [] (print 8))", result.stdout)

        runtime_records = self.records("runtime")
        self.assertEqual(len(runtime_records), 2)
        self.assertEqual(
            [record["source_text"] for record in runtime_records],
            ["(defn main [] (runtime-fail))", "(defn main [] (print 8))"],
        )
        self.assert_no_forbidden_commands()

    def test_quit_stops_stdin_before_following_expression(self):
        result = self.run_repl(":quit\n(print 999)\n")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(result.stderr, "")
        self.assertEqual(self.records("compile"), [])
        self.assertEqual(self.records("runtime"), [])
        self.assert_no_forbidden_commands()

    def test_interactive_tty_shows_prompt_and_quit_stops_before_compiling(self):
        master_fd, slave_fd = pty.openpty()
        process = None
        try:
            process = subprocess.Popen(
                [sys.executable, str(REPL), "--program", str(self.program)],
                stdin=slave_fd,
                stdout=slave_fd,
                stderr=subprocess.PIPE,
                env=self.environment,
                close_fds=True,
            )
        finally:
            os.close(slave_fd)

        try:
            readable, _, _ = select.select([master_fd], [], [], 5)
            self.assertTrue(readable, "interactive runner did not write a prompt")
            prompt = os.read(master_fd, 1024).decode("utf-8")
            self.assertIn("lsharp> ", prompt)
            os.write(master_fd, b":quit\n")
            self.assertEqual(process.wait(timeout=5), 0)
            self.assertEqual(process.stderr.read().decode("utf-8"), "")
        finally:
            if process is not None:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)
                if process.stderr is not None:
                    process.stderr.close()
            os.close(master_fd)

        self.assertEqual(self.records("compile"), [])
        self.assertEqual(self.records("runtime"), [])
        self.assert_no_forbidden_commands()

    def test_explicit_wasmtime_path_overrides_path_lookup(self):
        explicit_wasmtime = self.root / "explicit-wasmtime"
        self.write_fake_wasmtime(explicit_wasmtime)

        result = self.run_repl("(print 42)\n", wasmtime=explicit_wasmtime)

        self.assertEqual(result.returncode, 0, result.stderr)
        runtime_records = self.records("runtime")
        self.assertEqual(len(runtime_records), 1)
        self.assertEqual(
            runtime_records[0]["executable"], str(explicit_wasmtime.resolve())
        )
        self.assert_no_forbidden_commands()

    def test_requires_program_option(self):
        self.assertTrue(REPL.is_file(), "native selfhost REPL runner is missing")

        result = subprocess.run(
            [sys.executable, str(REPL), "--stdin"],
            input="(print 1)\n",
            text=True,
            capture_output=True,
            check=False,
            env=self.environment,
        )

        self.assertEqual(result.returncode, 2)
        self.assertIn("the following arguments are required: --program", result.stderr)
        self.assert_no_forbidden_commands()

    def test_rejects_missing_path_wasmtime_without_host_fallback(self):
        self.assertTrue(REPL.is_file(), "native selfhost REPL runner is missing")
        no_wasmtime_dir = self.root / "no-wasmtime-bin"
        no_wasmtime_dir.mkdir()
        self.write_forbidden_command(no_wasmtime_dir / "cargo")
        self.write_forbidden_command(no_wasmtime_dir / "lsharp")
        environment = self.environment.copy()
        environment["PATH"] = str(no_wasmtime_dir)

        result = self.run_repl("(print 1)\n", environment=environment)

        self.assertEqual(result.returncode, 2)
        self.assertIn("wasmtime was not found in PATH", result.stderr)
        self.assertEqual(self.records("compile"), [])
        self.assertEqual(self.records("runtime"), [])
        self.assert_no_forbidden_commands()


if __name__ == "__main__":
    unittest.main()
