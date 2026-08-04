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
LSP_NAV_URI = "file:///tmp/lsharp-uri-contract.ls"
LSP_NAV_SOURCE = "(defn helper [x] x)\n(defn main [] (helper 1))"
LSP_DIDCHANGE_VALID_SOURCE = "(defn main [] 42)\n"
LSP_DIDCHANGE_INVALID_SOURCE = "(defn bad [] (+ 1 true))\n"
LSP_LINT_URI = "file:///tmp/lsharp-lsp-lint.ls"
LSP_LINT_SOURCE = "(defn main [] (let [unused 42] 0))\n"
LSP_HYPHEN_LINT_URI = "file:///tmp/lsharp-lsp-lint-hyphen.ls"
LSP_HYPHEN_LINT_SOURCE = "(defn main [] (let [unused-a 42] 0))\n"
LSP_TYPE_UNDEFINED_URI = "file:///tmp/lsharp-lsp-type-undefined.ls"
LSP_TYPE_UNDEFINED_SOURCE = "(defn main [] missing)\n"
LSP_TYPE_IF_URI = "file:///tmp/lsharp-lsp-type-if.ls"
LSP_TYPE_IF_SOURCE = "(defn main [] (if 1 true false))\n"
LSP_TYPE_ARGUMENT_URI = "file:///tmp/lsharp-lsp-type-argument.ls"
LSP_TYPE_ARGUMENT_SOURCE = "(defn bad [] (+ 1 true))\n"
LSP_TYPE_INFINITE_URI = "file:///tmp/lsharp-lsp-type-infinite.ls"
LSP_TYPE_INFINITE_SOURCE = "(defn main [x] (x x))\n"
LSP_PARSE_URI = "file:///tmp/lsharp-lsp-parse-standard.ls"
LSP_PARSE_SOURCE = ")"
LSP_EMPTY_DO_URI = "file:///tmp/lsharp-lsp-empty-do.ls"
LSP_EMPTY_DO_SOURCE = "(defn main [] (do))\n"
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


def run_lsp_didchange_diagnostics(program, root, *, clear=False):
    uri = "file:///tmp/lsharp-lsp-didchange.ls"
    messages = [
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
                        "text": LSP_DIDCHANGE_VALID_SOURCE,
                    }
                },
            }
        ),
        lsp_frame(
            {
                "jsonrpc": "2.0",
                "method": "textDocument/didChange",
                "params": {
                    "textDocument": {"uri": uri, "version": 2},
                    "contentChanges": [{"text": LSP_DIDCHANGE_INVALID_SOURCE}],
                },
            }
        ),
    ]
    if clear:
        messages.append(
            lsp_frame(
                {
                    "jsonrpc": "2.0",
                    "method": "textDocument/didChange",
                    "params": {
                        "textDocument": {"uri": uri, "version": 3},
                        "contentChanges": [{"text": LSP_DIDCHANGE_VALID_SOURCE}],
                    },
                }
            )
        )
    wire = b"".join(messages)
    result = subprocess.run(
        [str(program), "lsp", "--stdio"],
        cwd=root,
        input=wire,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"lsp didChange diagnostics failed: exit={result.returncode} "
            f"stdout={result.stdout!r} stderr={result.stderr!r}"
        )
    if result.stderr:
        raise AssertionError(
            f"lsp didChange diagnostics emitted stderr: {result.stderr!r}"
        )
    return parse_lsp_frames(result.stdout), uri


def run_lsp_source_diagnostics(program, root, uri, source, label):
    messages = [
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
                        "text": source,
                    }
                },
            }
        ),
    ]
    result = subprocess.run(
        [str(program), "lsp", "--stdio"],
        cwd=root,
        input=b"".join(messages),
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"lsp {label} diagnostics failed: exit={result.returncode} "
            f"stdout={result.stdout!r} stderr={result.stderr!r}"
        )
    if result.stderr:
        raise AssertionError(
            f"lsp {label} diagnostics emitted stderr: {result.stderr!r}"
        )
    return parse_lsp_frames(result.stdout)


