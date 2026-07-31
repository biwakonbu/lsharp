import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def assert_errors_lookup(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        codes = ["LS1001", "E0001", "E0002", "E0003", "E0004", "E0005", "LS9999"]
        payload = b"".join(
            request(index, "tools/call", {"name": "lsharp_errors", "arguments": {"error_code": code}})
            for index, code in enumerate(codes, 1)
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(codes))
        known = responses[0]["result"]["structuredContent"]
        test.assertEqual(known["code"], "LS1001")
        test.assertEqual(known["legacy_code"], "E0001")
        test.assertEqual(known["name"], "undefined-variable")
        test.assertEqual(known["description"], "未定義の識別子です")
        test.assertEqual(known["detail"], "type checker が現在 scope に存在しない変数または関数を参照しました。")
        test.assertEqual(known["fix"], "定義、import、module path、綴りを確認してください。")
        test.assertEqual(known["doc"], "docs/guides/error-reference.md")
        expected_aliases = {
            "E0001": ("LS1001", "E0001", "undefined-variable"),
            "E0002": ("LS1002", "E0002", "type-mismatch"),
            "E0003": ("LS1002", "E0002", "type-mismatch"),
            "E0004": ("LS1004", "E0004", "arity-mismatch"),
            "E0005": ("LS1003", "E0005", "infinite-type"),
        }
        for index, code in enumerate(codes[1:-1], 1):
            actual = responses[index]["result"]["structuredContent"]
            test.assertEqual((actual["code"], actual["legacy_code"], actual["name"]), expected_aliases[code])
        test.assertEqual(
            responses[-1]["result"]["structuredContent"],
            {"code": "LS9999", "name": "unknown", "description": "未知のエラーコードです", "fix": "最新版ドキュメントを確認してください", "doc": "docs/guides/error-reference.md"},
        )
        test.assertFalse((root / "native.log").exists())


def assert_errors_reject_invalid_arguments(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        payload = b"".join(
            [
                request(1, "tools/call", {"name": "lsharp_errors", "arguments": {}}),
                request(2, "tools/call", {"name": "lsharp_errors", "arguments": {"error_code": "LS1001", "unexpected": True}}),
                request(3, "tools/call", {"name": "lsharp_errors", "arguments": {"error_code": ""}}),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertTrue(responses[0]["result"]["isError"])
        test.assertIn("error_code", responses[0]["result"]["content"][0]["text"])
        test.assertTrue(responses[1]["result"]["isError"])
        test.assertIn("未知", responses[1]["result"]["content"][0]["text"])
        test.assertTrue(responses[2]["result"]["isError"])
        test.assertIn("空でない", responses[2]["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())
