import json
import pathlib
import sys
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def write_executable(path, source):
    path.write_text(source, encoding="utf-8")
    path.chmod(0o755)
    return path


def write_compile_program(root):
    return write_executable(
        root / "compile-program.native",
        """#!__PYTHON__
import json
import os
import pathlib
import sys


def record(kind, **values):
    payload = {"kind": kind}
    payload.update(values)
    with open(os.environ["FAKE_NATIVE_LOG"], "a", encoding="utf-8") as output:
        output.write(json.dumps(payload) + "\\n")


arguments = sys.argv[1:]
if len(arguments) != 4 or arguments[0] != "compile" or arguments[2] != "-o":
    print("fake native compile received unexpected arguments", file=sys.stderr)
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
if "omit-artifact" not in source_text:
    wasm_path.write_text(source_text, encoding="utf-8")
print("fake native compile noise")
""".replace("__PYTHON__", sys.executable),
    )


def write_wasmtime(root):
    return write_executable(
        root / "wasmtime",
        """#!__PYTHON__
import json
import os
import pathlib
import sys


arguments = sys.argv[1:]
if len(arguments) != 1:
    print("fake wasmtime received unexpected arguments", file=sys.stderr)
    raise SystemExit(98)
wasm_path = pathlib.Path(arguments[0])
source_text = wasm_path.read_text(encoding="utf-8")
record = {
    "kind": "runtime",
    "arguments": arguments,
    "wasm": str(wasm_path),
    "source_text": source_text,
}
with open(os.environ["FAKE_WASMTIME_LOG"], "a", encoding="utf-8") as output:
    output.write(json.dumps(record) + "\\n")
if "runtime-fail" in source_text:
    print("fake wasmtime runtime failure", file=sys.stderr)
    raise SystemExit(23)
sys.stdout.write("fake wasmtime output: " + source_text)
""".replace("__PYTHON__", sys.executable),
    )


def setup(test):
    temporary_directory = tempfile.TemporaryDirectory()
    test.addCleanup(temporary_directory.cleanup)
    root = pathlib.Path(temporary_directory.name)
    program = write_compile_program(root)
    wasmtime = write_wasmtime(root)
    (root / "wasmtime.log").write_text("", encoding="utf-8")
    return root, program, wasmtime


def records(path):
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ] if path.exists() else []


def call(test, program, wasmtime, root, arguments):
    payload = request(
        1,
        "tools/call",
        {"name": "lsharp_compile_run", "arguments": arguments},
    )
    result = test.run_shim(program, payload, root, wasmtime_path=wasmtime)
    test.assertEqual(result.returncode, 0, result.stderr.decode())
    return test.responses(result.stdout)[0]["result"]


def assert_compile_run_projects_source_and_external_runtime(test):
    root, program, wasmtime = setup(test)
    response = call(test, program, wasmtime, root, {"source": "(print 42)\n"})
    test.assertFalse(response["isError"])
    test.assertEqual(
        response["structuredContent"],
        {
            "ok": True,
            "formatted": "(print 42)\n",
            "stdout": "fake wasmtime output: (print 42)\n",
            "exit_code": 0,
        },
    )
    compile_record = records(root / "native.log")[0]
    runtime_record = records(root / "wasmtime.log")[0]
    test.assertEqual(compile_record["kind"], "compile")
    test.assertEqual(runtime_record["kind"], "runtime")
    test.assertEqual(runtime_record["source_text"], "(print 42)\n")
    test.assertEqual(pathlib.Path(runtime_record["wasm"]).resolve(), pathlib.Path(compile_record["wasm"]).resolve())
    test.assertFalse(pathlib.Path(compile_record["source"]).exists())
    test.assertFalse(pathlib.Path(compile_record["wasm"]).exists())


def assert_compile_run_projects_file_without_mutating_input(test):
    root, program, wasmtime = setup(test)
    source = root / "input.ls"
    source.write_text("(defn main [] (print 7))\n", encoding="utf-8")
    response = call(test, program, wasmtime, root, {"file": str(source)})
    test.assertFalse(response["isError"])
    test.assertEqual(response["structuredContent"]["formatted"], source.read_text(encoding="utf-8"))
    test.assertEqual(source.read_text(encoding="utf-8"), "(defn main [] (print 7))\n")
    compile_record = records(root / "native.log")[0]
    test.assertNotEqual(pathlib.Path(compile_record["source"]).resolve(), source.resolve())


def assert_compile_run_rejects_invalid_arguments_before_native(test):
    root, program, wasmtime = setup(test)
    cases = [
        {},
        {"source": "", "file": "input.ls"},
        {"source": 42},
        {"file": str(root / "missing.ls")},
        {"source": "x", "unknown": True},
    ]
    payload = b"".join(
        request(index, "tools/call", {"name": "lsharp_compile_run", "arguments": arguments})
        for index, arguments in enumerate(cases, 1)
    )
    result = test.run_shim(program, payload, root, wasmtime_path=wasmtime)
    test.assertEqual(result.returncode, 0, result.stderr.decode())
    for response in test.responses(result.stdout):
        test.assertTrue(response["result"]["isError"])
    test.assertEqual(records(root / "native.log"), [])
    test.assertEqual(records(root / "wasmtime.log"), [])


def assert_compile_run_fails_closed_and_cleans_artifacts(test):
    root, program, wasmtime = setup(test)
    cases = ["compile-fail", "omit-artifact", "runtime-fail"]
    payload = b"".join(
        request(index, "tools/call", {"name": "lsharp_compile_run", "arguments": {"source": value}})
        for index, value in enumerate(cases, 1)
    )
    result = test.run_shim(program, payload, root, wasmtime_path=wasmtime)
    test.assertEqual(result.returncode, 0, result.stderr.decode())
    responses = test.responses(result.stdout)
    test.assertEqual(len(responses), len(cases))
    for response in responses:
        test.assertTrue(response["result"]["isError"])
    self_records = records(root / "native.log")
    runtime_records = records(root / "wasmtime.log")
    test.assertEqual(len(self_records), 3)
    test.assertEqual(len(runtime_records), 1)
    test.assertIn("compile failure", responses[0]["result"]["content"][0]["text"])
    test.assertIn("artifact", responses[1]["result"]["content"][0]["text"])
    test.assertIn("runtime", responses[2]["result"]["content"][0]["text"])
    for record in self_records:
        test.assertFalse(pathlib.Path(record["source"]).exists())
        test.assertFalse(pathlib.Path(record["wasm"]).exists())


def assert_compile_run_requires_explicit_runtime_without_host_fallback(test):
    root, program, wasmtime = setup(test)
    missing = root / "missing-wasmtime"
    response = call(test, program, missing, root, {"source": "(print 1)\n"})
    test.assertTrue(response["isError"])
    test.assertIn("wasmtime", response["content"][0]["text"])
    test.assertEqual(records(root / "native.log"), [])
