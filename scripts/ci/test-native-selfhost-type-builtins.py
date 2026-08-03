#!/usr/bin/env python3
"""Check the native built-in type environment through CLI and LSP surfaces."""

import argparse
import json
import pathlib
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
LSP_SHIM = ROOT / "scripts/native-selfhost-lsp-stdio.py"
VALID_SOURCE = "(defn main [] (+ 1 2))\n"
INVALID_SOURCE = "(defn bad [] (+ 1 true))\n"
VALID_BUILTIN_SOURCES = {
    "subtraction": "(defn main [] (- 3 1))\n",
    "multiplication": "(defn main [] (* 3 2))\n",
    "division": "(defn main [] (/ 6 2))\n",
    "remainder": "(defn main [] (% 7 2))\n",
    "comparison": "(defn main [] (= 3 3))\n",
    "string-length": "(defn main [] (string-length \"abc\"))\n",
    "string-concat": "(defn main [] (string-concat \"a\" \"b\"))\n",
    "vector": "(defn main [] (vector-length (vector-new 2)))\n",
    "map": "(defn main [] (map-size (map-new)))\n",
    "reference": "(defn main [] (ref-get (ref-new 3)))\n",
}
INVALID_BUILTIN_SOURCES = {
    "subtraction": "(defn bad [] (- 3 true))\n",
    "string-length": "(defn bad [] (string-length 3))\n",
    "vector-length": "(defn bad [] (vector-length 3))\n",
}
URI = "file:///tmp/lsharp-native-type-builtins.ls"


def frame(body):
    encoded = json.dumps(body, separators=(",", ":")).encode("utf-8")
    return b"Content-Length: " + str(len(encoded)).encode("ascii") + b"\r\n\r\n" + encoded


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
        frames.append(json.loads(data[body_start:body_end]))
        offset = body_end
    return frames


def run_check(program, source):
    with tempfile.TemporaryDirectory(prefix="lsharp-native-type-builtins-") as directory:
        path = pathlib.Path(directory) / "input.ls"
        path.write_text(source, encoding="utf-8")
        return subprocess.run(
            [str(program), "check", str(path)],
            capture_output=True,
            check=False,
        )


def run_lsp(program):
    requests = b"".join(
        (
            frame({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            frame(
                {
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": URI,
                            "languageId": "lsharp",
                            "version": 1,
                            "text": INVALID_SOURCE.rstrip("\n"),
                        }
                    },
                }
            ),
        )
    )
    result = subprocess.run(
        ["python3", str(LSP_SHIM), "--program", str(program)],
        input=requests,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(result.stderr.decode("utf-8", "replace"))
    return parse_frames(result.stdout)


class NativeSelfhostTypeBuiltinsTest(unittest.TestCase):
    def test_valid_plus_is_resolved_by_native_check(self):
        result = run_check(self.program, VALID_SOURCE)
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertIn(b"diagnostics:0", result.stdout)

    def test_builtin_type_environment_resolves_numeric_string_and_container_families(self):
        for name, source in VALID_BUILTIN_SOURCES.items():
            with self.subTest(name=name):
                result = run_check(self.program, source)
                self.assertEqual(result.returncode, 0, result.stderr.decode())
                self.assertIn(b"diagnostics:0", result.stdout)

    def test_builtin_type_environment_reports_argument_mismatch_across_families(self):
        for name, source in INVALID_BUILTIN_SOURCES.items():
            with self.subTest(name=name):
                result = run_check(self.program, source)
                self.assertEqual(result.returncode, 1, result.stderr.decode())
                self.assertIn(b"function argument type mismatch", result.stdout)

    def test_invalid_plus_reports_argument_mismatch(self):
        result = run_check(self.program, INVALID_SOURCE)
        self.assertEqual(result.returncode, 1, result.stderr.decode())
        self.assertIn(b"diagnostics:1,T0001@1:1,first-body:function argument type mismatch", result.stdout)

    def test_invalid_plus_uses_standard_lsp_diagnostic(self):
        diagnostics = [
            frame_body
            for frame_body in run_lsp(self.program)
            if frame_body.get("method") == "textDocument/publishDiagnostics"
        ]
        self.assertEqual(len(diagnostics), 1)
        self.assertEqual(
            diagnostics[0],
            {
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": {
                    "uri": URI,
                    "diagnostics": [
                        {
                            "range": {
                                "start": {"line": 0, "character": 0},
                                "end": {"line": 0, "character": 0},
                            },
                            "severity": 1,
                            "code": "LS1004",
                            "source": "lsharp",
                            "message": "function argument type mismatch",
                        }
                    ],
                },
            },
        )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--program", required=True, type=pathlib.Path)
    args = parser.parse_args()
    if not args.program.is_file() or not args.program.stat().st_mode & 0o111:
        parser.error(f"native program is not executable: {args.program}")
    NativeSelfhostTypeBuiltinsTest.program = args.program.resolve()
    return unittest.main(argv=[__file__], exit=False).result.wasSuccessful()


if __name__ == "__main__":
    raise SystemExit(0 if main() else 1)
