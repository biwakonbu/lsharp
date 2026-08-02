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


def assert_search_ignores_non_directory_symlinks(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        valid = write_package(root, "valid-1.0.0", "valid", "1.0.0")
        external_file = root / "outside.txt"
        external_file.write_text("not a package\n", encoding="utf-8")
        packages = root / ".lsharp" / "packages"
        (packages / "file-link-1.0.0").symlink_to(external_file)
        (packages / "dangling-1.0.0").symlink_to(root / "missing-package")
        directory_link = packages / "linked-1.0.0"
        directory_link.symlink_to(valid, target_is_directory=True)

        payload = b"".join(
            [
                request(
                    1,
                    "tools/call",
                    {"name": "lsharp_search", "arguments": {"project_dir": str(root)}},
                ),
                request(
                    2,
                    "tools/call",
                    {
                        "name": "lsharp_package_api",
                        "arguments": {"project_dir": str(root), "name": "file-link"},
                    },
                ),
                request(
                    3,
                    "tools/call",
                    {
                        "name": "lsharp_package_api",
                        "arguments": {"project_dir": str(root), "name": "dangling"},
                    },
                ),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), 3)
        response = responses[0]
        test.assertFalse(response["result"]["isError"])
        packages_result = response["result"]["structuredContent"]["packages"]
        test.assertEqual(
            packages_result,
            [
                {"name": "valid", "version": "1.0.0", "path": str(directory_link)},
                {"name": "valid", "version": "1.0.0", "path": str(valid)},
            ],
        )
        for response in responses[1:]:
            test.assertTrue(response["result"]["isError"])
            test.assertIn("見つかりません", response["result"]["content"][0]["text"])
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


def assert_package_api_generates_from_native_doc(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        package = write_package(root, "demo-1.0.0", "demo", "1.0.0")
        source = package / "src" / "Geometry.ls"
        source.parent.mkdir(parents=True)
        source.write_text("(module Geometry)\n(defn distance [left] left)\n", encoding="utf-8")
        native_document = {
            "module": "module-Geometry",
            "functions": [{
                "name": "distance",
                "arity": 1,
                "params": [{"name": "left", "type": "Point", "doc": "point"}],
                "returns": {"type": "Float", "doc": "distance"},
                "doc": "距離",
                "example": "(distance p)",
            }],
            "types": [{"name": "Point", "kind": "recorddef"}],
            "html": {
                "title": "module-Geometry",
                "sections": [{"id": "functions", "count": 1}, {"id": "types", "count": 1}],
            },
        }
        payload = request(
            1,
            "tools/call",
            {"name": "lsharp_package_api", "arguments": {"project_dir": str(root), "name": "demo"}},
        )
        result = test.run_shim(program, payload, root, doc_output=native_document)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        test.assertEqual(
            response["result"]["structuredContent"],
            {
                "package": "demo",
                "version": "1.0.0",
                "modules": [{
                    "name": "Geometry",
                    "doc": None,
                    "functions": [{
                        "name": "distance",
                        "signature": "Point -> Float",
                        "params": [{"name": "left", "type": "Point", "doc": "point"}],
                        "returns": {"type": "Float", "doc": "distance"},
                        "doc": "距離",
                        "example": "(distance p)",
                    }],
                    "types": [{"name": "Point", "kind": "record"}],
                }],
            },
        )
        test.assertFalse((package / "docs" / "api.json").exists())
        test.assertEqual(
            json.loads((root / "native.log").read_text(encoding="utf-8")),
            ["doc", str(source), "--json"],
        )


def assert_package_api_rejects_identity_mismatch(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        package = write_package(root, "demo-1.0.0", "demo", "1.0.0")
        api_path = package / "docs" / "api.json"
        api_path.parent.mkdir(parents=True)
        cases = (("other", "1.0.0"), ("demo", "2.0.0"))

        responses = []
        for index, (package_name, version) in enumerate(cases, 1):
            api_path.write_text(
                json.dumps({"package": package_name, "version": version, "modules": []}),
                encoding="utf-8",
            )
            result = test.run_shim(
                program,
                request(
                    index,
                    "tools/call",
                    {
                        "name": "lsharp_package_api",
                        "arguments": {"project_dir": str(root), "name": "demo"},
                    },
                ),
                root,
            )
            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            responses.append(response)

        for response in responses:
            test.assertTrue(response["result"]["isError"])
            message = response["result"]["content"][0]["text"]
            test.assertIn("api.json identity mismatch", message)
            test.assertIn("expected package 'demo' version '1.0.0'", message)
        test.assertFalse((root / "native.log").exists())


def assert_package_api_rejects_malformed_native_doc(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        package = write_package(root, "demo-1.0.0", "demo", "1.0.0")
        source = package / "src" / "Geometry.ls"
        source.parent.mkdir(parents=True)
        source.write_text("(module Geometry)\n", encoding="utf-8")
        payload = request(
            1,
            "tools/call",
            {"name": "lsharp_package_api", "arguments": {"project_dir": str(root), "name": "demo"}},
        )
        result = test.run_shim(program, payload, root, doc_output={"module": "module-Geometry"})
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertTrue(response["result"]["isError"])
        test.assertIn("native doc", response["result"]["content"][0]["text"])
        test.assertIn("missing required keys", response["result"]["content"][0]["text"])
        test.assertFalse((package / "docs" / "api.json").exists())


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
        duplicate = root / ".lsharp" / "packages" / "duplicate-1.0.0" / "docs"
        duplicate.mkdir(parents=True)
        (duplicate / "api.json").write_text(
            '{"package":"duplicate","package":"duplicate",'
            '"version":"0.1.0","modules":[]}',
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
            {"name": "duplicate", "project_dir": str(root)},
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
        test.assertIn("duplicate JSON object key: package", responses[7]["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())


def assert_install_is_explicit_external_boundary(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        metadata = {
            root / "lsharp.toml": b'[project]\nname = "demo"\n',
            root / ".lsharp" / "lock.toml": b"sentinel-lock\n",
            root / ".lsharp" / "module-index.json": b"sentinel-index\n",
        }
        for path, contents in metadata.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(contents)

        payload = b"".join(
            [
                request(1, "tools/list"),
                request(
                    2,
                    "tools/call",
                    {
                        "name": "lsharp_install",
                        "arguments": {"name": "demo", "project_dir": str(root)},
                    },
                ),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        install_tool = next(
            tool for tool in responses[0]["result"]["tools"] if tool["name"] == "lsharp_install"
        )
        test.assertEqual(install_tool["inputSchema"]["oneOf"], [{"required": ["name"]}])
        test.assertFalse(install_tool["inputSchema"]["additionalProperties"])
        response = responses[1]
        test.assertTrue(response["result"]["isError"])
        test.assertEqual(
            response["result"]["content"][0]["text"],
            "native MCP package installation requires an explicit external provider adapter",
        )
        for path, contents in metadata.items():
            test.assertEqual(path.read_bytes(), contents)
        test.assertFalse((root / "native.log").exists())
