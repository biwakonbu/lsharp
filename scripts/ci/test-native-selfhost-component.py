#!/usr/bin/env python3

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import textwrap
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent.parent
HELPER = SCRIPTS_DIR / "native-selfhost-component.py"


class NativeSelfhostComponentTest(unittest.TestCase):
    def write_executable(self, path, content):
        path.write_text(textwrap.dedent(content), encoding="utf-8")
        os.chmod(path, 0o755)
        return path

    def write_fake_native_program(self, root):
        return self.write_executable(
            root / "program.native",
            f"""\
            #!{sys.executable}
            import json
            import os
            import pathlib
            import sys

            arguments = sys.argv[1:]
            record = {{"arguments": arguments}}
            with pathlib.Path(os.environ["FAKE_NATIVE_LOG"]).open(
                "a", encoding="utf-8"
            ) as log:
                log.write(json.dumps(record) + "\\n")

            if len(arguments) != 4 or arguments[0] not in ("compile", "build") or arguments[2] != "-o":
                sys.stderr.write("unexpected native arguments: " + repr(arguments) + "\\n")
                raise SystemExit(91)

            output = pathlib.Path(arguments[3])
            mode = os.environ.get("FAKE_NATIVE_MODE", "success")
            if mode == "fail":
                output.write_bytes(b"partial-core")
                sys.stderr.write("fake native failure\\n")
                raise SystemExit(17)

            if mode == "invalid-core":
                output.write_bytes(b"not-wasm")
                raise SystemExit(0)

            output.write_bytes(b"\\x00asmfake-core")
            if mode == "warning":
                sys.stderr.write("fake native warning\\n")
            """,
        )

    def write_fake_wasm_tools(self, path):
        return self.write_executable(
            path,
            f"""\
            #!{sys.executable}
            import json
            import os
            import pathlib
            import sys

            arguments = sys.argv[1:]
            record = {{"arguments": arguments}}
            with pathlib.Path(os.environ["FAKE_WASM_TOOLS_LOG"]).open(
                "a", encoding="utf-8"
            ) as log:
                log.write(json.dumps(record) + "\\n")

            if arguments[:1] == ["validate"]:
                if len(arguments) != 2:
                    raise SystemExit(92)
                input_path = pathlib.Path(arguments[1])
                if not input_path.is_file() or not input_path.read_bytes().startswith(b"\\x00asm"):
                    raise SystemExit(93)
                mode = os.environ.get("FAKE_WASM_TOOLS_MODE", "success")
                if mode == "semantic-invalid":
                    sys.stderr.write("fake wasm-tools semantic validation failure\\n")
                    raise SystemExit(24)
                raise SystemExit(0)

            if len(arguments) != 5 or arguments[:2] != ["component", "new"] or arguments[3] != "-o":
                sys.stderr.write("unexpected wasm-tools arguments: " + repr(arguments) + "\\n")
                raise SystemExit(92)

            core = pathlib.Path(arguments[2])
            output = pathlib.Path(arguments[4])
            if not core.is_file() or not core.read_bytes().startswith(b"\\x00asm"):
                sys.stderr.write("missing core Wasm input\\n")
                raise SystemExit(93)

            mode = os.environ.get("FAKE_WASM_TOOLS_MODE", "success")
            if mode == "fail":
                output.write_bytes(b"partial-component")
                sys.stderr.write("fake wasm-tools failure\\n")
                raise SystemExit(23)
            if mode == "directory":
                output.mkdir()
                raise SystemExit(0)

            if mode == "invalid-output":
                output.write_bytes(b"not-wasm")
                raise SystemExit(0)

            output.write_bytes(b"\\x00asmfake-component")
            if mode == "atomic-fail":
                pathlib.Path(os.environ["FAKE_FINAL_OUTPUT"]).mkdir()
            if mode == "warning":
                sys.stderr.write("fake wasm-tools warning\\n")
            """,
        )

    def write_fake_wasmtime(self, path):
        return self.write_executable(
            path,
            f"""\
            #!{sys.executable}
            import json
            import os
            import pathlib
            import sys

            arguments = sys.argv[1:]
            with pathlib.Path(os.environ["FAKE_WASMTIME_LOG"]).open(
                "a", encoding="utf-8"
            ) as log:
                log.write(json.dumps({{"arguments": arguments}}) + "\\n")

            if len(arguments) != 2 or arguments[0] != "run":
                sys.stderr.write("unexpected wasmtime arguments: " + repr(arguments) + "\\n")
                raise SystemExit(94)
            component = pathlib.Path(arguments[1])
            if not component.is_file() or not component.read_bytes().startswith(b"\\x00asm"):
                sys.stderr.write("missing component runtime input\\n")
                raise SystemExit(95)
            if os.environ.get("FAKE_WASMTIME_MODE") == "fail":
                sys.stderr.write("fake component runtime failure\\n")
                raise SystemExit(31)
            """,
        )

    def write_forbidden_command(self, directory, name):
        return self.write_executable(
            directory / name,
            f"""\
            #!{sys.executable}
            import os
            import pathlib

            pathlib.Path(os.environ["FORBIDDEN_LOG"]).write_text(
                {name!r}, encoding="utf-8"
            )
            raise SystemExit(99)
            """,
        )

    def make_environment(self, root):
        tools_directory = root / "tools"
        tools_directory.mkdir()
        for command in ("cargo", "lsharp", "host-launcher"):
            self.write_forbidden_command(tools_directory, command)

        environment = os.environ.copy()
        environment.update(
            {
                "FAKE_NATIVE_LOG": str(root / "native.jsonl"),
                "FAKE_WASM_TOOLS_LOG": str(root / "wasm-tools.jsonl"),
                "FAKE_WASMTIME_LOG": str(root / "wasmtime.jsonl"),
                "FORBIDDEN_LOG": str(root / "forbidden-command-ran"),
                "PATH": str(tools_directory),
            }
        )
        return environment, tools_directory

    def run_helper(
        self,
        program,
        source,
        output,
        environment,
        command="compile",
        wasm_tools=None,
        wasmtime=None,
    ):
        self.assertTrue(HELPER.is_file(), "native selfhost component helper is missing")
        arguments = [
            sys.executable,
            str(HELPER),
            "--program",
            str(program),
            "--command",
            command,
            "--source",
            str(source),
            "--output",
            str(output),
        ]
        if wasm_tools is not None:
            arguments.extend(("--wasm-tools", str(wasm_tools)))
        if wasmtime is not None:
            arguments.extend(("--wasmtime", str(wasmtime)))
        return subprocess.run(
            arguments,
            capture_output=True,
            check=False,
            env=environment,
        )

    def read_records(self, root, name):
        path = root / name
        if not path.exists():
            return []
        return [
            json.loads(line)
            for line in path.read_text(encoding="utf-8").splitlines()
            if line
        ]

    def assert_no_forbidden_command(self, root):
        self.assertFalse(
            (root / "forbidden-command-ran").exists(),
            "helper must not fall back to a Rust host command",
        )

    def assert_successful_invocations(self, root, source, output, command):
        native_records = self.read_records(root, "native.jsonl")
        wasm_tools_records = self.read_records(root, "wasm-tools.jsonl")
        self.assertEqual(len(native_records), 1)
        self.assertEqual(len(wasm_tools_records), 2)

        native_arguments = native_records[0]["arguments"]
        self.assertEqual(
            native_arguments[:3], [command, str(source.resolve()), "-o"]
        )
        self.assertEqual(len(native_arguments), 4)
        core_output = pathlib.Path(native_arguments[3])
        self.assertEqual(core_output.name, "core.wasm")

        wasm_tools_arguments = wasm_tools_records[0]["arguments"]
        self.assertEqual(
            wasm_tools_arguments,
            [
                "component",
                "new",
                str(core_output),
                "-o",
                wasm_tools_arguments[4],
            ],
        )
        temporary_component = pathlib.Path(wasm_tools_arguments[4])
        self.assertEqual(
            wasm_tools_records[1]["arguments"], ["validate", str(temporary_component)]
        )
        self.assertEqual(temporary_component.parent, output.parent)
        self.assertFalse(core_output.exists())
        self.assertFalse(temporary_component.exists())
        self.assertEqual(list(output.parent.glob(f".{output.name}.*.tmp")), [])
        self.assert_no_forbidden_command(root)

    def test_compile_uses_path_wasm_tools_and_replaces_output_after_success(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "dist" / "program.component.wasm"
            output.parent.mkdir()
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(output.read_bytes(), b"\x00asmfake-component")
            self.assert_successful_invocations(root, source, output, "compile")

    def test_build_uses_explicit_wasm_tools_over_path_lookup(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_forbidden_command(tools_directory, "wasm-tools")
            explicit_wasm_tools = self.write_fake_wasm_tools(root / "explicit-wasm-tools")

            result = self.run_helper(
                program,
                source,
                output,
                environment,
                command="build",
                wasm_tools=explicit_wasm_tools,
            )

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(output.read_bytes(), b"\x00asmfake-component")
            self.assert_successful_invocations(root, source, output, "build")

    def test_explicit_wasmtime_runs_component_before_atomic_replace(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            wasmtime = self.write_fake_wasmtime(root / "wasmtime")

            result = self.run_helper(
                program, source, output, environment, wasmtime=wasmtime
            )

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(output.read_bytes(), b"\x00asmfake-component")
            runtime_records = self.read_records(root, "wasmtime.jsonl")
            self.assertEqual(len(runtime_records), 1)
            self.assertEqual(runtime_records[0]["arguments"][0], "run")
            component = pathlib.Path(runtime_records[0]["arguments"][1])
            self.assertFalse(component.exists())
            self.assertFalse(output.is_symlink())
            self.assert_no_forbidden_command(root)

    def test_component_runtime_failure_preserves_existing_output_and_cleans_temporary(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            wasmtime = self.write_fake_wasmtime(root / "wasmtime")
            environment["FAKE_WASMTIME_MODE"] = "fail"

            result = self.run_helper(
                program, source, output, environment, wasmtime=wasmtime
            )

            self.assertEqual(result.returncode, 1)
            self.assertIn(
                b"wasmtime component runtime exited with status 31", result.stderr
            )
            self.assertIn(b"fake component runtime failure", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            runtime_records = self.read_records(root, "wasmtime.jsonl")
            self.assertEqual(len(runtime_records), 1)
            self.assertFalse(pathlib.Path(runtime_records[0]["arguments"][1]).exists())
            self.assertEqual(list(output.parent.glob(f".{output.name}.*.tmp")), [])
            self.assert_no_forbidden_command(root)

    def test_forwards_successful_native_and_wasm_tools_stderr(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_NATIVE_MODE"] = "warning"
            environment["FAKE_WASM_TOOLS_MODE"] = "warning"

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertIn(b"fake native warning", result.stderr)
            self.assertIn(b"fake wasm-tools warning", result.stderr)
            self.assertEqual(output.read_bytes(), b"\x00asmfake-component")
            self.assert_successful_invocations(root, source, output, "compile")

    def test_native_failure_preserves_existing_output_and_cleans_core_temporary(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_NATIVE_MODE"] = "fail"

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(b"native program exited with status 17", result.stderr)
            self.assertIn(b"fake native failure", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            native_records = self.read_records(root, "native.jsonl")
            self.assertEqual(len(native_records), 1)
            self.assertFalse(pathlib.Path(native_records[0]["arguments"][3]).exists())
            self.assertEqual(self.read_records(root, "wasm-tools.jsonl"), [])
            self.assert_no_forbidden_command(root)

    def test_wasm_tools_failure_preserves_existing_output_and_cleans_temporaries(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_WASM_TOOLS_MODE"] = "fail"

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(b"wasm-tools exited with status 23", result.stderr)
            self.assertIn(b"fake wasm-tools failure", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            native_records = self.read_records(root, "native.jsonl")
            wasm_tools_records = self.read_records(root, "wasm-tools.jsonl")
            self.assertEqual(len(native_records), 1)
            self.assertEqual(len(wasm_tools_records), 1)
            self.assertFalse(pathlib.Path(native_records[0]["arguments"][3]).exists())
            self.assertFalse(pathlib.Path(wasm_tools_records[0]["arguments"][4]).exists())
            self.assert_no_forbidden_command(root)

    def test_wasm_tools_directory_output_is_removed_after_validation_failure(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_WASM_TOOLS_MODE"] = "directory"

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(b"wasm-tools did not create component output", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            native_records = self.read_records(root, "native.jsonl")
            wasm_tools_records = self.read_records(root, "wasm-tools.jsonl")
            self.assertEqual(len(native_records), 1)
            self.assertEqual(len(wasm_tools_records), 1)
            self.assertFalse(pathlib.Path(native_records[0]["arguments"][3]).exists())
            self.assertFalse(pathlib.Path(wasm_tools_records[0]["arguments"][4]).exists())
            self.assert_no_forbidden_command(root)

    def test_invalid_native_core_is_rejected_before_wasm_tools_and_replace(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_NATIVE_MODE"] = "invalid-core"

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(b"native program produced invalid Wasm artifact", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            self.assertEqual(len(self.read_records(root, "native.jsonl")), 1)
            self.assertEqual(self.read_records(root, "wasm-tools.jsonl"), [])
            self.assert_no_forbidden_command(root)

    def test_invalid_packaged_component_is_rejected_before_atomic_replace(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_WASM_TOOLS_MODE"] = "invalid-output"

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(b"wasm-tools produced invalid Wasm artifact", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            native_records = self.read_records(root, "native.jsonl")
            wasm_tools_records = self.read_records(root, "wasm-tools.jsonl")
            self.assertEqual(len(native_records), 1)
            self.assertEqual(len(wasm_tools_records), 1)
            self.assertFalse(pathlib.Path(wasm_tools_records[0]["arguments"][4]).exists())
            self.assert_no_forbidden_command(root)

    def test_semantically_invalid_component_is_rejected_before_atomic_replace(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_WASM_TOOLS_MODE"] = "semantic-invalid"

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(
                b"wasm-tools semantic validation exited with status 24", result.stderr
            )
            self.assertIn(b"fake wasm-tools semantic validation failure", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            wasm_tools_records = self.read_records(root, "wasm-tools.jsonl")
            self.assertEqual(len(wasm_tools_records), 2)
            self.assertEqual(wasm_tools_records[1]["arguments"][0], "validate")
            self.assertFalse(pathlib.Path(wasm_tools_records[0]["arguments"][4]).exists())
            self.assertEqual(list(output.parent.glob(f".{output.name}.*.tmp")), [])
            self.assert_no_forbidden_command(root)

    def test_missing_wasm_tools_fails_before_running_native_program(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            output.write_bytes(b"existing-component")
            program = self.write_fake_native_program(root)
            environment, _ = self.make_environment(root)

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(b"wasm-tools was not found on PATH", result.stderr)
            self.assertEqual(output.read_bytes(), b"existing-component")
            self.assertEqual(self.read_records(root, "native.jsonl"), [])
            self.assertEqual(self.read_records(root, "wasm-tools.jsonl"), [])
            self.assert_no_forbidden_command(root)

    def test_rejects_missing_source_and_directory_output_before_command_execution(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            missing_source = root / "missing.ls"
            output = root / "program.component.wasm"

            missing_source_result = self.run_helper(
                program, missing_source, output, environment
            )

            self.assertEqual(missing_source_result.returncode, 1)
            self.assertIn(
                b"source file is not a regular file", missing_source_result.stderr
            )
            self.assertEqual(self.read_records(root, "native.jsonl"), [])

            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output_directory = root / "output-directory"
            output_directory.mkdir()

            output_result = self.run_helper(
                program, source, output_directory, environment
            )

            self.assertEqual(output_result.returncode, 1)
            self.assertIn(b"output path is a directory", output_result.stderr)
            self.assertEqual(self.read_records(root, "native.jsonl"), [])
            self.assert_no_forbidden_command(root)

    def test_atomic_replace_failure_cleans_temporary_component(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\\n", encoding="utf-8")
            output = root / "program.component.wasm"
            program = self.write_fake_native_program(root)
            environment, tools_directory = self.make_environment(root)
            self.write_fake_wasm_tools(tools_directory / "wasm-tools")
            environment["FAKE_WASM_TOOLS_MODE"] = "atomic-fail"
            environment["FAKE_FINAL_OUTPUT"] = str(output)

            result = self.run_helper(program, source, output, environment)

            self.assertEqual(result.returncode, 1)
            self.assertIn(b"failed to atomically replace output", result.stderr)
            self.assertTrue(output.is_dir())
            native_records = self.read_records(root, "native.jsonl")
            wasm_tools_records = self.read_records(root, "wasm-tools.jsonl")
            self.assertEqual(len(native_records), 1)
            self.assertEqual(len(wasm_tools_records), 2)
            self.assertFalse(pathlib.Path(native_records[0]["arguments"][3]).exists())
            self.assertFalse(pathlib.Path(wasm_tools_records[0]["arguments"][4]).exists())
            self.assert_no_forbidden_command(root)


if __name__ == "__main__":
    unittest.main()
