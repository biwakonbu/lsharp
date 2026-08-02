import json
import pathlib
import tempfile


def request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return (json.dumps(payload, separators=(",", ":")) + "\n").encode()


def assert_validate_rejects_non_object_manifest_before_native(test):
    """manifest の root は object に限定し、native 実行前に安定して拒否する。"""
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        cases = ["[]", "null", "42"]
        payload = b"".join(
            request(
                index,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {"manifest": manifest},
                },
            )
            for index, manifest in enumerate(cases, 1)
        )

        result = test.run_shim(program, payload, root)

        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), len(cases))
        for response in responses:
            test.assertTrue(response["result"]["isError"])
            test.assertIn("JSON object", response["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())


def assert_validate_rejects_non_object_manifest_file_before_native(test):
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        manifest_file = root / "manifest.json"
        manifest_file.write_text("null\n", encoding="utf-8")
        result = test.run_shim(
            program,
            request(
                1,
                "tools/call",
                {
                    "name": "lsharp_validate",
                    "arguments": {"manifest_file": str(manifest_file)},
                },
            ),
            root,
        )

        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertTrue(response["result"]["isError"])
        test.assertIn("JSON object", response["result"]["content"][0]["text"])
        test.assertFalse((root / "native.log").exists())


def assert_validate_rejects_invalid_emitted_manifest(test):
    cases = (
        ("array", "JSON object"),
        ("null", "JSON object"),
        ("malformed", "manifest"),
        ("duplicate", "duplicate JSON object key: schema_version"),
        ("missing", "missing field"),
        ("unknown", "unknown field"),
        ("nodes-object", "must be an array"),
        ("node-missing", "nodes[0] is missing field: kind"),
        ("node-kind", "nodes[0].kind has invalid value"),
        ("node-extra", "nodes[0] has unknown field: extra"),
        ("node-text", "nodes[0].text must be a non-empty string"),
        ("node-span", "nodes[0].span.start must be a non-negative integer"),
        ("review-missing", "reviews[0] is missing field: namespace"),
        ("review-visibility", "reviews[0].visibility has invalid value"),
        ("review-extra", "reviews[0] has unknown field: extra"),
        ("edge-missing", "edges[0] is missing field: relation"),
        ("edge-relation", "edges[0].relation has invalid value"),
        ("edge-extra", "edges[0] has unknown field: extra"),
        ("edge-id", "edges[0].intent is missing field: key"),
        ("edge-subject", "edges[0].subject.kind has invalid value"),
        ("evidence-missing", "evidence[0] is missing field: namespace"),
        ("evidence-method", "evidence[0].method has invalid value"),
        ("evidence-subject", "evidence[0].subject.kind has invalid value"),
        ("evidence-execution", "evidence[0].execution is missing field: runner"),
        ("evidence-sampling", "evidence[0].execution.sampling.cases must be a non-negative integer"),
        (
            "evidence-coverage-mismatch",
            "evidence[0].execution.sampling.coverage total must equal cases: cases=1, covered=2",
        ),
        (
            "evidence-coverage-overflow",
            "evidence[0].execution.sampling.coverage total exceeds u64 range",
        ),
        ("evidence-provenance", "evidence[0].provenance.producer must be a non-empty string"),
        ("evidence-extra", "evidence[0] has unknown field: extra"),
        ("closure-duplicate-node", "nodes[1] duplicates ID: demo/claim-1"),
        ("closure-duplicate-evidence", "evidence[1] duplicates ID: demo/evidence-1"),
        ("closure-duplicate-review", "reviews[1] duplicates ID: demo/review-1"),
        ("closure-evidence-subject", "evidence[0].subject references missing node: demo/claim-1"),
        ("closure-edge-node", "edges[0].intent references missing node: demo/intent-1"),
        ("closure-edge-kind", "edges[0].claim references node kind intent, expected claim: demo/intent-1"),
        ("closure-edge-evidence", "edges[0].observation references missing evidence: demo/evidence-1"),
        ("closure-edge-review", "edges[0].review references missing review: demo/review-1"),
    )
    for manifest_mode, expected_message in cases:
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
                        "arguments": {
                            "source": "(defn main [] true)",
                            "include_manifest": True,
                        },
                    },
                ),
                root,
                manifest_mode=manifest_mode,
            )

            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertTrue(response["result"]["isError"])
            test.assertIn(expected_message, response["result"]["content"][0]["text"])
            test.assertNotIn("Traceback", response["result"]["content"][0]["text"])


