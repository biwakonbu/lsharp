import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def assert_check_rejects_invalid_output(test):
    cases = (
        ("array", "JSON object"),
        ("null", "JSON object"),
        ("malformed", "malformed native JSON"),
        ("missing", "missing field"),
        ("unknown", "unknown field"),
        ("ok-type", "ok must be a boolean"),
        ("diagnostics-type", "diagnostics must be an array"),
    )
    for check_mode, expected_message in cases:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = test.write_fake_program(root)
            result = test.run_shim(
                program,
                request(
                    1,
                    "tools/call",
                    {
                        "name": "lsharp_check",
                        "arguments": {"source": "(defn main [] true)"},
                    },
                ),
                root,
                check_mode=check_mode,
            )

            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(expected_message, response["result"]["content"][0]["text"])
            test.assertNotIn("Traceback", response["result"]["content"][0]["text"])


def assert_check_rejects_invalid_arguments_before_native(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        cases = (
            {"source": "(defn main [] true)", "unknown": True},
            {"source": "(defn main [] true)", "file": "other.ls"},
        )
        payload = b"".join(
            request(index, "tools/call", {"name": "lsharp_check", "arguments": arguments})
            for index, arguments in enumerate(cases, 1)
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response, expected_message in zip(responses, ("未知", "いずれか一つ")):
            test.assertTrue(response["result"]["isError"])
            test.assertIn(expected_message, response["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())
