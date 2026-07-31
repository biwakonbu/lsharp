import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def assert_validate_rejects_non_object_manifest_before_native(test):
    """manifest の root は object に限定し、native 実行前に安定して拒否する。"""
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        cases = ["[]", "null", "42"]
        payload = b"".join(
            request(
                index,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {"manifest": manifest},
                },
            )
            for index, manifest in enumerate(cases, 1)
        )

        result = test.run_shim(program, payload, root)

        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response in responses:
            test.assertTrue(response["result"]["isError"])
            test.assertIn("JSON object", response["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())


def assert_validate_rejects_non_object_manifest_file_before_native(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        manifest_file = root / "manifest.json"
        manifest_file.write_text("null\n", encoding="utf-8")
        result = test.run_shim(
            program,
            request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {"manifest_file": str(manifest_file)},
                },
            ),
            root,
        )

        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertTrue(response["result"]["isError"])
        test.assertIn("JSON object", response["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())


def assert_validate_rejects_invalid_emitted_manifest(test):
    cases = (
        ("array", "JSON object"),
        ("null", "JSON object"),
        ("malformed", "manifest"),
        ("missing", "missing field"),
        ("unknown", "unknown field"),
        ("nodes-object", "must be an array"),
    )
    for manifest_mode, expected_message in cases:
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
                        "arguments": {
                            "source": "(defn main [] true)",
                            "include_manifest": True,
                        },
                    },
                ),
                root,
                manifest_mode=manifest_mode,
            )

            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(expected_message, response["result"]["content"][0]["text"])
            test.assertNotIn("Traceback", response["result"]["content"][0]["text"])
