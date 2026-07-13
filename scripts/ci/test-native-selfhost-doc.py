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
HELPER = SCRIPTS_DIR / "native-selfhost-doc.py"

DOCUMENT = {
    "module": "module-<Demo &>",
    "functions": [
        {
            "name": "render<&>",
            "arity": 2,
            "params": [
                {
                    "name": "left<&>",
                    "type": "Int & <T>",
                    "doc": "left <tag> &",
                },
                {
                    "name": "right",
                    "type": "String",
                    "doc": "right > value",
                },
            ],
            "returns": {"type": "String", "doc": "result <strong> &"},
            "doc": "Use <script>alert(1)</script> &",
            "example": "(render \"<x>\" \"&\")",
        }
    ],
    "types": [
        {"name": "Result<Ok>", "kind": "type"},
        {"name": "PublicAlias", "kind": "typealias"},
    ],
    "html": {
        "title": "module-<Demo &>",
        "sections": [
            {"id": "functions", "count": 1},
            {"id": "types", "count": 2},
        ],
    },
}


class NativeSelfhostDocTest(unittest.TestCase):
    def write_fake_program(self, root):
        program = root / "program.native"
        program.write_text(
            textwrap.dedent(
                f"""\
                #!{sys.executable}
                import json
                import os
                import pathlib
                import sys

                pathlib.Path(os.environ["FAKE_NATIVE_LOG"]).write_text(
                    json.dumps(sys.argv[1:]), encoding="utf-8"
                )
                expected = ["doc", os.environ["FAKE_SOURCE"], "--json"]
                if sys.argv[1:] != expected:
                    sys.stderr.write("unexpected native arguments: " + repr(sys.argv[1:]) + "\\n")
                    raise SystemExit(91)

                mode = os.environ.get("FAKE_MODE", "success")
                if mode == "stderr":
                    sys.stdout.write(os.environ["FAKE_DOCUMENT"])
                    sys.stderr.write("native diagnostic\\n")
                    raise SystemExit(0)
                if mode == "nonzero":
                    sys.stderr.write("native failure\\n")
                    raise SystemExit(23)
                if mode == "malformed":
                    sys.stdout.write('{{"module":')
                    raise SystemExit(0)

                sys.stdout.write(os.environ["FAKE_DOCUMENT"])
                """
            ),
            encoding="utf-8",
        )
        os.chmod(program, 0o755)
        return program

    def write_poison_command(self, directory, name):
        command = directory / name
        command.write_text(
            textwrap.dedent(
                f"""\
                #!{sys.executable}
                import os
                import pathlib
                pathlib.Path(os.environ["HOST_COMMAND_RAN"]).write_text(
                    {name!r}, encoding="ascii"
                )
                raise SystemExit(99)
                """
            ),
            encoding="ascii",
        )
        os.chmod(command, 0o755)

    def make_environment(self, root, source, document=None, mode="success"):
        poison_bin = root / "poison-bin"
        poison_bin.mkdir()
        self.write_poison_command(poison_bin, "cargo")
        self.write_poison_command(poison_bin, "lsharp")
        environment = os.environ.copy()
        environment.update(
            {
                "FAKE_DOCUMENT": json.dumps(document or DOCUMENT),
                "FAKE_MODE": mode,
                "FAKE_NATIVE_LOG": str(root / "native-args.json"),
                "FAKE_SOURCE": str(source.resolve()),
                "HOST_COMMAND_RAN": str(root / "host-command-ran"),
                "PATH": str(poison_bin) + os.pathsep + environment["PATH"],
            }
        )
        return environment

    def run_helper(self, program, source, environment, *arguments):
        return subprocess.run(
            [
                sys.executable,
                str(HELPER),
                "--program",
                str(program),
                str(source),
                *arguments,
            ],
            capture_output=True,
            env=environment,
            check=False,
        )

    def assert_native_only(self, root, source):
        self.assertEqual(
            json.loads((root / "native-args.json").read_text(encoding="utf-8")),
            ["doc", str(source.resolve()), "--json"],
        )
        self.assertFalse((root / "host-command-ran").exists())

    def test_renders_html_to_stdout_with_all_metadata_escaped(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn render [left right] left)\n", encoding="utf-8")
            program = self.write_fake_program(root)
            environment = self.make_environment(root, source)

            result = self.run_helper(program, source, environment)

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            html = result.stdout.decode("utf-8")
            self.assertIn("<!DOCTYPE html>", html)
            self.assertIn("<h1>module-&lt;Demo &amp;&gt;</h1>", html)
            self.assertIn("<h2>Functions</h2>", html)
            self.assertIn("render&lt;&amp;&gt;", html)
            self.assertIn("<strong>Arity:</strong> 2", html)
            self.assertIn("left&lt;&amp;&gt;", html)
            self.assertIn("Int &amp; &lt;T&gt;", html)
            self.assertIn("Use &lt;script&gt;alert(1)&lt;/script&gt; &amp;", html)
            self.assertIn("(render &quot;&lt;x&gt;&quot; &quot;&amp;&quot;)", html)
            self.assertIn("<h2>Types</h2>", html)
            self.assertIn("Result&lt;Ok&gt;", html)
            self.assertNotIn("<script>alert(1)</script>", html)
            self.assert_native_only(root, source)

    def test_writes_html_output_and_creates_parent_directories(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\n", encoding="utf-8")
            output = root / "generated" / "nested" / "api.html"
            program = self.write_fake_program(root)
            environment = self.make_environment(root, source)

            result = self.run_helper(program, source, environment, "-o", str(output))

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(
                pathlib.Path(result.stdout.decode("utf-8").strip()).resolve(),
                output.resolve(),
            )
            self.assertTrue(output.is_file())
            self.assertIn("<h1>module-&lt;Demo &amp;&gt;</h1>", output.read_text(encoding="utf-8"))
            self.assertEqual(list(output.parent.glob(".api.html.*.tmp")), [])
            self.assert_native_only(root, source)

    def test_writes_json_to_nearest_project_root_by_default(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            (root / "lsharp.toml").write_text("[package]\nname = 'outer'\n", encoding="utf-8")
            project = root / "project"
            project.mkdir()
            (project / "lsharp.toml").write_text("[package]\nname = 'inner'\n", encoding="utf-8")
            source = project / "src" / "nested" / "input.ls"
            source.parent.mkdir(parents=True)
            source.write_text("(defn main [] 0)\n", encoding="utf-8")
            program = self.write_fake_program(root)
            environment = self.make_environment(root, source)
            output = project / "docs" / "api.json"

            result = self.run_helper(program, source, environment, "--json")

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(
                pathlib.Path(result.stdout.decode("utf-8").strip()).resolve(),
                output.resolve(),
            )
            self.assertEqual(json.loads(output.read_text(encoding="utf-8")), DOCUMENT)
            self.assert_native_only(root, source)

    def test_writes_json_next_to_source_without_project_manifest(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "nested" / "input.ls"
            source.parent.mkdir()
            source.write_text("(defn main [] 0)\n", encoding="utf-8")
            program = self.write_fake_program(root)
            environment = self.make_environment(root, source)
            output = source.parent / "docs" / "api.json"

            result = self.run_helper(program, source, environment, "--json")

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(
                pathlib.Path(result.stdout.decode("utf-8").strip()).resolve(),
                output.resolve(),
            )
            self.assertEqual(json.loads(output.read_text(encoding="utf-8")), DOCUMENT)
            self.assert_native_only(root, source)

    def test_writes_json_to_explicit_output_with_format_alias(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\n", encoding="utf-8")
            output = root / "manual" / "nested" / "api.json"
            program = self.write_fake_program(root)
            environment = self.make_environment(root, source)

            result = self.run_helper(
                program, source, environment, "--output", str(output), "--format", "json"
            )

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(result.stdout.decode("utf-8"), f"{output}\n")
            self.assertEqual(json.loads(output.read_text(encoding="utf-8")), DOCUMENT)
            self.assert_native_only(root, source)

    def test_rejects_native_stderr_nonzero_and_malformed_json(self):
        cases = (
            ("stderr", b"native program wrote to stderr", b"native diagnostic"),
            ("nonzero", b"native program exited with status 23", b"native failure"),
            ("malformed", b"malformed native JSON", b""),
        )
        for mode, message, detail in cases:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary_directory:
                root = pathlib.Path(temporary_directory)
                source = root / "input.ls"
                source.write_text("(defn main [] 0)\n", encoding="utf-8")
                program = self.write_fake_program(root)
                environment = self.make_environment(root, source, mode=mode)

                result = self.run_helper(program, source, environment, "--json")

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)
                if detail:
                    self.assertIn(detail, result.stderr)
                self.assertEqual(result.stdout, b"")
                self.assertFalse((root / "docs" / "api.json").exists())

    def test_rejects_schema_violations_before_writing_output(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            source = root / "input.ls"
            source.write_text("(defn main [] 0)\n", encoding="utf-8")
            invalid_document = json.loads(json.dumps(DOCUMENT))
            invalid_document["functions"][0]["unexpected"] = "not allowed"
            program = self.write_fake_program(root)
            environment = self.make_environment(root, source, document=invalid_document)
            output = root / "invalid" / "api.json"

            result = self.run_helper(program, source, environment, "--json", "-o", str(output))

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"native JSON schema violation", result.stderr)
            self.assertFalse(output.exists())

    def test_rejects_missing_native_metadata_and_negative_values_before_writing_output(self):
        cases = (
            (
                "missing-function-doc",
                lambda document: document["functions"][0].pop("doc"),
                b"missing required keys: doc",
            ),
            (
                "missing-function-example",
                lambda document: document["functions"][0].pop("example"),
                b"missing required keys: example",
            ),
            (
                "missing-param-doc",
                lambda document: document["functions"][0]["params"][0].pop("doc"),
                b"missing required keys: doc",
            ),
            (
                "missing-returns-doc",
                lambda document: document["functions"][0]["returns"].pop("doc"),
                b"missing required keys: doc",
            ),
            (
                "negative-arity",
                lambda document: document["functions"][0].__setitem__("arity", -1),
                b"expected non-negative integer",
            ),
            (
                "negative-section-count",
                lambda document: document["html"]["sections"][0].__setitem__("count", -1),
                b"expected non-negative integer",
            ),
        )
        for name, mutate, message in cases:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary_directory:
                root = pathlib.Path(temporary_directory)
                source = root / "input.ls"
                source.write_text("(defn main [] 0)\n", encoding="utf-8")
                invalid_document = json.loads(json.dumps(DOCUMENT))
                mutate(invalid_document)
                program = self.write_fake_program(root)
                environment = self.make_environment(root, source, document=invalid_document)
                output = root / "invalid" / f"{name}.json"

                result = self.run_helper(program, source, environment, "--json", "-o", str(output))

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(b"native JSON schema violation", result.stderr)
                self.assertIn(message, result.stderr)
                self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
