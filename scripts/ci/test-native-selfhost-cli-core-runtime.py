#!/usr/bin/env python3
"""Check the native App.Cli core command surface without a Rust fallback."""

import argparse
import json
import pathlib
import subprocess
import tempfile


INPUT_SOURCE = "(defn main [] 42)\n"
METADATA_SOURCE = """(defn abs [x]
  :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"""
DOC_JSON_SOURCE = "(defn main [] 42)\n"
LSP_SOURCE = "(defn add [x y] (+ x y))\n(defn main [] (add 1 2))\n"
VALIDATION_SOURCE = """(defn cancel []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
  :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
  true)
"""
STRICT_DOC_SOURCE = (
    "(defn main [] 42)\n"
    "; Doc-Review-Status: Passed\n"
    "; Doc-Reviewed-By: anonymous\n"
)


def run(program, root, args, stdin=b""):
    result = subprocess.run(
        [str(program), *args],
        cwd=root,
        input=stdin,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"{' '.join(args)} failed: exit={result.returncode} "
            f"stdout={result.stdout!r} stderr={result.stderr!r}"
        )
    if result.stderr:
        raise AssertionError(f"{' '.join(args)} emitted stderr: {result.stderr!r}")
    return result.stdout


def require_exact(label, actual, expected):
    if actual != expected:
        raise AssertionError(f"{label} mismatch: actual={actual!r} expected={expected!r}")


def require_contains(label, actual, expected):
    if expected not in actual:
        raise AssertionError(f"{label} is missing {expected!r}: actual={actual!r}")


def lsp_frame(payload):
    body = json.dumps(payload, separators=(",", ":")).encode("utf-8")
    return b"Content-Length: " + str(len(body)).encode("ascii") + b"\r\n\r\n" + body


def parse_lsp_frames(data):
    frames = []
    offset = 0
    while offset < len(data):
        header_end = data.find(b"\r\n\r\n", offset)
        if header_end < 0:
            raise AssertionError("native lsp output is missing frame header terminator")
        content_length = None
        for line in data[offset:header_end].split(b"\r\n"):
            name, value = line.split(b":", 1)
            if name.lower() == b"content-length":
                content_length = int(value.strip())
        if content_length is None:
            raise AssertionError("native lsp output is missing Content-Length")
        body_start = header_end + 4
        body_end = body_start + content_length
        if body_end > len(data):
            raise AssertionError("native lsp output has a truncated body")
        frames.append(json.loads(data[body_start:body_end].decode("utf-8")))
        offset = body_end
    return frames


