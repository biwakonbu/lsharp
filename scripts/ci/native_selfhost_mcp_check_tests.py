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
        ("migration-missing", "missing field: code"),
        ("migration-code", "code must be one of"),
        ("migration-range", "range must be an object"),
        ("migration-position", "line must be a non-negative integer"),
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


def assert_check_accepts_valid_migration_diagnostics(test):
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
            check_mode="migration-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        test.assertEqual(
            response["result"]["structuredContent"]["migrationDiagnostics"][0],
            {
                "code": "LS2001",
                "owner": "main",
                "selectedSemantics": "legacy-example-truthiness",
                "disposition": "assertion",
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 1},
                },
                "message": "legacy example",
            },
        )


def assert_check_projects_legacy_clean_summary(test):
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
                    "arguments": {"source": "(defn main [] 42)"},
                },
            ),
            root,
            check_mode="legacy-clean",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        test.assertEqual(
            response["result"]["structuredContent"],
            {"ok": True, "diagnostics": [], "migrationDiagnostics": []},
        )


def assert_check_rejects_legacy_unsupported_summary(test):
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
                    "arguments": {"source": "(defn main [] 42)"},
                },
            ),
            root,
            check_mode="legacy-unsupported",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertTrue(response["result"]["isError"])
        test.assertIn(
            "legacy diagnostics cannot satisfy the structured MCP output",
            response["result"]["content"][0]["text"],
        )


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


def assert_source_input_schema_requires_non_empty_strings(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        result = test.run_shim(program, request(1, "tools/list"), root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        tools = {
            tool["name"]: tool
            for tool in test.responses(result.stdout)[0]["result"]["tools"]
        }
        source_tools = (
            "lsharp_hover",
            "lsharp_definition",
            "lsharp_references",
            "lsharp_completion",
            "lsharp_check",
            "lsharp_validate",
            "lsharp_format",
            "lsharp_compile_run",
        )
        for name in source_tools:
            source_schema = tools[name]["inputSchema"]["properties"]["source"]
            test.assertEqual(source_schema["type"], "string")
            test.assertEqual(source_schema["minLength"], 1)


def assert_check_rejects_blank_source_before_native(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        cases = ("", " \t\n")
        payload = b"".join(
            request(
                index,
                "tools/call",
                {"name": "lsharp_check", "arguments": {"source": source}},
            )
            for index, source in enumerate(cases, 1)
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response in responses:
            test.assertTrue(response["result"]["isError"])
            test.assertIn("空でない", response["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())
