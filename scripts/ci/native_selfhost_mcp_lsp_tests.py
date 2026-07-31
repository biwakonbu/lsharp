import base64
import json
import pathlib
import tempfile


def frame(body):
    return b"Content-Length: " + str(len(body)).encode("ascii") + b"\r\n\r\n" + body


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def write_fake_lsp_program(root, output, mode="success"):
    program = root / "lsp-program.native"
    encoded = base64.b64encode(output).decode("ascii")
    program.write_text(
        """#!/usr/bin/env python3
import base64
import json
import pathlib
import sys

root = pathlib.Path(%r)
root.joinpath("lsp-input.bin").write_bytes(sys.stdin.buffer.read())
root.joinpath("lsp-args.json").write_text(json.dumps(sys.argv[1:]), encoding="utf-8")
mode = %r
if mode == "stderr":
    sys.stderr.write("native lsp diagnostic\\n")
    raise SystemExit(0)
if mode == "nonzero":
    sys.stderr.write("native lsp failure\\n")
    raise SystemExit(23)
if mode == "malformed":
    sys.stdout.buffer.write(b"Content-Length: 7\\r\\n\\r\\nshort")
    raise SystemExit(0)
sys.stdout.buffer.write(base64.b64decode(%r))
"""
        % (str(root), mode, encoded),
        encoding="ascii",
    )
    program.chmod(0o755)
    return program


def initialize_output():
    initialize = json.dumps(
        {"jsonrpc": "2.0", "id": 1, "result": {"capabilities": {}}},
        separators=(",", ":"),
    ).encode()
    return frame(initialize)


def lsp_output(hover_text):
    hover = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "result": {"contents": {"kind": "plaintext", "value": hover_text}},
        },
        separators=(",", ":"),
    ).encode()
    return initialize_output() + frame(hover)


def definition_output(start=(0, 5), end=(0, 8)):
    definition = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "uri": "file:///tmp/input.ls",
                "range": {
                    "start": {"line": start[0], "character": start[1]},
                    "end": {"line": end[0], "character": end[1]},
                },
            },
        },
        separators=(",", ":"),
    ).encode()
    return initialize_output() + frame(definition)


