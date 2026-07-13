#!/usr/bin/env python3

import os
import pathlib
import subprocess
import sys
import tempfile
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent.parent
SHIM = SCRIPTS_DIR / "native-selfhost-lsp-stdio.py"


def frame(body):
    return b"Content-Length: " + str(len(body)).encode("ascii") + b"\r\n\r\n" + body


def parse_frames(data):
    frames = []
    offset = 0
    while offset < len(data):
        header_end = data.find(b"\r\n\r\n", offset)
        if header_end < 0:
            raise AssertionError("missing LSP header terminator")
        header = data[offset:header_end]
        content_length = None
        for line in header.split(b"\r\n"):
            name, value = line.split(b":", 1)
            if name.lower() == b"content-length":
                content_length = int(value.strip())
        if content_length is None:
            raise AssertionError("missing Content-Length")
        body_start = header_end + 4
        body_end = body_start + content_length
        if body_end > len(data):
            raise AssertionError("truncated LSP body")
        frames.append(data[offset:body_end])
        offset = body_end
    return frames


class NativeSelfhostLspStdioTest(unittest.TestCase):
    def write_fake_program(self, root):
        program = root / "fake-app-cli.py"
        program.write_text(
            """#!/usr/bin/env python3
import os
import pathlib
import sys


def frame(body):
    return b"Content-Length: " + str(len(body)).encode("ascii") + b"\\r\\n\\r\\n" + body


def count_frames(data):
    count = 0
    offset = 0
    while offset < len(data):
        header_end = data.index(b"\\r\\n\\r\\n", offset)
        length = None
        for line in data[offset:header_end].split(b"\\r\\n"):
            name, value = line.split(b":", 1)
            if name.lower() == b"content-length":
                length = int(value.strip())
        if length is None:
            raise SystemExit(41)
        offset = header_end + 4 + length
        count += 1
    return count


record_dir = pathlib.Path(os.environ["FAKE_RECORD_DIR"])
call_index = len(list(record_dir.glob("call-*.bin"))) + 1
data = sys.stdin.buffer.read()
(record_dir / f"call-{call_index}.bin").write_bytes(data)
(record_dir / f"args-{call_index}.bin").write_bytes(b"\\0".join(arg.encode() for arg in sys.argv[1:]))
mode = os.environ.get("FAKE_MODE", "success")
if mode == "stderr":
    sys.stdout.buffer.write(frame(b"ignored"))
    sys.stderr.buffer.write(b"child diagnostic\\n")
    raise SystemExit(0)
if mode == "nonzero":
    sys.stdout.buffer.write(frame(b"ignored"))
    sys.stderr.buffer.write(b"child failure\\n")
    raise SystemExit(23)
if mode == "malformed-output":
    sys.stdout.buffer.write(b"Content-Length: 8\\r\\n\\r\\nshort")
    raise SystemExit(0)
if mode == "regression" and count_frames(data) > 1:
    raise SystemExit(0)
for index in range(1, count_frames(data) + 1):
    sys.stdout.buffer.write(frame(f"response-{index}".encode("ascii")))
""",
            encoding="ascii",
        )
        os.chmod(program, 0o755)
        return program

    def write_poison_command(self, directory, name):
        command = directory / name
        command.write_text(
            """#!/usr/bin/env python3
import os
import pathlib
pathlib.Path(os.environ["HOST_COMMAND_RAN"]).touch()
raise SystemExit(99)
""",
            encoding="ascii",
        )
        os.chmod(command, 0o755)

    def run_shim(self, program, data, environment, extra_args=()):
        return subprocess.run(
            [
                sys.executable,
                str(SHIM),
                "--program",
                str(program),
                "--",
                *extra_args,
            ],
            input=data,
            capture_output=True,
            env=environment,
            check=False,
        )

    def test_replays_aggregate_and_forwards_only_new_frames(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            records = root / "records"
            records.mkdir()
            program = self.write_fake_program(root)
            poison_bin = root / "poison-bin"
            poison_bin.mkdir()
            self.write_poison_command(poison_bin, "cargo")
            self.write_poison_command(poison_bin, "lsharp")
            environment = os.environ.copy()
            environment.update(
                {
                    "FAKE_RECORD_DIR": str(records),
                    "HOST_COMMAND_RAN": str(root / "host-command-ran"),
                    "PATH": str(poison_bin) + os.pathsep + environment["PATH"],
                }
            )
            first = frame(b'{"jsonrpc":"2.0","id":1,"method":"initialize"}')
            second = frame(
                b'{"jsonrpc":"2.0","id":2,"method":"workspace/didChange"}'
            )

            result = self.run_shim(
                program, first + second, environment, extra_args=("--fake-flag",)
            )

            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
            self.assertEqual(
                parse_frames(result.stdout),
                [frame(b"response-1"), frame(b"response-2")],
            )
            self.assertEqual((records / "call-1.bin").read_bytes(), first)
            self.assertEqual((records / "call-2.bin").read_bytes(), first + second)
            self.assertEqual(
                (records / "args-1.bin").read_bytes().split(b"\0"),
                [b"lsp", b"--stdio", b"--fake-flag"],
            )
            self.assertEqual(
                (records / "args-2.bin").read_bytes().split(b"\0"),
                [b"lsp", b"--stdio", b"--fake-flag"],
            )
            self.assertFalse((root / "host-command-ran").exists())

    def test_rejects_malformed_or_truncated_input_without_output(self):
        for data, message in (
            (b"Content-Length: seven\r\n\r\n", b"invalid Content-Length"),
            (b"Content-Length: 7\r\n\r\nshort", b"truncated inbound frame"),
        ):
            with self.subTest(data=data), tempfile.TemporaryDirectory() as temporary_directory:
                root = pathlib.Path(temporary_directory)
                records = root / "records"
                records.mkdir()
                program = self.write_fake_program(root)
                environment = os.environ.copy()
                environment["FAKE_RECORD_DIR"] = str(records)

                result = self.run_shim(program, data, environment)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)
                self.assertEqual(result.stdout, b"")
                self.assertEqual(list(records.glob("call-*.bin")), [])

    def test_rejects_child_stderr_nonzero_and_malformed_output(self):
        for mode, message in (
            ("stderr", b"child diagnostic"),
            ("nonzero", b"child failure"),
            ("malformed-output", b"malformed child output"),
        ):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary_directory:
                root = pathlib.Path(temporary_directory)
                records = root / "records"
                records.mkdir()
                program = self.write_fake_program(root)
                environment = os.environ.copy()
                environment.update({"FAKE_RECORD_DIR": str(records), "FAKE_MODE": mode})

                result = self.run_shim(program, frame(b"{}"), environment)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)
                self.assertEqual(result.stdout, b"")

    def test_rejects_a_response_frame_count_regression(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            records = root / "records"
            records.mkdir()
            program = self.write_fake_program(root)
            environment = os.environ.copy()
            environment.update({"FAKE_RECORD_DIR": str(records), "FAKE_MODE": "regression"})

            result = self.run_shim(program, frame(b"{}") + frame(b"[]"), environment)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"response frame count regressed", result.stderr)
            self.assertEqual(parse_frames(result.stdout), [frame(b"response-1")])

    def test_rejects_a_non_executable_program(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = root / "not-executable"
            program.write_text("not executable\n", encoding="ascii")
            environment = os.environ.copy()

            result = self.run_shim(program, frame(b"{}"), environment)

            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b"not executable", result.stderr)
            self.assertEqual(result.stdout, b"")


if __name__ == "__main__":
    unittest.main(verbosity=2)
