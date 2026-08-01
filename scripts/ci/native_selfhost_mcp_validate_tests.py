import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def assert_validate_rejects_invalid_report(test):
    cases = (
        ("array", "JSON object"),
        ("null", "JSON object"),
        ("malformed", "malformed native JSON"),
        ("missing", "missing field"),
        ("unknown", "unknown field"),
        ("status", "status"),
        ("count-bool", "open_questions"),
        ("trace-gap-missing", "missing field: code"),
        ("trace-gap-code", "code must be one of"),
        ("trace-gap-extra", "unknown field: extra"),
        ("review-verification-missing", "missing field: review_id"),
        ("review-verification-id", "review_id has invalid format"),
        ("review-verification-state", "state must be one of"),
    )
    for report_mode, expected_message in cases:
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
                        "arguments": {"source": "(defn main [] true)"},
                    },
                ),
                root,
                report_mode=report_mode,
            )

            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(expected_message, response["result"]["content"][0]["text"])
            test.assertNotIn("Traceback", response["result"]["content"][0]["text"])


def assert_validate_accepts_valid_nested_report(test):
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
                    "arguments": {"source": "(defn main [] true)"},
                },
            ),
            root,
            report_mode="nested-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        report = response["result"]["structuredContent"]
        test.assertEqual(
            report["trace_gaps"],
            [{"code": "trace-gap.claim-without-test", "subject_id": "claim-1"}],
        )
        test.assertEqual(
            report["review_verifications"],
            [{"review_id": "review:team/one", "state": "verified"}],
        )


def assert_validate_rejects_invalid_report_identity(test):
    cases = (
        ("identity-missing", "missing field: source_commit"),
        ("identity-extra", "unknown field: extra"),
        ("identity-type", "trust_store_digest must be a string or null"),
    )
    manifest = {"schema_version": 1, "nodes": [], "evidence": [], "edges": []}
    for report_mode, expected_message in cases:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = test.write_fake_program(root)
            result = test.run_shim(
                program,
                request(
                    1,
                    "tools/call",
                    {"name": "lsharp_validate", "arguments": {"manifest": manifest}},
                ),
                root,
                report_mode=report_mode,
            )

            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(expected_message, response["result"]["content"][0]["text"])
            test.assertNotIn("Traceback", response["result"]["content"][0]["text"])


def assert_validate_accepts_valid_report_identity(test):
    manifest = {"schema_version": 1, "nodes": [], "evidence": [], "edges": []}
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        result = test.run_shim(
            program,
            request(
                1,
                "tools/call",
                {"name": "lsharp_validate", "arguments": {"manifest": manifest}},
            ),
            root,
            report_mode="identity-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        test.assertEqual(
            response["result"]["structuredContent"]["review_evidence_identity"],
            {
                "subject_digest": "sha256:subject",
                "source_commit": "a" * 40,
                "artifact_digest": "sha256:artifact",
                "trust_store_digest": None,
                "lifecycle_digest": None,
                "now": "2026-08-01T00:00:00Z",
            },
        )