def run_lsp_stdio(program, root):
    uri = "file:///tmp/lsharp-native-cli-core.ls"
    wire = b"".join(
        [
            lsp_frame(
                {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {"capabilities": {}, "rootUri": None},
                }
            ),
            lsp_frame(
                {
                    "jsonrpc": "2.0",
                    "method": "textDocument/didOpen",
                    "params": {
                        "textDocument": {
                            "uri": uri,
                            "languageId": "lsharp",
                            "version": 1,
                            "text": LSP_SOURCE,
                        }
                    },
                }
            ),
            lsp_frame(
                {
                    "jsonrpc": "2.0",
                    "id": 2,
                    "method": "textDocument/hover",
                    "params": {
                        "textDocument": {"uri": uri},
                        "position": {"line": 0, "character": 7},
                    },
                }
            ),
        ]
    )
    result = subprocess.run(
        [str(program), "lsp", "--stdio"],
        cwd=root,
        input=wire,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"lsp --stdio failed: exit={result.returncode} "
            f"stdout={result.stdout!r} stderr={result.stderr!r}"
        )
    if result.stderr:
        raise AssertionError(f"lsp --stdio emitted stderr: {result.stderr!r}")
    return parse_lsp_frames(result.stdout), uri


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--program", required=True, type=pathlib.Path)
    args = parser.parse_args()
    program = args.program.expanduser().resolve()
    if not program.is_file() or not program.stat().st_mode & 0o111:
        raise SystemExit(f"native program is not executable: {program}")

    with tempfile.TemporaryDirectory(prefix="lsharp-native-cli-core-") as directory:
        root = pathlib.Path(directory)
        (root / "input.ls").write_text(INPUT_SOURCE, encoding="utf-8")
        (root / "metadata.ls").write_text(METADATA_SOURCE, encoding="utf-8")
        (root / "doc-json.ls").write_text(DOC_JSON_SOURCE, encoding="utf-8")
        (root / "validation.ls").write_text(VALIDATION_SOURCE, encoding="utf-8")
        (root / "strict.ls").write_text(STRICT_DOC_SOURCE, encoding="utf-8")

        require_contains("help", run(program, root, ["--help"]), b"Usage: lsharp")
        require_exact("version", run(program, root, ["--version"]), b"lsharp 0.1.0")
        require_exact(
            "parse",
            run(program, root, ["parse", "input.ls"]),
            b"decls:1\nfirst-decl:defn\nfirst-body:int\ndiagnostics:0\n",
        )
        require_exact("check", run(program, root, ["check", "input.ls"]), b"Int\ndiagnostics:0\n")
        require_exact("fmt", run(program, root, ["fmt", "input.ls"]), b"(defn main [] 42)\n")
        require_exact(
            "test",
            run(program, root, ["test", "input.ls"]),
            b"examples:0\ninvariants:0\nfailures:0\n",
        )
        require_exact(
            "metadata test",
            run(program, root, ["test", "metadata.ls"]),
            b"examples:2\ninvariants:1\nfailures:0\n",
        )

        for command in ("compile", "build"):
            output_path = root / f"{command}.wasm"
            stdout = run(program, root, [command, "input.ls", "-o", str(output_path)])
            if not output_path.is_file() or output_path.read_bytes()[:4] != b"\x00asm":
                raise AssertionError(f"{command} did not produce a core Wasm artifact")
            require_contains(f"{command} summary", stdout, b"wasm-size:")

        require_exact(
            "review",
            run(program, root, ["review", "input.ls"]),
            b"0\nclean\ndiagnostics:0\nclean\n-\n",
        )
        require_exact(
            "review json",
            run(program, root, ["review", "input.ls", "--json"]),
            b'{"source":"source-200","diagnostics":[]}\n',
        )
        require_exact(
            "doc",
            run(program, root, ["doc", "input.ls"]),
            b"module-global\nfunctions:1,types:0,first-fn:main\n",
        )
        doc_json = run(program, root, ["doc", "doc-json.ls", "--json"])
        require_exact(
            "doc json",
            doc_json,
            b'{"module":"module-42","functions":[{"name":"main","arity":0,"params":[],"returns":{"type":"Int","doc":""},"doc":"","example":""}],"types":[],"html":{"title":"module-42","sections":[{"id":"functions","count":1}]}}\n',
        )
        require_exact(
            "doc format json",
            run(program, root, ["doc", "doc-json.ls", "--format", "json"]),
            doc_json,
        )
        require_exact(
            "doc ack",
            run(program, root, ["doc-ack", "input.ls"]),
            b"ack:recorded\nmodule-global\nfunctions:1,types:0,first-fn:main\n; Doc-Reviewed-By: anonymous\n",
        )
        require_exact(
            "doc ack trailer",
            run(program, root, ["doc-ack", "input.ls", "--trailer"]),
            b"; Doc-Reviewed-By: anonymous\n",
        )
        require_exact(
            "doc check",
            run(program, root, ["doc-check", "input.ls"]),
            b"status:ok\nmodule-global\nfunctions:1,types:0,first-fn:main\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: anonymous\n",
        )
        require_exact(
            "doc check strict",
            run(program, root, ["doc-check", "strict.ls", "--strict"]),
            b"status:ok\nmodule-global\nfunctions:1,types:0,first-fn:main\n; Doc-Review-Status: Passed\n; Doc-Reviewed-By: anonymous\n",
        )
        validation = subprocess.run(
            [
                str(program),
                "validate",
                "--source",
                "validation.ls",
                "--format",
                "text",
            ],
            cwd=root,
            capture_output=True,
            check=False,
        )
        if validation.returncode != 2 or validation.stderr:
            raise AssertionError(
                "validate source boundary mismatch: "
                f"exit={validation.returncode} stderr={validation.stderr!r}"
            )
        require_exact(
            "validate source text",
            validation.stdout,
            b"status: unknown\n"
            b"trace-gap.claim-without-test: claim:checkout/cancel-rejects-shipped\n"
            b"open-questions: 0\n"
            b"independent-reviews: 0\n"
            b"contradicting-observations: 0\n"
            b"stale-reviews: 0\n"
            b"stale-evidence: 0\n",
        )
        require_exact(
            "install",
            run(program, root, ["install", "core"]),
            b"package:core\nstatus:planned\n",
        )
        require_exact(
            "repl",
            run(program, root, ["repl"]),
            b"type:Int\nevals:1\ninput-bytes:17\n",
        )
        require_exact(
            "lsp summary",
            run(program, root, ["lsp"]),
            b"sync:full\nhover:true\ncompletion:true\ndefinition:true\nreferences:true\nrename:true\nformatting:true\nrequests:1\ndocuments:0\nsource-bytes:0\n",
        )
        lsp_frames, uri = run_lsp_stdio(program, root)
        if len(lsp_frames) != 3:
            raise AssertionError(f"lsp --stdio frame count mismatch: {lsp_frames!r}")
        capabilities = lsp_frames[0].get("result", {}).get("capabilities", {})
        if capabilities.get("hoverProvider") is not True:
            raise AssertionError(f"lsp initialize hover capability missing: {lsp_frames[0]!r}")
        if lsp_frames[1] != {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {"sourceBytes": len(LSP_SOURCE.encode("utf-8")), "uri": uri},
        }:
            raise AssertionError(f"lsp didOpen projection mismatch: {lsp_frames[1]!r}")
        hover = lsp_frames[2]
        if hover.get("id") != 2 or hover.get("result", {}).get("contents") != "defn add":
            raise AssertionError(f"lsp hover projection mismatch: {hover!r}")

        invalid_doc = subprocess.run(
            [str(program), "doc", "input.ls", "--format", "yaml"],
            cwd=root,
            capture_output=True,
            check=False,
        )
        if invalid_doc.returncode != 1 or invalid_doc.stderr:
            raise AssertionError(
                f"invalid doc format boundary mismatch: exit={invalid_doc.returncode} "
                f"stdout={invalid_doc.stdout!r} stderr={invalid_doc.stderr!r}"
            )
        require_exact(
            "invalid doc format",
            invalid_doc.stdout,
            b"error: unsupported option: yaml\n",
        )

    print("native CLI core runtime matrix passed: 23 cases")


if __name__ == "__main__":
    main()
