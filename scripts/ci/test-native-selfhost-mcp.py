#!/usr/bin/env python3

import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tempfile
import textwrap
import unittest


SCRIPTS_DIR = pathlib.Path(__file__).resolve().parent.parent
SHIM = SCRIPTS_DIR / "native-selfhost-mcp.py"


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


class NativeSelfhostMcpTest(unittest.TestCase):
    def write_fake_program(self, root):
        program = root / "program.native"
        program.write_text(
            textwrap.dedent(
                f"""\
                #!{sys.executable}
                import json
                import os
                import pathlib
                import sys

                log = pathlib.Path(os.environ["FAKE_NATIVE_LOG"])
                with log.open("a", encoding="utf-8") as stream:
                    stream.write(json.dumps(sys.argv[1:]) + "\\n")
                args = sys.argv[1:]
                if args[:1] == ["check"]:
                    print(json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": []}}))
                    raise SystemExit(0)
                if args[:1] == ["validate"]:
                    if "--emit-manifest" in args:
                        output = pathlib.Path(args[args.index("--emit-manifest") + 1])
                        output.parent.mkdir(parents=True, exist_ok=True)
                        output.write_text(json.dumps({{"schema_version": 1, "nodes": [], "evidence": [], "edges": []}}), encoding="utf-8")
                    print(json.dumps({{"status": "unknown", "trace_gaps": [], "open_questions": 0, "independent_reviews": 0, "contradicting_observations": 0, "stale_reviews": 0, "stale_evidence": 0}}))
                    raise SystemExit(2)
                if args[:1] == ["fmt"]:
                    print("(formatted)")
                    raise SystemExit(0)
                sys.stderr.write("unexpected native arguments: " + repr(args) + "\\n")
                raise SystemExit(91)
                """
            ),
            encoding="utf-8",
        )
        os.chmod(program, 0o755)
        return program

    def run_shim(self, program, payload, root):
        environment = os.environ.copy()
        environment["FAKE_NATIVE_LOG"] = str(root / "native.log")
        return subprocess.run(
            [sys.executable, str(SHIM), "--program", str(program)],
            input=payload,
            capture_output=True,
            env=environment,
            check=False,
        )

    def responses(self, output):
        return [json.loads(line) for line in output.decode().splitlines() if line]

    def test_initialize_tools_and_supported_calls_stay_native_only(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            source = root / "input.ls"
            source.write_text("(defn main [] true)\n", encoding="utf-8")
            payload = b"".join(
                [
                    request(1, "initialize"),
                    request(2, "tools/list"),
                    request(3, "tools/call", {"name": "lsharp_check", "arguments": {"source": source.read_text()}}),
                    request(4, "tools/call", {"name": "lsharp_validate", "arguments": {"file": str(source)}}),
                    request(5, "tools/call", {"name": "lsharp_format", "arguments": {"source": source.read_text()}}),
                ]
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            responses = self.responses(result.stdout)
            self.assertEqual(len(responses), 5)
            self.assertEqual(responses[0]["result"]["protocolVersion"], "2025-11-25")
            tool_names = {tool["name"] for tool in responses[1]["result"]["tools"]}
            self.assertEqual(tool_names, {"lsharp_check", "lsharp_validate", "lsharp_format"})
            check_schema = next(
                tool for tool in responses[1]["result"]["tools"] if tool["name"] == "lsharp_check"
            )["inputSchema"]
            self.assertEqual(
                check_schema["oneOf"], [{"required": ["source"]}, {"required": ["file"]}]
            )
            validate_schema = next(
                tool for tool in responses[1]["result"]["tools"] if tool["name"] == "lsharp_validate"
            )["inputSchema"]
            self.assertEqual(
                validate_schema["oneOf"],
                [
                    {"required": ["source"]},
                    {"required": ["file"]},
                    {"required": ["manifest"]},
                    {"required": ["manifest_file"]},
                ],
            )
            self.assertEqual(
                validate_schema["dependentRequired"],
                {
                    "trust_store": ["review_lifecycle"],
                    "review_lifecycle": ["trust_store"],
                    "review_subject_digest": [
                        "review_source_commit",
                        "review_artifact_digest",
                        "review_now",
                    ],
                    "review_source_commit": [
                        "review_subject_digest",
                        "review_artifact_digest",
                        "review_now",
                    ],
                    "review_artifact_digest": [
                        "review_subject_digest",
                        "review_source_commit",
                        "review_now",
                    ],
                    "review_now": [
                        "review_subject_digest",
                        "review_source_commit",
                        "review_artifact_digest",
                    ],
                },
            )
            self.assertFalse(validate_schema["additionalProperties"])
            self.assertEqual(responses[2]["result"]["structuredContent"]["ok"], True)
            self.assertEqual(responses[3]["result"]["structuredContent"]["status"], "unknown")
            self.assertEqual(responses[4]["result"]["structuredContent"], {"formatted": "(formatted)\n"})
            calls = [json.loads(line) for line in (root / "native.log").read_text().splitlines()]
            self.assertEqual(calls[0][0], "check")
            self.assertEqual(calls[0][2:4], ["--format", "json"])
            self.assertEqual(calls[1][0:2], ["validate", "--source"])
            self.assertEqual(calls[2][0], "fmt")

    def test_validate_forwards_explicit_identity_and_manifest_request(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {
                        "source": "(defn main [] true)",
                        "include_manifest": True,
                        "review_subject_digest": "sha256:subject",
                        "review_source_commit": "a" * 40,
                        "review_artifact_digest": "sha256:artifact",
                        "review_trust_store_digest": "sha256:trust",
                        "review_lifecycle_digest": "sha256:lifecycle",
                        "review_now": "2026-08-01T00:00:00Z",
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            calls = [json.loads(line) for line in (root / "native.log").read_text().splitlines()]
            self.assertEqual(
                calls[0][0:4],
                ["validate", "--source", calls[0][2], "--format"],
            )
            self.assertIn("--review-subject-digest", calls[0])
            self.assertIn("--review-lifecycle-digest", calls[0])
            self.assertIn("--emit-manifest", calls[0])
            self.assertEqual(self.responses(result.stdout)[0]["result"]["isError"], False)

    def test_partial_review_identity_is_rejected_before_native_execution(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {
                        "source": "(defn main [] true)",
                        "review_subject_digest": "sha256:subject",
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]
            self.assertTrue(response["result"]["isError"])
            self.assertIn("review identity requires", response["result"]["content"][0]["text"])
            self.assertFalse((root / "native.log").exists())

    def test_unknown_validate_argument_is_rejected_before_native_execution(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {
                        "source": "(defn main [] true)",
                        "unexpected": True,
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]
            self.assertTrue(response["result"]["isError"])
            self.assertIn("未知", response["result"]["content"][0]["text"])
            self.assertFalse((root / "native.log").exists())

    def test_provider_paths_are_hashed_and_forwarded_to_native(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            trust_store = root / "trust.json"
            lifecycle = root / "lifecycle.json"
            trust_bytes = b"trust snapshot\n"
            lifecycle_bytes = b"lifecycle snapshot\n"
            trust_store.write_bytes(trust_bytes)
            lifecycle.write_bytes(lifecycle_bytes)
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {
                        "source": "(defn main [] true)",
                        "trust_store": str(trust_store),
                        "review_lifecycle": str(lifecycle),
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]
            self.assertFalse(response["result"]["isError"])
            calls = [json.loads(line) for line in (root / "native.log").read_text().splitlines()]
            self.assertEqual(len(calls), 1)
            command = calls[0]
            self.assertIn(
                ["--review-trust-store-digest", f"sha256:{hashlib.sha256(trust_bytes).hexdigest()}"],
                [command[index : index + 2] for index in range(len(command) - 1)],
            )
            self.assertIn(
                ["--review-lifecycle-digest", f"sha256:{hashlib.sha256(lifecycle_bytes).hexdigest()}"],
                [command[index : index + 2] for index in range(len(command) - 1)],
            )

    def test_provider_digest_mismatch_is_rejected_before_native_execution(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            trust_store = root / "trust.json"
            lifecycle = root / "lifecycle.json"
            trust_store.write_bytes(b"trust snapshot\n")
            lifecycle.write_bytes(b"lifecycle snapshot\n")
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {
                        "source": "(defn main [] true)",
                        "trust_store": str(trust_store),
                        "review_lifecycle": str(lifecycle),
                        "review_trust_store_digest": "sha256:" + "0" * 64,
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]
            self.assertTrue(response["result"]["isError"])
            self.assertIn("digest mismatch", response["result"]["content"][0]["text"])
            self.assertFalse((root / "native.log").exists())

    def test_matching_provider_digests_are_forwarded_once(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            trust_store = root / "trust.json"
            lifecycle = root / "lifecycle.json"
            trust_store.write_bytes(b"trust snapshot\n")
            lifecycle.write_bytes(b"lifecycle snapshot\n")
            trust_digest = f"sha256:{hashlib.sha256(trust_store.read_bytes()).hexdigest()}"
            lifecycle_digest = f"sha256:{hashlib.sha256(lifecycle.read_bytes()).hexdigest()}"
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {
                        "source": "(defn main [] true)",
                        "trust_store": str(trust_store),
                        "review_lifecycle": str(lifecycle),
                        "review_trust_store_digest": trust_digest,
                        "review_lifecycle_digest": lifecycle_digest,
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertFalse(self.responses(result.stdout)[0]["result"]["isError"])
            command = json.loads((root / "native.log").read_text().splitlines()[0])
            self.assertEqual(command.count("--review-trust-store-digest"), 1)
            self.assertEqual(command.count("--review-lifecycle-digest"), 1)
            self.assertEqual(command[command.index("--review-trust-store-digest") + 1], trust_digest)
            self.assertEqual(command[command.index("--review-lifecycle-digest") + 1], lifecycle_digest)

    def test_provider_paths_require_both_existing_non_empty_files(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            trust_store = root / "trust.json"
            lifecycle = root / "lifecycle.json"
            trust_store.write_bytes(b"trust snapshot\n")
            lifecycle.write_bytes(b"lifecycle snapshot\n")

            cases = [
                (
                    {"trust_store": str(trust_store)},
                    "同時指定",
                ),
                (
                    {"trust_store": str(trust_store), "review_lifecycle": str(root / "missing.json")},
                    "見つかりません",
                ),
            ]
            snapshot_directory = root / "snapshot-directory"
            snapshot_directory.mkdir()
            cases.append(
                (
                    {"trust_store": str(snapshot_directory), "review_lifecycle": str(lifecycle)},
                    "見つかりません",
                )
            )
            trust_link = root / "trust-link.json"
            trust_link.symlink_to(trust_store)
            cases.append(
                (
                    {"trust_store": str(trust_link), "review_lifecycle": str(lifecycle)},
                    "見つかりません",
                )
            )
            lifecycle.write_bytes(b"")
            cases.append(
                (
                    {"trust_store": str(trust_store), "review_lifecycle": str(lifecycle)},
                    "empty",
                )
            )
            for provider_arguments, message in cases:
                arguments = {"source": "(defn main [] true)", **provider_arguments}
                result = self.run_shim(
                    program,
                    request(1, "tools/call", {"name": "lsharp_validate", "arguments": arguments}),
                    root,
                )

                self.assertEqual(result.returncode, 0, result.stderr.decode())
                response = self.responses(result.stdout)[0]
                self.assertTrue(response["result"]["isError"])
                self.assertIn(message, response["result"]["content"][0]["text"])
                self.assertFalse((root / "native.log").exists())

    def test_malformed_json_or_missing_program_is_fail_closed(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)

            malformed = self.run_shim(program, b"not-json\n", root)
            self.assertNotEqual(malformed.returncode, 0)
            self.assertIn(b"invalid JSON", malformed.stderr)
            self.assertEqual(malformed.stdout, b"")

            missing = self.run_shim(root / "missing", request(1, "ping"), root)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn(b"not a regular executable", missing.stderr)
            self.assertEqual(missing.stdout, b"")


if __name__ == "__main__":
    unittest.main(verbosity=2)
