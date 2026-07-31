import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def write_package(root, directory_name, name, version):
    package = root / ".lsharp" / "packages" / directory_name
    package.mkdir(parents=True)
    (package / "lsharp.toml").write_text(
        f'[project]\nname = "{name}"\nversion = "{version}"\n',
        encoding="utf-8",
    )
    return package


def assert_search_projects_local_packages(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        alpha = write_package(root, "alpha-0.2.0", "alpha", "0.2.0")
        zeta = write_package(root, "zeta-2.0.0", "zeta", "2.0.0")
        payload = b"".join(
            [
                request(1, "tools/list"),
                request(2, "tools/call", {"name": "lsharp_search", "arguments": {"project_dir": str(root), "query": "a"}}),
                request(3, "tools/call", {"name": "lsharp_search", "arguments": {"project_dir": str(root), "query": "zz"}}),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), 3)
        tool = next(tool for tool in responses[0]["result"]["tools"] if tool["name"] == "lsharp_search")
        test.assertEqual(tool["inputSchema"]["oneOf"], [{"required": []}])
        test.assertFalse(tool["inputSchema"].get("additionalProperties", True))
        test.assertEqual(tool["outputSchema"]["required"], ["packages"])
        test.assertFalse(tool["outputSchema"]["additionalProperties"])
        test.assertEqual(
            responses[1]["result"]["structuredContent"],
            {"packages": [{"name": "alpha", "version": "0.2.0", "path": str(alpha)}, {"name": "zeta", "version": "2.0.0", "path": str(zeta)}]},
        )
        test.assertEqual(responses[2]["result"]["structuredContent"], {"packages": []})
        test.assertFalse((root / "native.log").exists())


def assert_search_rejects_invalid_arguments(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        payload = b"".join(
            [
                request(1, "tools/call", {"name": "lsharp_search", "arguments": {"unknown": True}}),
                request(2, "tools/call", {"name": "lsharp_search", "arguments": {"query": 42}}),
                request(3, "tools/call", {"name": "lsharp_search", "arguments": {"project_dir": 42}}),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        for response in responses:
            test.assertTrue(response["result"]["isError"])
        test.assertIn("未知", responses[0]["result"]["content"][0]["text"])
        test.assertIn("query", responses[1]["result"]["content"][0]["text"])
        test.assertIn("project_dir", responses[2]["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())


def write_api_package(root):
    package = root / ".lsharp" / "packages" / "demo-12345678"
    (package / "docs").mkdir(parents=True)
    api = {
        "package": "demo",
        "version": "0.1.0",
        "modules": [{
            "name": "Geometry",
            "doc": None,
            "functions": [{
                "name": "distance",
                "signature": "distance : Point -> Point -> Float",
                "params": [{"name": "left", "type": "Point", "doc": None}],
                "returns": {"type": "Float", "doc": None},
                "doc": "距離",
                "example": None,
            }],
            "types": [{"name": "Point", "kind": "record"}],
        }],
    }
    (package / "docs" / "api.json").write_text(json.dumps(api), encoding="utf-8")
    return package, api


def assert_package_api_projects_local_api_json(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        package, api = write_api_package(root)
        payload = b"".join([
            request(1, "tools/list"),
            request(2, "tools/call", {"name": "lsharp_package_api", "arguments": {"project_dir": str(root), "name": "demo"}}),
        ])
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        tool = next(tool for tool in responses[0]["result"]["tools"] if tool["name"] == "lsharp_package_api")
        test.assertEqual(tool["inputSchema"]["oneOf"], [{"required": ["name"]}])
        test.assertEqual(tool["inputSchema"]["properties"]["name"]["minLength"], 1)
        test.assertFalse(tool["inputSchema"]["additionalProperties"])
        test.assertEqual(tool["outputSchema"]["required"], ["package", "version", "modules"])
        test.assertFalse(tool["outputSchema"]["additionalProperties"])
        test.assertEqual(responses[1]["result"]["structuredContent"], api)
        test.assertFalse((root / "native.log").exists())


def assert_package_api_rejects_invalid_arguments(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        write_api_package(root)
        missing = root / ".lsharp" / "packages" / "missing-1.0.0"
        missing.mkdir(parents=True)
        invalid = root / ".lsharp" / "packages" / "invalid-1.0.0" / "docs"
        invalid.mkdir(parents=True)
        (invalid / "api.json").write_text("[]", encoding="utf-8")
        malformed = root / ".lsharp" / "packages" / "malformed-1.0.0" / "docs"
        malformed.mkdir(parents=True)
        (malformed / "api.json").write_text(
            json.dumps({
                "package": "malformed",
                "version": "0.1.0",
                "modules": [{
                    "name": "Geometry",
                    "doc": None,
                    "functions": [],
                    "types": [],
                    "extra": True,
                }],
            }),
            encoding="utf-8",
        )
        cases = [
            {"unknown": True},
            {"name": ""},
            {"name": 42},
            {"name": "demo", "project_dir": 42},
            {"name": "missing", "project_dir": str(root)},
            {"name": "invalid", "project_dir": str(root)},
            {"name": "malformed", "project_dir": str(root)},
        ]
        payload = b"".join(request(index, "tools/call", {"name": "lsharp_package_api", "arguments": arguments}) for index, arguments in enumerate(cases, 1))
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response in responses:
            test.assertTrue(response["result"]["isError"])
        test.assertIn("未知", responses[0]["result"]["content"][0]["text"])
        test.assertIn("name", responses[1]["result"]["content"][0]["text"])
        test.assertIn("name", responses[2]["result"]["content"][0]["text"])
        test.assertIn("project_dir", responses[3]["result"]["content"][0]["text"])
        test.assertIn("api.json", responses[4]["result"]["content"][0]["text"])
        test.assertFalse((missing / "docs" / "api.json").exists())
        test.assertIn("root", responses[5]["result"]["content"][0]["text"])
        test.assertIn("modules[0].extra", responses[6]["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())
