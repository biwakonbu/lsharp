#!/usr/bin/env python3

import os
import pathlib
import subprocess
import sys
import tarfile
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent
PRODUCER = SCRIPTS_DIR / "create-native-release-input-bundle.py"
ALLOWED_NAMES = (
    "program.native",
    "manifest.json",
    "smoke-stdout.txt",
    "smoke-stderr.txt",
)


class NativeReleaseInputBundleTest(unittest.TestCase):
    def write_inputs(self, root):
        program = root / "program.native"
        program.write_bytes(b"native-program\n")
        os.chmod(program, 0o755)

        manifest = root / "manifest.json"
        manifest.write_text('{"status":"pass"}\n', encoding="utf-8")

        smoke_stdout = root / "actual-stage3-target-smoke-stdout.txt"
        smoke_stdout.write_text("lsharp 0.1.0\n", encoding="utf-8")

        smoke_stderr = root / "actual-stage3-target-smoke-stderr.txt"
        smoke_stderr.write_text("", encoding="utf-8")

        return {
            "program": program,
            "manifest": manifest,
            "smoke_stdout": smoke_stdout,
            "smoke_stderr": smoke_stderr,
        }

    def run_producer(self, output, inputs):
        return subprocess.run(
            [
                sys.executable,
                str(PRODUCER),
                "--output",
                str(output),
                "--program",
                str(inputs["program"]),
                "--manifest",
                str(inputs["manifest"]),
                "--smoke-stdout",
                str(inputs["smoke_stdout"]),
                "--smoke-stderr",
                str(inputs["smoke_stderr"]),
            ],
            capture_output=True,
            text=True,
            check=False,
        )

    def test_creates_exact_canonical_bundle_without_appledouble(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            inputs = self.write_inputs(root)
            (root / "._program.native").write_bytes(b"appledouble")
            output = root / "native-input-bundle.tar.gz"

            result = self.run_producer(output, inputs)

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(output.is_file())
            with tarfile.open(output, "r:gz") as bundle:
                members = bundle.getmembers()
                self.assertEqual([member.name for member in members], list(ALLOWED_NAMES))
                self.assertTrue(all(member.isfile() for member in members))
                self.assertNotIn("._program.native", [member.name for member in members])
                self.assertEqual(bundle.getmember("program.native").mode, 0o755)
                self.assertEqual(
                    bundle.extractfile("smoke-stdout.txt").read(),
                    b"lsharp 0.1.0\n",
                )
                self.assertEqual(bundle.extractfile("smoke-stderr.txt").read(), b"")

    def test_rejects_non_regular_inputs(self):
        for input_name in ("program", "manifest", "smoke_stdout", "smoke_stderr"):
            with self.subTest(input_name=input_name), tempfile.TemporaryDirectory() as temporary_directory:
                root = pathlib.Path(temporary_directory)
                inputs = self.write_inputs(root)
                invalid_input = root / "not-a-regular-file"
                invalid_input.mkdir()
                inputs[input_name] = invalid_input
                output = root / "native-input-bundle.tar.gz"

                result = self.run_producer(output, inputs)

                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output.exists())

    def test_rejects_empty_program_and_manifest(self):
        for input_name in ("program", "manifest"):
            with self.subTest(input_name=input_name), tempfile.TemporaryDirectory() as temporary_directory:
                root = pathlib.Path(temporary_directory)
                inputs = self.write_inputs(root)
                inputs[input_name].write_bytes(b"")
                output = root / "native-input-bundle.tar.gz"

                result = self.run_producer(output, inputs)

                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main(verbosity=2)
