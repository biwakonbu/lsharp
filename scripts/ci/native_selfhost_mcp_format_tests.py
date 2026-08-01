import pathlib
import tempfile

from native_selfhost_mcp_check_tests import request


def assert_format_output_schema_is_closed(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        result = test.run_shim(program, request(1, "tools/list"), root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        tool = next(tool for tool in response["result"]["tools"] if tool["name"] == "lsharp_format")
        output_schema = tool["outputSchema"]
        test.assertFalse(output_schema["additionalProperties"])
        test.assertEqual(output_schema["required"], ["formatted"])
        test.assertEqual(output_schema["properties"], {"formatted": {"type": "string"}})


def assert_format_rejects_native_failures(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        result = test.run_shim(
            program,
            request(
                1,
                "tools/call",
                {
                    "name": "lsharp_format",
                    "arguments": {"source": "(defn main [] true)"},
                },
            ),
            root,
            format_mode="nonzero",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertTrue(response["result"]["isError"])
        message = response["result"]["content"][0]["text"]
        test.assertIn("status 7", message)
        test.assertIn("format diagnostic", message)
        test.assertNotIn("Traceback", message)
