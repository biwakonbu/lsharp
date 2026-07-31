import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def assert_stdlib_api_projects_generated_metadata(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        artifact_path = pathlib.Path(__file__).resolve().parents[2] / "stdlib" / "api.json"
        expected = json.loads(artifact_path.read_text(encoding="utf-8"))
        payload = b"".join(
            [
                request(1, "tools/list"),
                request(2, "tools/call", {"name": "lsharp_stdlib_api", "arguments": {}}),
                request(3, "tools/call", {"name": "lsharp_stdlib_api", "arguments": {"module": "List"}}),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        tool = next(tool for tool in responses[0]["result"]["tools"] if tool["name"] == "lsharp_stdlib_api")
        test.assertEqual(tool["inputSchema"]["oneOf"], [{"required": []}])
        test.assertEqual(tool["inputSchema"]["properties"]["module"]["minLength"], 1)
        test.assertFalse(tool["inputSchema"]["additionalProperties"])
        test.assertEqual(tool["outputSchema"]["required"], ["package", "version", "modules"])
        test.assertFalse(tool["outputSchema"]["additionalProperties"])
        test.assertEqual(responses[1]["result"]["structuredContent"], expected)
        selected = responses[2]["result"]["structuredContent"]
        test.assertEqual(selected["package"], "stdlib")
        test.assertEqual([module["name"] for module in selected["modules"]], ["List"])
        test.assertTrue(selected["modules"][0]["functions"])
        test.assertFalse((root / "native.log").exists())


def assert_stdlib_api_rejects_invalid_arguments(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        cases = [
            {"unknown": True},
            {"module": ""},
            {"module": 42},
        ]
        payload = b"".join(
            request(index, "tools/call", {"name": "lsharp_stdlib_api", "arguments": arguments})
            for index, arguments in enumerate(cases, 1)
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response in responses:
            test.assertTrue(response["result"]["isError"])
        test.assertIn("未知", responses[0]["result"]["content"][0]["text"])
        test.assertIn("module", responses[1]["result"]["content"][0]["text"])
        test.assertIn("module", responses[2]["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())
