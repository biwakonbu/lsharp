import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def assert_validate_rejects_invalid_report(test):
    cases = (
        ("array", "JSON object"),
        ("null", "JSON object"),
        ("malformed", "malformed native JSON"),
        ("missing", "missing field"),
        ("unknown", "unknown field"),
        ("status", "status"),
        ("count-bool", "open_questions"),
    )
    for report_mode, expected_message in cases:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = test.write_fake_program(root)
            result = test.run_shim(
                program,
                request(
                    1,
                    "tools/call",
                    {
                        "name": "lsharp_validate",
                        "arguments": {"source": "(defn main [] true)"},
                    },
                ),
                root,
                report_mode=report_mode,
            )

            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(expected_message, response["result"]["content"][0]["text"])
            test.assertNotIn("Traceback", response["result"]["content"][0]["text"])
