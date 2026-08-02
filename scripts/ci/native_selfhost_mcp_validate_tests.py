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
        ("count-overflow", "stale_evidence must be a non-negative integer"),
        ("duplicate", "duplicate JSON object key: status"),
        ("nonstandard", "non-standard JSON constant: NaN"),
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


def review_attestation_projection():
    return {
        "review_id": "review:checkout/reviewer-001",
        "subject_digest": "sha256:subject-001",
        "source_commit": "0123456789abcdef",
        "provenance_digest": "sha256:review-001",
        "provider": "github",
        "key_id": "org/reviews-2026",
        "algorithm": "ed25519",
        "signature": "AAECAw",
        "issued_at": "2026-08-01T00:00:00Z",
        "expires_at": "2026-09-01T00:00:00Z",
        "sequence": 3,
        "state": "unverified",
        "canonical_bytes": [0, 1, 2],
        "span": {"start": 12, "end": 34},
    }


def assert_validate_accepts_review_attestation_report(test):
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
            report_mode="attestation-valid",
        )

        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        test.assertEqual(
            response["result"]["structuredContent"]["review_attestations"],
            [review_attestation_projection()],
        )


def assert_validate_rejects_invalid_review_attestation_report(test):
    cases = (
        ("attestation-missing", "missing field: signature"),
        ("attestation-extra", "unknown field: extra"),
        ("attestation-state", "state must be one of"),
        ("attestation-bytes", "canonical_bytes[0] must be a byte"),
        ("attestation-span", "span.start must be a non-negative integer"),
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


def review_verification_receipt_for_attestation():
    return {
        "review_id": "review:checkout/reviewer-001",
        "state": "verified",
        "provider": "github",
        "key_id": "org/reviews-2026",
        "algorithm": "ed25519",
        "attestation_digest": "sha256:ae4b3280e56e2faf83f414a6e3dabe9d5fbe18976544c05fed121accb85b53fc",
        "trust_store_digest": "sha256:" + "b" * 64,
        "verification_now": "2026-08-02T00:00:00Z",
    }


def review_attestation_receipt_projection(receipt):
    return {
        "review_id": receipt["review_id"],
        "subject_digest": "sha256:subject-001",
        "source_commit": "0123456789abcdef",
        "provenance_digest": "sha256:review-001",
        "provider": receipt["provider"],
        "key_id": receipt["key_id"],
        "algorithm": receipt["algorithm"],
        "signature": "AAECAw",
        "issued_at": "2026-08-01T00:00:00Z",
        "expires_at": "2026-09-01T00:00:00Z",
        "sequence": 3,
        "state": "verified",
        "canonical_bytes": [0, 1, 2],
        "span": {"start": 12, "end": 34},
    }


def assert_validate_accepts_review_attestation_receipt_projection(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        receipt = review_verification_receipt_for_attestation()
        receipt_path = root / "receipt.json"
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
        result = test.run_shim(
            program,
            request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {
                        "source": "(defn main [] true)",
                        "review_verification_receipt": str(receipt_path),
                    },
                },
            ),
            root,
            report_mode="receipt-attestation-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        test.assertEqual(
            response["result"]["structuredContent"]["review_attestations"],
            [review_attestation_receipt_projection(receipt)],
        )


def assert_validate_rejects_unbound_review_attestation_receipt(test):
    cases = (
        ("receipt-attestation-missing", "missing or ambiguous"),
        ("receipt-attestation-state", "state mismatch"),
        ("receipt-attestation-identity", "provider mismatch"),
        ("receipt-attestation-digest", "canonical digest mismatch"),
    )
    for report_mode, expected_message in cases:
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = test.write_fake_program(root)
            receipt = review_verification_receipt_for_attestation()
            receipt_path = root / "receipt.json"
            receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
            result = test.run_shim(
                program,
                request(
                    1,
                    "tools/call",
                    {
                        "name": "lsharp_validate",
                        "arguments": {
                            "source": "(defn main [] true)",
                            "review_verification_receipt": str(receipt_path),
                        },
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