def assert_validate_rejects_report_manifest_mismatch(test):
    """report 内と emit 出力の manifest が異なる場合は fail-closed にする。"""
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
                    "arguments": {
                        "source": "(defn main [] true)",
                        "include_manifest": True,
                    },
                },
            ),
            root,
            report_mode="embedded-manifest-mismatch",
        )

        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertTrue(response["result"]["isError"])
        test.assertIn(
            "native validate report manifest projection mismatch",
            response["result"]["content"][0]["text"],
        )
        test.assertNotIn("Traceback", response["result"]["content"][0]["text"])


def assert_validate_rejects_duplicate_manifest_input_before_native(test):
    duplicate_manifest = (
        '{"schema_version":1,"schema_version":1,"nodes":[],"evidence":[],"edges":[]}'
    )
    with tempfile.TemporaryDirectory() as temporary_directory:
        root = pathlib.Path(temporary_directory)
        program = test.write_fake_program(root)
        manifest_file = root / "manifest.json"
        manifest_file.write_text(duplicate_manifest, encoding="utf-8")
        payload = b"".join(
            (
                request(
                    request_id,
                    "tools/call",
                    {
                        "name": "lsharp_validate",
                        "arguments": arguments,
                    },
                )
            )
            for request_id, arguments in enumerate(
                (
                    {"manifest": duplicate_manifest},
                    {"manifest_file": str(manifest_file)},
                ),
                1,
            )
        )
        result = test.run_shim(program, payload, root)

        test.assertEqual(result.returncode, 0, result.stderr.decode())
        responses = test.responses(result.stdout)
        test.assertEqual(len(responses), 2)
        for response in responses:
            test.assertTrue(response["result"]["isError"])
            test.assertIn(
                "duplicate JSON object key: schema_version",
                response["result"]["content"][0]["text"],
            )
        test.assertFalse((root / "native.log").exists())


def assert_validate_accepts_valid_emitted_manifest_items(test):
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
                    "arguments": {
                        "source": "(defn main [] true)",
                        "include_manifest": True,
                    },
                },
            ),
            root,
            manifest_mode="nested-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        manifest = response["result"]["structuredContent"]["manifest"]
        test.assertEqual(manifest["nodes"][0]["span"], {"start": 0, "end": 5})
        test.assertEqual(manifest["reviews"][0]["verification_state"], "verified")


def assert_validate_accepts_valid_emitted_manifest_edges(test):
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
                    "arguments": {
                        "source": "(defn main [] true)",
                        "include_manifest": True,
                    },
                },
            ),
            root,
            manifest_mode="edges-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        edges = response["result"]["structuredContent"]["manifest"]["edges"]
        test.assertEqual(
            [edge["relation"] for edge in edges],
            [
                "motivates",
                "supports",
                "constrained-by",
                "tested-by",
                "contradicts",
                "evaluates",
                "invalidates",
            ],
        )


def assert_validate_accepts_valid_emitted_manifest_evidence(test):
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
                    "arguments": {
                        "source": "(defn main [] true)",
                        "include_manifest": True,
                    },
                },
            ),
            root,
            manifest_mode="evidence-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        evidence = response["result"]["structuredContent"]["manifest"]["evidence"][0]
        test.assertEqual(evidence["method"], "example")
        test.assertEqual(evidence["execution"]["sampling"]["coverage"], {"branch": 1})


def assert_validate_accepts_opaque_manifest_references(test):
    cases = (
        ("closure-opaque-review-valid", "evaluates"),
        ("closure-contract-subject-valid", "evidence"),
    )
    for manifest_mode, expected_field in cases:
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
                        "arguments": {
                            "source": "(defn main [] true)",
                            "include_manifest": True,
                        },
                    },
                ),
                root,
                manifest_mode=manifest_mode,
            )
            test.assertEqual(result.returncode, 0, result.stderr.decode())
            response = test.responses(result.stdout)[0]
            test.assertFalse(response["result"]["isError"])
            manifest = response["result"]["structuredContent"]["manifest"]
            if expected_field == "evaluates":
                test.assertEqual(manifest["edges"][0]["relation"], "evaluates")
            else:
                test.assertEqual(manifest["evidence"][0]["subject"]["kind"], "contract")


def assert_validate_accepts_empty_sampling_coverage(test):
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
                    "arguments": {
                        "source": "(defn main [] true)",
                        "include_manifest": True,
                    },
                },
            ),
            root,
            manifest_mode="evidence-coverage-empty-valid",
        )
        test.assertEqual(result.returncode, 0, result.stderr.decode())
        response = test.responses(result.stdout)[0]
        test.assertFalse(response["result"]["isError"])
        sampling = response["result"]["structuredContent"]["manifest"]["evidence"][0]["execution"]["sampling"]
        test.assertEqual(sampling["coverage"], {})
