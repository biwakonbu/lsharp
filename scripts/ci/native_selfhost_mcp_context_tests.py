import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def write_context_fixture(root):
    (root / "src").mkdir()
    (root / "src" / "main.ls").write_text("(defn main [] true)\n", encoding="utf-8")
    (root / "lsharp.toml").write_text(
        """[project]
name = "context-demo"
version = "1.2.3"
description = "context fixture"

[project.exports]
modules = ["Demo", "Demo.Util"]

[dependencies]
math = "1.0.0"

[dependencies.local]
path = "./libs/local"

[dependencies.gitlib]
git = "https://example.invalid/gitlib.git"
branch = "main"
""",
        encoding="utf-8",
    )
    local = root / "libs" / "local"
    local.mkdir(parents=True)
    (root / ".lsharp" / "packages" / "zeta-2.0.0").mkdir(parents=True)
    (root / ".lsharp" / "packages" / "zeta-2.0.0" / "lsharp.toml").write_text(
        '[project]\nname = "zeta"\nversion = "2.0.0"\n', encoding="utf-8"
    )
    return local


def assert_project_context_projects_local_metadata(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        write_context_fixture(root)
        payload = b"".join(
            [
                request(1, "tools/list"),
                request(
                    2,
                    "tools/call",
                    {"name": "lsharp_project_context", "arguments": {"project_dir": str(root)}},
                ),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        tool = next(
            tool for tool in responses[0]["result"]["tools"] if tool["name"] == "lsharp_project_context"
        )
        test.assertEqual(tool["inputSchema"]["oneOf"], [{"required": []}])
        test.assertFalse(tool["inputSchema"]["additionalProperties"])
        test.assertEqual(tool["outputSchema"]["required"], ["project", "dependencies", "installedPackages"])
        test.assertFalse(tool["outputSchema"]["additionalProperties"])
        context = responses[1]["result"]["structuredContent"]
        test.assertEqual(
            context["project"],
            {
                "name": "context-demo",
                "version": "1.2.3",
                "description": "context fixture",
                "exports": ["Demo", "Demo.Util"],
            },
        )
        test.assertEqual(
            context["dependencies"],
            [
                {"name": "gitlib", "source": "git", "git": "https://example.invalid/gitlib.git", "branch": "main", "tag": None},
                {"name": "local", "source": "path", "path": f"{root}/./libs/local"},
                {"name": "math", "source": "registry", "version": "1.0.0"},
            ],
        )
        test.assertEqual(context["installedPackages"][0]["name"], "zeta")
        test.assertFalse((root / "native.log").exists())


def assert_project_context_rejects_invalid_arguments(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        payload = b"".join(
            [
                request(1, "tools/call", {"name": "lsharp_project_context", "arguments": {"unknown": True}}),
                request(2, "tools/call", {"name": "lsharp_project_context", "arguments": {"project_dir": 42}}),
            ]
        )
        result = test.run_shim(program, payload, root)
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        for response in responses:
            test.assertTrue(response["result"]["isError"])
        test.assertIn("未知", responses[0]["result"]["content"][0]["text"])
        test.assertIn("project_dir", responses[1]["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())


def assert_project_context_rejects_ambiguous_dependency_sources(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        (root / "src").mkdir()
        (root / "src" / "main.ls").write_text("(defn main [] true)\n", encoding="utf-8")
        cases = [
            (
                """[dependencies.bad]
path = "./libs/bad"
git = "https://example.invalid/bad.git"
""",
                "path または git",
            ),
            (
                """[dependencies.bad]
path = "./libs/bad"
branch = "main"
""",
                "path には",
            ),
            (
                """[dependencies.bad]
path = "./libs/bad"
checksum = "sha256:bad"
""",
                "未知",
            ),
            (
                """[dependencies.bad]
git = ""
""",
                "git は空でない",
            ),
        ]
        for case_index, (dependency, expected_message) in enumerate(cases, start=1):
            (root / "lsharp.toml").write_text(
                """[project]
name = "ambiguous-context"
entry = "src/main.ls"

""" + dependency,
                encoding="utf-8",
            )
            result = test.run_shim(
                program,
                request(
                    case_index,
                    "tools/call",
                    {"name": "lsharp_project_context", "arguments": {"project_dir": str(root)}},
                ),
                root,
            )
            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            error_text = response["result"]["content"][0]["text"]
            test.assertIn("dependencies.bad", error_text)
            test.assertIn(expected_message, error_text)
        test.assertFalse((root / "native.log").exists())