def assert_definition_projects_native_lsp(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = write_fake_lsp_program(root, definition_output((0, 6), (0, 9)))
        payload = b"".join(
            [
                request(1, "tools/list"),
                request(
                    2,
                    "tools/call",
                    {
                        "name": "lsharp_definition",
                        "arguments": {
                            "source": "(defn add [x y] (+ x y))\n",
                            "line": 0,
                            "character": 18,
                        },
                    },
                ),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        tool = next(tool for tool in responses[0]["result"]["tools"] if tool["name"] == "lsharp_definition")
        test.assertEqual(tool["inputSchema"]["oneOf"], [{"required": ["line", "character"]}])
        test.assertFalse(tool["inputSchema"]["additionalProperties"])
        test.assertEqual(tool["outputSchema"]["required"], ["start", "end"])
        test.assertEqual(
            responses[1]["result"]["structuredContent"],
            {
                "start": {"line": 0, "character": 6},
                "end": {"line": 0, "character": 9},
            },
        )
        messages = [json.loads(body) for body in _parse_frames((root / "lsp-input.bin").read_bytes())]
        test.assertEqual(
            [message["method"] for message in messages],
            ["initialize", "initialized", "textDocument/didOpen", "textDocument/definition"],
        )
        test.assertEqual(messages[-1]["params"]["position"], {"line": 0, "character": 18})


def assert_definition_supports_file_and_col_alias(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        source = root / "input.ls"
        source.write_text("(defn id [x] x)\n", encoding="utf-8")
        program = write_fake_lsp_program(root, definition_output((0, 6), (0, 8)))
        payload = request(
            1,
            "tools/call",
            {
                "name": "lsharp_definition",
                "arguments": {"file": str(source), "line": 0, "col": 7},
            },
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertEqual(
            response["result"]["structuredContent"],
            {
                "start": {"line": 0, "character": 6},
                "end": {"line": 0, "character": 8},
            },
        )
        messages = [json.loads(body) for body in _parse_frames((root / "lsp-input.bin").read_bytes())]
        test.assertEqual(messages[2]["params"]["textDocument"]["uri"], source.resolve().as_uri())


def assert_definition_rejects_invalid_arguments_before_native(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = write_fake_lsp_program(root, definition_output())
        cases = [
            {"unknown": True, "line": 0, "character": 0},
            {"source": "x", "character": 0},
            {"source": "x", "line": -1, "character": 0},
            {"source": "x", "line": 0, "character": True},
            {"source": "x", "file": "other.ls", "line": 0, "character": 0},
        ]
        payload = b"".join(
            request(index, "tools/call", {"name": "lsharp_definition", "arguments": arguments})
            for index, arguments in enumerate(cases, 1)
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response in responses:
            test.assertTrue(response["result"]["isError"])
        test.assertIn("未知", responses[0]["result"]["content"][0]["text"])
        test.assertIn("line", responses[1]["result"]["content"][0]["text"])
        test.assertFalse((root / "lsp-input.bin").exists())


def assert_definition_rejects_native_failures(test):
    invalid_range = json.dumps(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "result": {"range": {"start": {"line": 0, "character": 1}}},
        },
        separators=(",", ":"),
    ).encode()
    cases = (
        ("stderr", definition_output(), "diagnostic"),
        ("nonzero", definition_output(), "failure"),
        ("malformed", definition_output(), "malformed"),
        ("success", initialize_output(), "response がありません"),
        ("success", initialize_output() + frame(invalid_range), "range"),
    )
    for mode, output, message in cases:
        with test.subTest(mode=mode, message=message), tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = write_fake_lsp_program(root, output, mode=mode)
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_definition",
                    "arguments": {"source": "x", "line": 0, "character": 0},
                },
            )
            result = test.run_shim(program, payload, root)
            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(message, response["result"]["content"][0]["text"])


def assert_hover_projects_native_lsp(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = write_fake_lsp_program(root, lsp_output("add : Int -> Int -> Int\n加算"))
        payload = b"".join(
            [
                request(1, "tools/list"),
                request(
                    2,
                    "tools/call",
                    {
                        "name": "lsharp_hover",
                        "arguments": {
                            "source": "(defn add [x y] (+ x y))\n",
                            "line": 0,
                            "character": 8,
                        },
                    },
                ),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        tool = next(tool for tool in responses[0]["result"]["tools"] if tool["name"] == "lsharp_hover")
        test.assertEqual(tool["inputSchema"]["oneOf"], [{"required": ["line", "character"]}])
        test.assertFalse(tool["inputSchema"]["additionalProperties"])
        test.assertEqual(tool["outputSchema"]["required"], ["name", "type", "doc"])
        test.assertEqual(
            responses[1]["result"]["structuredContent"],
            {"name": "add", "type": "Int -> Int -> Int", "doc": "加算"},
        )
        sent = _parse_frames((root / "lsp-input.bin").read_bytes())
        messages = [json.loads(body) for body in sent]
        test.assertEqual(
            [message["method"] for message in messages],
            ["initialize", "initialized", "textDocument/didOpen", "textDocument/hover"],
        )
        test.assertEqual(messages[-1]["params"]["position"], {"line": 0, "character": 8})
        test.assertEqual(
            json.loads((root / "lsp-args.json").read_text(encoding="utf-8")),
            ["lsp", "--stdio"],
        )


def assert_hover_supports_file_and_col_alias(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        source = root / "input.ls"
        source.write_text("(defn id [x] x)\n", encoding="utf-8")
        program = write_fake_lsp_program(root, lsp_output("id : a -> a"))
        payload = request(
            1,
            "tools/call",
            {
                "name": "lsharp_hover",
                "arguments": {"file": str(source), "line": 0, "col": 7},
            },
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertEqual(
            response["result"]["structuredContent"],
            {"name": "id", "type": "a -> a", "doc": None},
        )
        messages = [json.loads(body) for body in _parse_frames((root / "lsp-input.bin").read_bytes())]
        test.assertEqual(messages[2]["params"]["textDocument"]["uri"], source.resolve().as_uri())


def assert_hover_rejects_invalid_arguments_before_native(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = write_fake_lsp_program(root, lsp_output("unused"))
        cases = [
            {"unknown": True, "line": 0, "character": 0},
            {"source": "x", "character": 0},
            {"source": "x", "line": -1, "character": 0},
            {"source": "x", "line": 0, "character": "0"},
            {"source": "x", "file": "other.ls", "line": 0, "character": 0},
        ]
        payload = b"".join(
            request(index, "tools/call", {"name": "lsharp_hover", "arguments": arguments})
            for index, arguments in enumerate(cases, 1)
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response in responses:
            test.assertTrue(response["result"]["isError"])
        test.assertIn("未知", responses[0]["result"]["content"][0]["text"])
        test.assertIn("line", responses[1]["result"]["content"][0]["text"])
        test.assertFalse((root / "lsp-input.bin").exists())


def assert_hover_rejects_native_failures(test):
    cases = (
        ("stderr", lsp_output("unused"), "diagnostic"),
        ("nonzero", lsp_output("unused"), "failure"),
        ("malformed", lsp_output("unused"), "malformed"),
        ("success", initialize_output(), "response がありません"),
        ("success", lsp_output("invalid hover payload"), "signature"),
    )
    for mode, output, message in cases:
        with test.subTest(mode=mode), tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = write_fake_lsp_program(root, output, mode=mode)
            payload = request(
                1,
                "tools/call",
                {"name": "lsharp_hover", "arguments": {"source": "x", "line": 0, "character": 0}},
            )
            result = test.run_shim(program, payload, root)
            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(message, response["result"]["content"][0]["text"])


def _parse_frames(data):
    frames = []
    offset = 0
    while offset < len(data):
        header_end = data.find(b"\r\n\r\n", offset)
        if header_end < 0:
            raise AssertionError("missing LSP frame header")
        length = next(
            int(value.strip())
            for name, value in (line.split(b":", 1) for line in data[offset:header_end].split(b"\r\n"))
            if name.lower() == b"content-length"
        )
        body_start = header_end + 4
        body_end = body_start + length
        if body_end > len(data):
            raise AssertionError("truncated LSP frame")
        frames.append(data[body_start:body_end])
        offset = body_end
    return frames
