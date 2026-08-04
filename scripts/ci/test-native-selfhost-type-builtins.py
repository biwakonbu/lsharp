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
    "float-add": "(defn main [] (+. 1.0 2.0))\n",
    "float-subtraction": "(defn main [] (-. 3.0 1.0))\n",
    "float-multiplication": "(defn main [] (*. 2.0 4.0))\n",
    "float-division": "(defn main [] (/. 8.0 2.0))\n",
    "print-polymorphic": '(defn main [] (do (print 1) (print "x") 0))\n',
    "polymorphic-collection-reuse": '(defn main [] (let [ints (vector-push (vector-new 1) 7) strings (vector-push (vector-new 1) "x") int-map (map-insert (map-new) 1 "one") string-map (map-insert (map-new) "two" 2)] (do (vector-get ints 0) (vector-get strings 0) (map-get int-map 1) (map-get string-map "two") 0)))\n',
    "subtraction": "(defn main [] (- 3 1))\n",
    "multiplication": "(defn main [] (* 3 2))\n",
    "division": "(defn main [] (/ 6 2))\n",
    "remainder": "(defn main [] (% 7 2))\n",
    "comparison": "(defn main [] (= 3 3))\n",
    "comparison-gt": "(defn main [] (> 3 2))\n",
    "comparison-lt": "(defn main [] (< 1 2))\n",
    "comparison-le": "(defn main [] (<= 1 2))\n",
    "comparison-ge": "(defn main [] (>= 2 1))\n",
    "comparison-eqeq": "(defn main [] (== 3 3))\n",
    "comparison-neq": "(defn main [] (!= 3 4))\n",
    "logic-not": "(defn main [] (not false))\n",
    "logic-and": "(defn main [] (and true false))\n",
    "logic-or": "(defn main [] (or false true))\n",
    "string-length": "(defn main [] (string-length \"abc\"))\n",
    "string-concat": "(defn main [] (string-concat \"a\" \"b\"))\n",
    "string-eq": "(defn main [] (string-eq \"a\" \"a\"))\n",
    "string-char-at": "(defn main [] (string-char-at \"abc\" 0))\n",
    "substring": "(defn main [] (substring \"abc\" 0 1))\n",
    "int-to-string": "(defn main [] (int-to-string 3))\n",
    "vector": "(defn main [] (vector-length (vector-new 2)))\n",
    "vector-get": "(defn main [] (vector-get (vector-push (vector-new 2) 7) 0))\n",
    "vector-set": "(defn main [] (vector-length (vector-set (vector-new 2) 0 7)))\n",
    "vector-push": "(defn main [] (vector-length (vector-push (vector-new 2) 7)))\n",
    "map": "(defn main [] (map-size (map-new)))\n",
    "map-insert": "(defn main [] (map-size (map-insert (map-new) 1 2)))\n",
    "map-get": "(defn main [] (map-get (map-insert (map-new) 1 2) 1))\n",
    "map-contains": "(defn main [] (map-contains? (map-insert (map-new) 1 2) 1))\n",
    "map-remove": "(defn main [] (map-size (map-remove (map-insert (map-new) 1 2) 1)))\n",
    "reference": "(defn main [] (ref-get (ref-new 3)))\n",
    "reference-set": '(defn main [] (let [r (ref-new "x")] (ref-set r "y")))\n',
    "command-line-args": "(defn main [] (command-line-args))\n",
    "command-line-arg": "(defn main [] (command-line-arg 0))\n",
    "read-stdin": "(defn main [] (read-stdin))\n",
    "file-exists?": "(defn main [] (file-exists? \"input.ls\"))\n",
    "read-file": "(defn main [] (read-file \"input.ls\"))\n",
    "write-file-bytes": "(defn main [] (write-file-bytes \"raw.wasm\" (vector-new 2)))\n",
}
INVALID_BUILTIN_SOURCES = {
    "float-add": "(defn bad [] (+. 1.0 2))\n",
    "float-subtraction": "(defn bad [] (-. 3.0 true))\n",
    "float-multiplication": '(defn bad [] (*. 2.0 "x"))\n',
    "float-division": "(defn bad [] (/. false 2.0))\n",
    "subtraction": "(defn bad [] (- 3 true))\n",
    "string-concat-arg": '(defn bad [] (string-concat "a" true))\n',
    "string-eq-left": '(defn bad [] (string-eq 1 "a"))\n',
    "string-length": "(defn bad [] (string-length 3))\n",
    "string-char-at": "(defn bad [] (string-char-at 3 0))\n",
    "string-char-at-index": '(defn bad [] (string-char-at "abc" true))\n',
    "substring-end": '(defn bad [] (substring "abc" 0 true))\n',
    "vector-new-size": "(defn bad [] (vector-new true))\n",
    "vector-set-receiver": "(defn bad [] (vector-set true 0 1))\n",
    "vector-push-receiver": "(defn bad [] (vector-push true 1))\n",
    "vector-length": "(defn bad [] (vector-length 3))\n",
    "vector-get": "(defn bad [] (vector-get 3 0))\n",
    "map-insert": "(defn bad [] (map-insert 3 1 2))\n",
    "map-get-receiver": "(defn bad [] (map-get true 1))\n",
    "map-contains-receiver": "(defn bad [] (map-contains? true 1))\n",
    "map-remove-receiver": "(defn bad [] (map-remove true 1))\n",
    "reference-get-receiver": "(defn bad [] (ref-get true))\n",
    "reference-set": '(defn bad [] (let [r (ref-new "x")] (ref-set r 1)))\n',
    "command-line-arg": "(defn bad [] (command-line-arg true))\n",
    "file-exists?": "(defn bad [] (file-exists? 3))\n",
    "comparison-gt": "(defn bad [] (> 3 true))\n",
    "logic-not": "(defn bad [] (not 1))\n",
    "logic-and": "(defn bad [] (and true 1))\n",
    "logic-or": "(defn bad [] (or 1 false))\n",
    "substring": "(defn bad [] (substring \"abc\" true 1))\n",
    "write-file-bytes": "(defn bad [] (write-file-bytes 3 (vector-new 2)))\n",
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