def run_lsp_definition(program, root):
    messages = [
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
                        "uri": LSP_NAV_URI,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": LSP_NAV_SOURCE,
                    }
                },
            }
        ),
        lsp_frame(
            {
                "jsonrpc": "2.0",
                "id": 93,
                "method": "textDocument/definition",
                "params": {
                    "textDocument": {"uri": LSP_NAV_URI},
                    "position": {"line": 1, "character": 15},
                },
            }
        ),
        lsp_frame(
            {
                "jsonrpc": "2.0",
                "id": 94,
                "method": "textDocument/references",
                "params": {
                    "textDocument": {"uri": LSP_NAV_URI},
                    "position": {"line": 1, "character": 15},
                    "context": {"includeDeclaration": True},
                },
            }
        ),
        lsp_frame(
            {
                "jsonrpc": "2.0",
                "id": 95,
                "method": "textDocument/rename",
                "params": {
                    "textDocument": {"uri": LSP_NAV_URI},
                    "position": {"line": 1, "character": 15},
                    "newName": "value",
                },
            }
        ),
    ]
    result = subprocess.run(
        [str(program), "lsp", "--stdio"],
        cwd=root,
        input=b"".join(messages),
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"lsp navigation failed: exit={result.returncode} "
            f"stdout={result.stdout!r} stderr={result.stderr!r}"
        )
    if result.stderr:
        raise AssertionError(f"lsp navigation emitted stderr: {result.stderr!r}")
    return parse_lsp_frames(result.stdout)


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

        didchange_frames, didchange_uri = run_lsp_didchange_diagnostics(program, root)
        if len(didchange_frames) != 4:
            raise AssertionError(
                f"lsp didChange diagnostics frame count mismatch: {didchange_frames!r}"
            )
        if didchange_frames[1] != {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "sourceBytes": len(LSP_DIDCHANGE_VALID_SOURCE.encode("utf-8")),
                "uri": didchange_uri,
            },
        }:
            raise AssertionError(
                f"lsp didChange didOpen projection mismatch: {didchange_frames[1]!r}"
            )
        if didchange_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "sourceBytes": len(LSP_DIDCHANGE_INVALID_SOURCE.encode("utf-8")),
                "uri": didchange_uri,
            },
        }:
            raise AssertionError(
                f"lsp didChange projection mismatch: {didchange_frames[2]!r}"
            )
        diagnostics = didchange_frames[3]
        if diagnostics != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": didchange_uri,
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
        }:
            raise AssertionError(
                f"lsp didChange diagnostics projection mismatch: {diagnostics!r}"
            )

        clear_frames, clear_uri = run_lsp_didchange_diagnostics(
            program, root, clear=True
        )
        if len(clear_frames) != 6:
            raise AssertionError(
                f"lsp didChange clear frame count mismatch: {clear_frames!r}"
            )
        if clear_frames[4] != {
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "sourceBytes": len(LSP_DIDCHANGE_VALID_SOURCE.encode("utf-8")),
                "uri": clear_uri,
            },
        }:
            raise AssertionError(
                f"lsp valid didChange projection mismatch: {clear_frames[4]!r}"
            )
        if clear_frames[5] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": clear_uri, "diagnostics": []},
        }:
            raise AssertionError(
                f"lsp stale diagnostics clear mismatch: {clear_frames[5]!r}"
            )

        lint_frames = run_lsp_source_diagnostics(
            program, root, LSP_LINT_URI, LSP_LINT_SOURCE, "lint"
        )
        if len(lint_frames) != 3:
            raise AssertionError(
                f"lsp lint diagnostics frame count mismatch: {lint_frames!r}"
            )
        if lint_frames[1] != {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "sourceBytes": len(LSP_LINT_SOURCE.encode("utf-8")),
                "uri": LSP_LINT_URI,
            },
        }:
            raise AssertionError(
                f"lsp lint didOpen projection mismatch: {lint_frames[1]!r}"
            )
        if lint_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_LINT_URI,
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 0},
                        },
                        "severity": 2,
                        "code": "L0001",
                        "source": "lsharp",
                        "message": "let binding unused is not used",
                    }
                ],
            },
        }:
            raise AssertionError(
                f"lsp lint diagnostics projection mismatch: {lint_frames[2]!r}"
            )

        hyphen_lint_frames = run_lsp_source_diagnostics(
            program,
            root,
            LSP_HYPHEN_LINT_URI,
            LSP_HYPHEN_LINT_SOURCE,
            "hyphenated-lint",
        )
        if len(hyphen_lint_frames) != 3:
            raise AssertionError(
                f"lsp hyphenated lint diagnostics frame count mismatch: {hyphen_lint_frames!r}"
            )
        if hyphen_lint_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_HYPHEN_LINT_URI,
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 0},
                        },
                        "severity": 2,
                        "code": "L0001",
                        "source": "lsharp",
                        "message": "let binding unusebka is not used",
                    }
                ],
            },
        }:
            raise AssertionError(
                f"lsp hyphenated lint diagnostics projection mismatch: {hyphen_lint_frames[2]!r}"
            )

        undefined_type_frames = run_lsp_source_diagnostics(
            program,
            root,
            LSP_TYPE_UNDEFINED_URI,
            LSP_TYPE_UNDEFINED_SOURCE,
            "undefined-type",
        )
        if undefined_type_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_TYPE_UNDEFINED_URI,
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 0},
                        },
                        "severity": 1,
                        "code": "LS1001",
                        "source": "lsharp",
                        "message": "undefined symbol",
                    }
                ],
            },
        }:
            raise AssertionError(
                f"lsp undefined type diagnostics projection mismatch: {undefined_type_frames[2]!r}"
            )

        if_type_frames = run_lsp_source_diagnostics(
            program,
            root,
            LSP_TYPE_IF_URI,
            LSP_TYPE_IF_SOURCE,
            "if-type",
        )
        if if_type_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_TYPE_IF_URI,
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 0},
                        },
                        "severity": 1,
                        "code": "LS1002",
                        "source": "lsharp",
                        "message": "if condition must be Bool",
                    }
                ],
            },
        }:
            raise AssertionError(
                f"lsp if type diagnostics projection mismatch: {if_type_frames[2]!r}"
            )

        argument_type_frames = run_lsp_source_diagnostics(
            program,
            root,
            LSP_TYPE_ARGUMENT_URI,
            LSP_TYPE_ARGUMENT_SOURCE,
            "argument-type",
        )
        if argument_type_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_TYPE_ARGUMENT_URI,
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
        }:
            raise AssertionError(
                f"lsp argument type diagnostics projection mismatch: {argument_type_frames[2]!r}"
            )

        infinite_type_frames = run_lsp_source_diagnostics(
            program,
            root,
            LSP_TYPE_INFINITE_URI,
            LSP_TYPE_INFINITE_SOURCE,
            "infinite-type",
        )
        if infinite_type_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_TYPE_INFINITE_URI,
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 0},
                        },
                        "severity": 1,
                        "code": "LS1003",
                        "source": "lsharp",
                        "message": "infinite type",
                    }
                ],
            },
        }:
            raise AssertionError(
                f"lsp infinite type diagnostics projection mismatch: {infinite_type_frames[2]!r}"
            )

        parse_frames = run_lsp_source_diagnostics(
            program, root, LSP_PARSE_URI, LSP_PARSE_SOURCE, "parse"
        )
        if parse_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_PARSE_URI,
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 1},
                        },
                        "severity": 1,
                        "code": "LS0101",
                        "source": "lsharp",
                        "message": "unexpected token )",
                    }
                ],
            },
        }:
            raise AssertionError(
                f"lsp parse diagnostics projection mismatch: {parse_frames[2]!r}"
            )

        definition_frames = run_lsp_definition(program, root)
        if definition_frames[2] != {
            "jsonrpc": "2.0",
            "id": 93,
            "result": {
                "uri": LSP_NAV_URI,
                "range": {
                    "start": {"line": 0, "character": 6},
                    "end": {"line": 0, "character": 6},
                },
            },
        }:
            raise AssertionError(
                f"lsp definition projection mismatch: {definition_frames[2]!r}"
            )

        if definition_frames[3] != {
            "jsonrpc": "2.0",
            "id": 94,
            "result": [
                {
                    "uri": LSP_NAV_URI,
                    "range": {
                        "start": {"line": 0, "character": 6},
                        "end": {"line": 0, "character": 6},
                    },
                },
                {
                    "uri": LSP_NAV_URI,
                    "range": {
                        "start": {"line": 1, "character": 15},
                        "end": {"line": 1, "character": 15},
                    },
                },
            ],
        }:
            raise AssertionError(
                f"lsp references projection mismatch: {definition_frames[3]!r}"
            )

        if definition_frames[4] != {
            "jsonrpc": "2.0",
            "id": 95,
            "result": {
                "changes": {
                    LSP_NAV_URI: [
                        {
                            "range": {
                                "start": {"line": 0, "character": 6},
                                "end": {"line": 0, "character": 12},
                            },
                            "newText": "value",
                        },
                        {
                            "range": {
                                "start": {"line": 1, "character": 15},
                                "end": {"line": 1, "character": 21},
                            },
                            "newText": "value",
                        },
                    ]
                }
            },
        }:
            raise AssertionError(
                f"lsp rename projection mismatch: {definition_frames[4]!r}"
            )

        empty_do_frames = run_lsp_source_diagnostics(
            program, root, LSP_EMPTY_DO_URI, LSP_EMPTY_DO_SOURCE, "empty-do"
        )
        if len(empty_do_frames) != 3:
            raise AssertionError(
                f"lsp empty-do diagnostics frame count mismatch: {empty_do_frames!r}"
            )
        if empty_do_frames[1] != {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "sourceBytes": len(LSP_EMPTY_DO_SOURCE.encode("utf-8")),
                "uri": LSP_EMPTY_DO_URI,
            },
        }:
            raise AssertionError(
                f"lsp empty-do didOpen projection mismatch: {empty_do_frames[1]!r}"
            )
        if empty_do_frames[2] != {
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": LSP_EMPTY_DO_URI,
                "diagnostics": [
                    {
                        "range": {
                            "start": {"line": 0, "character": 0},
                            "end": {"line": 0, "character": 0},
                        },
                        "severity": 2,
                        "code": "L0002",
                        "source": "lsharp",
                        "message": "do block has no expressions",
                    }
                ],
            },
        }:
            raise AssertionError(
                f"lsp empty-do diagnostics projection mismatch: {empty_do_frames[2]!r}"
            )

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

    print("native CLI core runtime matrix passed: 33 cases")


if __name__ == "__main__":
    main()
