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
from native_selfhost_mcp_error_tests import assert_errors_lookup, assert_errors_reject_invalid_arguments
from native_selfhost_mcp_package_tests import assert_package_api_generates_from_native_doc, assert_package_api_projects_local_api_json, assert_package_api_rejects_invalid_arguments, assert_package_api_rejects_malformed_native_doc, assert_search_projects_local_packages, assert_search_rejects_invalid_arguments
from native_selfhost_mcp_context_tests import assert_project_context_projects_local_metadata, assert_project_context_rejects_invalid_arguments
from native_selfhost_mcp_stdlib_tests import assert_stdlib_api_generates_from_native_doc, assert_stdlib_api_projects_generated_metadata, assert_stdlib_api_rejects_invalid_arguments, assert_stdlib_api_rejects_malformed_native_doc
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
                    def arg_value(flag):
                        try:
                            return args[args.index(flag) + 1]
                        except (ValueError, IndexError):
                            return None

                    identity = None
                    if "--review-subject-digest" in args:
                        identity = {{
                            "subject_digest": arg_value("--review-subject-digest"),
                            "source_commit": arg_value("--review-source-commit"),
                            "artifact_digest": arg_value("--review-artifact-digest"),
                            "trust_store_digest": arg_value("--review-trust-store-digest"),
                            "lifecycle_digest": arg_value("--review-lifecycle-digest"),
                            "now": arg_value("--review-now"),
                        }}
                    mode = os.environ.get("FAKE_NATIVE_IDENTITY_MODE", "match")
                    report_identity = dict(identity) if identity is not None else None
                    manifest_identity = dict(identity) if identity is not None else None
                    if mode == "missing":
                        report_identity = None
                        manifest_identity = None
                    elif mode == "unexpected":
                        implicit_identity = {{
                            "subject_digest": "sha256:implicit",
                            "source_commit": "b" * 40,
                            "artifact_digest": "sha256:implicit-artifact",
                            "trust_store_digest": None,
                            "lifecycle_digest": None,
                            "now": "2026-08-01T00:00:00Z",
                        }}
                        report_identity = dict(implicit_identity)
                        manifest_identity = dict(implicit_identity)
                    elif mode == "report-mismatch":
                        report_identity["subject_digest"] = "sha256:wrong"
                    elif mode == "manifest-mismatch":
                        manifest_identity["subject_digest"] = "sha256:wrong"
                    report = {{
                        "status": "unknown",
                        "trace_gaps": [],
                        "open_questions": 0,
                        "independent_reviews": 0,
                        "contradicting_observations": 0,
                        "stale_reviews": 0,
                        "stale_evidence": 0,
                    }}
                    if report_identity is not None:
                        report["review_evidence_identity"] = report_identity
                    if "--emit-manifest" in args:
                        output = pathlib.Path(args[args.index("--emit-manifest") + 1])
                        output.parent.mkdir(parents=True, exist_ok=True)
                        manifest = {{"schema_version": 1, "nodes": [], "evidence": [], "edges": []}}
                        if manifest_identity is not None:
                            manifest["review_evidence_identity"] = manifest_identity
                        output.write_text(json.dumps(manifest), encoding="utf-8")
                    print(json.dumps(report))
                    raise SystemExit(2)
                if args[:1] == ["fmt"]:
                    print("(formatted)")
                    raise SystemExit(0)
                if args[:1] == ["doc"]:
                    output = os.environ.get("FAKE_NATIVE_DOC_OUTPUT")
                    if output is None:
                        sys.stderr.write("unexpected native doc invocation\\n")
                        raise SystemExit(91)
                    print(output)
                    raise SystemExit(0)
                sys.stderr.write("unexpected native arguments: " + repr(args) + "\\n")
                raise SystemExit(91)
                """
            ),
            encoding="utf-8",
        )
        os.chmod(program, 0o755)
        return program

    def run_shim(self, program, payload, root, identity_mode=None, doc_output=None, stdlib_api_path=None, stdlib_path=None):
        environment = os.environ.copy()
        environment["FAKE_NATIVE_LOG"] = str(root / "native.log")
        if identity_mode is not None:
            environment["FAKE_NATIVE_IDENTITY_MODE"] = identity_mode
        if doc_output is not None:
            environment["FAKE_NATIVE_DOC_OUTPUT"] = json.dumps(doc_output, ensure_ascii=False)
        if stdlib_api_path is not None:
            environment["LSHARP_STDLIB_API_PATH"] = str(stdlib_api_path)
        if stdlib_path is not None:
            environment["LSHARP_STDLIB_PATH"] = str(stdlib_path)
        return subprocess.run(
            [sys.executable, str(SHIM), "--program", str(program)],
            input=payload,
            capture_output=True,
            env=environment,
            check=False,
        )

    def responses(self, output):
        return [json.loads(line) for line in output.decode().splitlines() if line]
    def test_stdlib_api_projects_generated_metadata(self):
        assert_stdlib_api_projects_generated_metadata(self)
    def test_stdlib_api_rejects_invalid_arguments(self):
        assert_stdlib_api_rejects_invalid_arguments(self)
    def test_stdlib_api_generates_from_native_doc(self):
        assert_stdlib_api_generates_from_native_doc(self)
    def test_stdlib_api_rejects_malformed_native_doc(self):
        assert_stdlib_api_rejects_malformed_native_doc(self)
    def test_package_api_generates_from_native_doc(self):
        assert_package_api_generates_from_native_doc(self)
    def test_package_api_rejects_malformed_native_doc(self):
        assert_package_api_rejects_malformed_native_doc(self)
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
            self.assertEqual(
                tool_names,
                {
                    "lsharp_check",
                    "lsharp_validate",
                    "lsharp_format",
                    "lsharp_errors",
                    "lsharp_search",
                    "lsharp_project_context",
                    "lsharp_package_api",
                    "lsharp_stdlib_api",
                },
            )
            check_tool = next(
                tool for tool in responses[1]["result"]["tools"] if tool["name"] == "lsharp_check"
            )
            check_schema = check_tool["inputSchema"]
            self.assertEqual(
                check_schema["oneOf"], [{"required": ["source"]}, {"required": ["file"]}]
            )
            migration_schema = check_tool["outputSchema"]["properties"]["migrationDiagnostics"]
            self.assertEqual(migration_schema["type"], "array")
            self.assertEqual(
                migration_schema["items"]["required"],
                ["code", "owner", "selectedSemantics", "disposition", "range"],
            )
            self.assertEqual(
                migration_schema["items"]["properties"]["code"]["enum"],
                ["LS2001", "LS2002", "LS2003"],
            )
            self.assertEqual(
                migration_schema["items"]["properties"]["selectedSemantics"]["enum"],
                [
                    "legacy-example-truthiness",
                    "legacy-invariant-deterministic-smoke",
                ],
            )
            self.assertEqual(
                migration_schema["items"]["properties"]["disposition"]["enum"],
                [
                    "docs-only-example",
                    "assertion",
                    "property-postcondition",
                    "manual-review",
                ],
            )
            self.assertEqual(
                migration_schema["items"]["properties"]["range"]["properties"]["start"],
                {"$ref": "#/$defs/position"},
            )
            self.assertEqual(
                check_tool["outputSchema"]["$defs"]["position"]["required"],
                ["line", "character"],
            )
            errors_tool = next(
                tool for tool in responses[1]["result"]["tools"] if tool["name"] == "lsharp_errors"
            )
            self.assertEqual(errors_tool["inputSchema"]["oneOf"], [{"required": ["error_code"]}])
            self.assertFalse(errors_tool["inputSchema"].get("additionalProperties", True))
            self.assertEqual(errors_tool["inputSchema"]["properties"]["error_code"]["minLength"], 1)
            self.assertEqual(
                errors_tool["outputSchema"]["required"],
                ["code", "name", "description", "fix", "doc"],
            )
            self.assertEqual(
                errors_tool["outputSchema"]["properties"]["doc"]["const"],
                "docs/guides/error-reference.md",
            )
            validate_tool = next(
                tool for tool in responses[1]["result"]["tools"] if tool["name"] == "lsharp_validate"
            )
            validate_schema = validate_tool["inputSchema"]
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
            self.assertEqual(
                validate_schema["properties"]["review_now"]["pattern"],
                r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$",
            )
            validate_output_schema = validate_tool["outputSchema"]
            self.assertFalse(validate_output_schema["additionalProperties"])
            trace_gap_schema = validate_output_schema["properties"]["trace_gaps"]["items"]
            self.assertEqual(trace_gap_schema["required"], ["code", "subject_id"])
            self.assertEqual(
                trace_gap_schema["properties"]["code"]["enum"],
                ["trace-gap.intent-without-claim", "trace-gap.claim-without-test"],
            )
            identity_schema = validate_output_schema["properties"]["review_evidence_identity"]
            self.assertEqual(
                identity_schema["required"],
                [
                    "subject_digest",
                    "source_commit",
                    "artifact_digest",
                    "trust_store_digest",
                    "lifecycle_digest",
                    "now",
                ],
            )
            self.assertEqual(
                identity_schema["properties"]["trust_store_digest"]["type"],
                ["string", "null"],
            )
            verification_schema = validate_output_schema["properties"]["review_verifications"]["items"]
            self.assertEqual(verification_schema["required"], ["review_id", "state"])
            self.assertEqual(
                verification_schema["properties"]["state"]["enum"],
                ["verified", "unverified", "stale", "revoked"],
            )
            manifest_schema = validate_output_schema["properties"]["manifest"]
            self.assertEqual(manifest_schema["type"], "object")
            self.assertEqual(
                manifest_schema["required"],
                ["schema_version", "nodes", "evidence", "edges"],
            )
            self.assertEqual(manifest_schema["properties"]["schema_version"]["const"], 1)
            for field in ("nodes", "evidence", "edges"):
                self.assertEqual(manifest_schema["properties"][field]["type"], "array")
            self.assertFalse(manifest_schema["additionalProperties"])
            node_schema = manifest_schema["properties"]["nodes"]["items"]
            self.assertEqual(node_schema["required"], ["kind", "namespace", "key", "text"])
            self.assertEqual(
                node_schema["properties"]["kind"]["enum"],
                ["intent", "claim", "assumption", "open-question"],
            )
            self.assertEqual(
                node_schema["properties"]["namespace"]["pattern"],
                r"^[A-Za-z0-9_.-]+$",
            )
            evidence_schema = manifest_schema["properties"]["evidence"]["items"]
            self.assertEqual(
                evidence_schema["required"],
                [
                    "namespace",
                    "key",
                    "method",
                    "subject",
                    "outcome",
                    "execution",
                    "provenance",
                    "independence",
                ],
            )
            self.assertEqual(
                evidence_schema["properties"]["method"]["enum"],
                ["example", "case", "assert", "property", "production", "reference", "proof", "review"],
            )
            self.assertEqual(len(manifest_schema["properties"]["edges"]["items"]["oneOf"]), 6)
            manifest_input_schema = validate_schema["properties"]["manifest"]["oneOf"][0]
            self.assertFalse(manifest_input_schema["additionalProperties"])
            self.assertEqual(
                manifest_input_schema["properties"]["nodes"]["items"]["required"],
                ["kind", "namespace", "key", "text"],
            )
            self.assertEqual(responses[2]["result"]["structuredContent"]["ok"], True)
            self.assertEqual(responses[3]["result"]["structuredContent"]["status"], "unknown")
            self.assertEqual(responses[4]["result"]["structuredContent"], {"formatted": "(formatted)\n"})
            calls = [json.loads(line) for line in (root / "native.log").read_text().splitlines()]
            self.assertEqual(calls[0][0], "check")
            self.assertEqual(calls[0][2:4], ["--format", "json"])
            self.assertEqual(calls[1][0:2], ["validate", "--source"])
            self.assertEqual(calls[2][0], "fmt")

    def test_errors_lookup_projects_canonical_table_without_native_execution(self):
        assert_errors_lookup(self)
    def test_errors_rejects_missing_or_unknown_arguments_before_native_execution(self):
        assert_errors_reject_invalid_arguments(self)
    def test_search_projects_local_packages_without_native_execution(self):
        assert_search_projects_local_packages(self)
    def test_search_rejects_invalid_arguments_before_native_execution(self):
        assert_search_rejects_invalid_arguments(self)
    def test_project_context_projects_local_metadata_without_native_execution(self):
        assert_project_context_projects_local_metadata(self)

    def test_project_context_rejects_invalid_arguments_before_native_execution(self):
        assert_project_context_rejects_invalid_arguments(self)
    def test_package_api_projects_local_api_json_without_native_execution(self):
        assert_package_api_projects_local_api_json(self)
    def test_package_api_rejects_invalid_arguments_before_native_execution(self):
        assert_package_api_rejects_invalid_arguments(self)

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
            response = self.responses(result.stdout)[0]["result"]
            self.assertEqual(response["isError"], False)
            expected_identity = {
                "subject_digest": "sha256:subject",
                "source_commit": "a" * 40,
                "artifact_digest": "sha256:artifact",
                "trust_store_digest": "sha256:trust",
                "lifecycle_digest": "sha256:lifecycle",
                "now": "2026-08-01T00:00:00Z",
            }
            self.assertEqual(response["structuredContent"]["review_evidence_identity"], expected_identity)
            self.assertEqual(response["structuredContent"]["manifest"]["review_evidence_identity"], expected_identity)

    def test_native_report_identity_mismatch_is_rejected_after_execution(self):
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
                        "review_source_commit": "a" * 40,
                        "review_artifact_digest": "sha256:artifact",
                        "review_now": "2026-08-01T00:00:00Z",
                    },
                },
            )

            result = self.run_shim(program, payload, root, identity_mode="report-mismatch")

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]["result"]
            self.assertTrue(response["isError"])
            self.assertIn("review_evidence_identity", response["content"][0]["text"])

    def test_explicit_identity_uses_computed_provider_digests(self):
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
                        "review_subject_digest": "sha256:subject",
                        "review_source_commit": "a" * 40,
                        "review_artifact_digest": "sha256:artifact",
                        "review_now": "2026-08-01T00:00:00Z",
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]["result"]
            self.assertFalse(response["isError"])
            identity = response["structuredContent"]["review_evidence_identity"]
            self.assertEqual(identity["trust_store_digest"], trust_digest)
            self.assertEqual(identity["lifecycle_digest"], lifecycle_digest)

    def test_native_report_identity_missing_is_rejected_after_execution(self):
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
                        "review_source_commit": "a" * 40,
                        "review_artifact_digest": "sha256:artifact",
                        "review_now": "2026-08-01T00:00:00Z",
                    },
                },
            )

            result = self.run_shim(program, payload, root, identity_mode="missing")

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]["result"]
            self.assertTrue(response["isError"])
            self.assertIn("is missing", response["content"][0]["text"])

    def test_implicit_report_identity_is_rejected_without_explicit_context(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            payload = request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {"source": "(defn main [] true)"},
                },
            )

            result = self.run_shim(program, payload, root, identity_mode="unexpected")

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]["result"]
            self.assertTrue(response["isError"])
            self.assertIn("implicit", response["content"][0]["text"])

    def test_native_manifest_identity_mismatch_is_rejected_after_execution(self):
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
                        "review_now": "2026-08-01T00:00:00Z",
                    },
                },
            )

            result = self.run_shim(program, payload, root, identity_mode="manifest-mismatch")

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]["result"]
            self.assertTrue(response["isError"])
            self.assertIn("review_evidence_identity", response["content"][0]["text"])

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

    def test_noncanonical_review_now_is_rejected_before_native_execution(self):
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
                        "review_source_commit": "a" * 40,
                        "review_artifact_digest": "sha256:artifact",
                        "review_now": "2026-08-01 00:00:00Z",
                    },
                },
            )

            result = self.run_shim(program, payload, root)

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]
            self.assertTrue(response["result"]["isError"])
            self.assertIn("review_now", response["result"]["content"][0]["text"])
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
