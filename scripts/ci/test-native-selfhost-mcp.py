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
from native_selfhost_mcp_package_tests import assert_package_api_generates_from_native_doc, assert_package_api_projects_local_api_json, assert_package_api_rejects_invalid_arguments, assert_package_api_rejects_malformed_native_doc, assert_search_ignores_non_directory_symlinks, assert_search_projects_local_packages, assert_search_rejects_invalid_arguments
from native_selfhost_mcp_context_tests import assert_project_context_projects_local_metadata, assert_project_context_rejects_invalid_arguments
from native_selfhost_mcp_compile_tests import assert_compile_run_fails_closed_and_cleans_artifacts, assert_compile_run_projects_file_without_mutating_input, assert_compile_run_projects_source_and_external_runtime, assert_compile_run_rejects_invalid_arguments_before_native, assert_compile_run_requires_explicit_runtime_without_host_fallback
from native_selfhost_mcp_stdlib_tests import assert_stdlib_api_generates_from_native_doc, assert_stdlib_api_projects_generated_metadata, assert_stdlib_api_rejects_duplicate_artifact, assert_stdlib_api_rejects_invalid_arguments, assert_stdlib_api_rejects_malformed_native_doc
from native_selfhost_mcp_lsp_tests import assert_completion_projects_empty_native_result, assert_completion_projects_native_lsp, assert_completion_rejects_invalid_arguments_before_native, assert_completion_rejects_native_failures, assert_completion_supports_file_and_col_alias, assert_definition_projects_native_lsp, assert_definition_rejects_invalid_arguments_before_native, assert_definition_rejects_native_failures, assert_definition_supports_file_and_col_alias, assert_hover_projects_native_lsp, assert_hover_rejects_invalid_arguments_before_native, assert_hover_rejects_native_failures, assert_hover_supports_file_and_col_alias, assert_lsp_position_alias_schema_is_exclusive, assert_lsp_rejects_both_position_aliases, assert_references_projects_empty_native_result, assert_references_projects_native_lsp, assert_references_rejects_invalid_arguments_before_native, assert_references_rejects_native_failures, assert_references_supports_file_and_col_alias
from native_selfhost_mcp_manifest_tests import assert_validate_accepts_empty_sampling_coverage, assert_validate_accepts_opaque_manifest_references, assert_validate_accepts_valid_emitted_manifest_edges, assert_validate_accepts_valid_emitted_manifest_evidence, assert_validate_accepts_valid_emitted_manifest_items, assert_validate_rejects_invalid_emitted_manifest, assert_validate_rejects_non_object_manifest_before_native, assert_validate_rejects_non_object_manifest_file_before_native, assert_validate_rejects_duplicate_manifest_input_before_native
from native_selfhost_mcp_validate_tests import assert_validate_accepts_valid_nested_report, assert_validate_accepts_valid_report_identity, assert_validate_rejects_invalid_report, assert_validate_rejects_invalid_report_identity
from native_selfhost_mcp_check_tests import assert_check_accepts_valid_migration_diagnostics, assert_check_rejects_blank_source_before_native, assert_check_rejects_invalid_arguments_before_native, assert_check_rejects_invalid_output, assert_source_input_schema_requires_non_empty_strings
from native_selfhost_mcp_format_tests import assert_check_format_input_schemas_are_closed, assert_format_output_schema_is_closed, assert_format_rejects_blank_source_before_native, assert_format_rejects_invalid_arguments_before_native, assert_format_rejects_native_failures
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
                    check_mode = os.environ.get("FAKE_NATIVE_CHECK_MODE", "object")
                    if check_mode == "array":
                        check_output = "[]"
                    elif check_mode == "null":
                        check_output = "null"
                    elif check_mode == "malformed":
                        check_output = "{{"
                    elif check_mode == "missing":
                        check_output = json.dumps({{"ok": True, "diagnostics": []}})
                    elif check_mode == "unknown":
                        check_output = json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": [], "extra": True}})
                    elif check_mode == "ok-type":
                        check_output = json.dumps({{"ok": "yes", "diagnostics": [], "migrationDiagnostics": []}})
                    elif check_mode == "diagnostics-type":
                        check_output = json.dumps({{"ok": True, "diagnostics": {{}}, "migrationDiagnostics": []}})
                    elif check_mode == "migration-missing":
                        check_output = json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": [{{}}]}})
                    elif check_mode == "migration-code":
                        check_output = json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": [{{
                            "code": "LS9999",
                            "owner": "main",
                            "selectedSemantics": "legacy-example-truthiness",
                            "disposition": "assertion",
                            "range": {{"start": {{"line": 0, "character": 0}}, "end": {{"line": 0, "character": 1}}}},
                        }}]}})
                    elif check_mode == "migration-range":
                        check_output = json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": [{{
                            "code": "LS2001",
                            "owner": "main",
                            "selectedSemantics": "legacy-example-truthiness",
                            "disposition": "assertion",
                            "range": [],
                        }}]}})
                    elif check_mode == "migration-position":
                        check_output = json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": [{{
                            "code": "LS2001",
                            "owner": "main",
                            "selectedSemantics": "legacy-example-truthiness",
                            "disposition": "assertion",
                            "range": {{"start": {{"line": -1, "character": 0}}, "end": {{"line": 0, "character": 1}}}},
                        }}]}})
                    elif check_mode == "migration-valid":
                        check_output = json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": [{{
                            "code": "LS2001",
                            "owner": "main",
                            "selectedSemantics": "legacy-example-truthiness",
                            "disposition": "assertion",
                            "range": {{"start": {{"line": 0, "character": 0}}, "end": {{"line": 0, "character": 1}}}},
                            "message": "legacy example",
                        }}]}})
                    else:
                        check_output = json.dumps({{"ok": True, "diagnostics": [], "migrationDiagnostics": []}})
                    print(check_output)
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
                    report_mode = os.environ.get("FAKE_NATIVE_REPORT_MODE", "object")
                    if report_mode == "missing":
                        report.pop("status")
                    elif report_mode == "unknown":
                        report["extra"] = True
                    elif report_mode == "status":
                        report["status"] = "maybe"
                    elif report_mode == "count-bool":
                        report["open_questions"] = True
                    elif report_mode == "count-overflow":
                        report["stale_evidence"] = 18446744073709551616
                    elif report_mode == "trace-gap-missing":
                        report["trace_gaps"] = [{{}}]
                    elif report_mode == "trace-gap-code":
                        report["trace_gaps"] = [{{
                            "code": "trace-gap.unknown",
                            "subject_id": "claim-1",
                        }}]
                    elif report_mode == "trace-gap-extra":
                        report["trace_gaps"] = [{{
                            "code": "trace-gap.claim-without-test",
                            "subject_id": "claim-1",
                            "extra": True,
                        }}]
                    elif report_mode == "review-verification-missing":
                        report["review_verifications"] = [{{}}]
                    elif report_mode == "review-verification-id":
                        report["review_verifications"] = [{{
                            "review_id": "invalid",
                            "state": "verified",
                        }}]
                    elif report_mode == "review-verification-state":
                        report["review_verifications"] = [{{
                            "review_id": "review:team/one",
                            "state": "maybe",
                        }}]
                    elif report_mode == "nested-valid":
                        report["trace_gaps"] = [{{
                            "code": "trace-gap.claim-without-test",
                            "subject_id": "claim-1",
                        }}]
                        report["review_verifications"] = [{{
                            "review_id": "review:team/one",
                            "state": "verified",
                        }}]
                    elif report_mode == "identity-missing":
                        report["review_evidence_identity"] = {{
                            "subject_digest": "sha256:subject",
                        }}
                    elif report_mode == "identity-extra":
                        report["review_evidence_identity"] = {{
                            "subject_digest": "sha256:subject",
                            "source_commit": "a" * 40,
                            "artifact_digest": "sha256:artifact",
                            "trust_store_digest": None,
                            "lifecycle_digest": None,
                            "now": "2026-08-01T00:00:00Z",
                            "extra": True,
                        }}
                    elif report_mode == "identity-type":
                        report["review_evidence_identity"] = {{
                            "subject_digest": "sha256:subject",
                            "source_commit": "a" * 40,
                            "artifact_digest": "sha256:artifact",
                            "trust_store_digest": 42,
                            "lifecycle_digest": None,
                            "now": "2026-08-01T00:00:00Z",
                        }}
                    elif report_mode == "identity-valid":
                        report["review_evidence_identity"] = {{
                            "subject_digest": "sha256:subject",
                            "source_commit": "a" * 40,
                            "artifact_digest": "sha256:artifact",
                            "trust_store_digest": None,
                            "lifecycle_digest": None,
                            "now": "2026-08-01T00:00:00Z",
                        }}
                    if report_identity is not None:
                        report["review_evidence_identity"] = report_identity
                    if "--emit-manifest" in args:
                        output = pathlib.Path(args[args.index("--emit-manifest") + 1])
                        output.parent.mkdir(parents=True, exist_ok=True)
                        manifest_mode = os.environ.get("FAKE_NATIVE_MANIFEST_MODE", "object")
                        evidence_item = {{
                            "namespace": "demo",
                            "key": "evidence-1",
                            "method": "example",
                            "subject": {{"kind": "claim", "namespace": "demo", "key": "claim-1"}},
                            "outcome": "pass",
                            "execution": {{
                                "runner": "native",
                                "target": "aarch64-apple-darwin",
                                "source_commit": "a" * 40,
                                "artifact_digest": "sha256:artifact",
                                "sampling": {{
                                    "cases": 1,
                                    "seed": 2,
                                    "generator": "fixed",
                                    "shrinks": [0, 1],
                                    "coverage": {{"branch": 1}},
                                }},
                            }},
                            "provenance": {{
                                "producer": "native",
                                "tool_version": "0.1",
                                "timestamp": "2026-08-01T00:00:00Z",
                            }},
                            "independence": "same-author",
                        }}
                        if manifest_mode == "array":
                            manifest_output = "[]"
                        elif manifest_mode == "null":
                            manifest_output = "null"
                        elif manifest_mode == "malformed":
                            manifest_output = "{{"
                        elif manifest_mode == "duplicate":
                            manifest_output = (
                                '{{"schema_version":1,"schema_version":1,'
                                '"nodes":[],"evidence":[],"edges":[]}}'
                            )
                        elif manifest_mode == "missing":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": []}})
                        elif manifest_mode == "unknown":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [], "edges": [], "extra": True}})
                        elif manifest_mode == "nodes-object":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": {{}}, "evidence": [], "edges": []}})
                        elif manifest_mode == "node-missing":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [{{}}], "evidence": [], "edges": []}})
                        elif manifest_mode == "node-kind":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [{{
                                "kind": "unknown",
                                "namespace": "demo",
                                "key": "claim-1",
                                "text": "claim",
                            }}], "evidence": [], "edges": []}})
                        elif manifest_mode == "node-extra":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [{{
                                "kind": "claim",
                                "namespace": "demo",
                                "key": "claim-1",
                                "text": "claim",
                                "extra": True,
                            }}], "evidence": [], "edges": []}})
                        elif manifest_mode == "node-text":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [{{
                                "kind": "claim",
                                "namespace": "demo",
                                "key": "claim-1",
                                "text": "",
                            }}], "evidence": [], "edges": []}})
                        elif manifest_mode == "node-span":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [{{
                                "kind": "claim",
                                "namespace": "demo",
                                "key": "claim-1",
                                "text": "claim",
                                "span": {{"start": -1, "end": 5}},
                            }}], "evidence": [], "edges": []}})
                        elif manifest_mode == "review-missing":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "reviews": [{{}}], "evidence": [], "edges": []}})
                        elif manifest_mode == "review-visibility":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "reviews": [{{
                                "namespace": "demo",
                                "key": "review-1",
                                "provenance_digest": "sha256:review",
                                "visibility": "private",
                            }}], "evidence": [], "edges": []}})
                        elif manifest_mode == "review-extra":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "reviews": [{{
                                "namespace": "demo",
                                "key": "review-1",
                                "provenance_digest": "sha256:review",
                                "visibility": "public",
                                "extra": True,
                            }}], "evidence": [], "edges": []}})
                        elif manifest_mode == "nested-valid":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [{{
                                "kind": "claim",
                                "namespace": "demo",
                                "key": "claim-1",
                                "text": "claim",
                                "span": {{"start": 0, "end": 5}},
                            }}], "reviews": [{{
                                "namespace": "demo",
                                "key": "review-1",
                                "provenance_digest": "sha256:review",
                                "visibility": "public",
                                "verification_state": "verified",
                            }}], "evidence": [], "edges": []}})
                        elif manifest_mode == "edge-missing":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [], "edges": [{{}}]}})
                        elif manifest_mode == "edge-relation":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [], "edges": [{{
                                "relation": "unknown",
                            }}]}})
                        elif manifest_mode == "edge-extra":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [], "edges": [{{
                                "relation": "motivates",
                                "intent": {{"namespace": "demo", "key": "intent-1"}},
                                "claim": {{"namespace": "demo", "key": "claim-1"}},
                                "extra": True,
                            }}]}})
                        elif manifest_mode == "edge-id":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [], "edges": [{{
                                "relation": "motivates",
                                "intent": {{"namespace": "demo"}},
                                "claim": {{"namespace": "demo", "key": "claim-1"}},
                            }}]}})
                        elif manifest_mode == "edge-subject":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [], "edges": [{{
                                "relation": "evaluates",
                                "review": {{"namespace": "demo", "key": "review-1"}},
                                "subject": {{"kind": "review", "namespace": "demo", "key": "review-1"}},
                            }}]}})
                        elif manifest_mode == "edges-valid":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "intent", "namespace": "demo", "key": "intent-1", "text": "intent"}},
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                                {{"kind": "assumption", "namespace": "demo", "key": "assumption-1", "text": "assumption"}},
                            ], "reviews": [{{
                                "namespace": "demo",
                                "key": "review-1",
                                "provenance_digest": "sha256:review",
                                "visibility": "public",
                            }}], "evidence": [evidence_item], "edges": [
                                {{
                                    "relation": "motivates",
                                    "intent": {{"namespace": "demo", "key": "intent-1"}},
                                    "claim": {{"namespace": "demo", "key": "claim-1"}},
                                }},
                                {{
                                    "relation": "supports",
                                    "observation": {{"namespace": "demo", "key": "evidence-1"}},
                                    "claim": {{"namespace": "demo", "key": "claim-1"}},
                                }},
                                {{
                                    "relation": "constrained-by",
                                    "claim": {{"namespace": "demo", "key": "claim-1"}},
                                    "assumption": {{"namespace": "demo", "key": "assumption-1"}},
                                }},
                                {{
                                    "relation": "tested-by",
                                    "claim": {{"namespace": "demo", "key": "claim-1"}},
                                    "contract": {{"namespace": "demo", "key": "contract-1"}},
                                }},
                                {{
                                    "relation": "contradicts",
                                    "observation": {{"namespace": "demo", "key": "evidence-1"}},
                                    "claim": {{"namespace": "demo", "key": "claim-1"}},
                                }},
                                {{
                                    "relation": "evaluates",
                                    "review": {{"namespace": "demo", "key": "review-1"}},
                                    "subject": {{"kind": "evidence", "namespace": "demo", "key": "evidence-1"}},
                                }},
                                {{
                                    "relation": "invalidates",
                                    "change": {{"namespace": "demo", "key": "change-1"}},
                                    "subject": {{"kind": "review", "namespace": "demo", "key": "review-1"}},
                                }},
                            ]}})
                        elif manifest_mode == "closure-duplicate-node":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "duplicate"}},
                            ], "evidence": [], "edges": []}})
                        elif manifest_mode == "closure-duplicate-evidence":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [evidence_item, evidence_item], "edges": []}})
                        elif manifest_mode == "closure-duplicate-review":
                            review = {{
                                "namespace": "demo",
                                "key": "review-1",
                                "provenance_digest": "sha256:review",
                                "visibility": "public",
                            }}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "reviews": [review, review], "evidence": [], "edges": []}})
                        elif manifest_mode == "closure-evidence-subject":
                            item = dict(evidence_item)
                            item["subject"] = {{"kind": "claim", "namespace": "demo", "key": "claim-1"}}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "closure-edge-node":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [], "edges": [{{
                                "relation": "motivates",
                                "intent": {{"namespace": "demo", "key": "intent-1"}},
                                "claim": {{"namespace": "demo", "key": "claim-1"}},
                            }}]}})
                        elif manifest_mode == "closure-edge-kind":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "intent", "namespace": "demo", "key": "intent-1", "text": "intent"}},
                            ], "evidence": [], "edges": [{{
                                "relation": "tested-by",
                                "claim": {{"namespace": "demo", "key": "intent-1"}},
                                "contract": {{"namespace": "demo", "key": "contract-1"}},
                            }}]}})
                        elif manifest_mode == "closure-edge-evidence":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [], "edges": [{{
                                "relation": "supports",
                                "observation": {{"namespace": "demo", "key": "evidence-1"}},
                                "claim": {{"namespace": "demo", "key": "claim-1"}},
                            }}]}})
                        elif manifest_mode == "closure-edge-review":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "reviews": [], "evidence": [], "edges": [{{
                                "relation": "evaluates",
                                "review": {{"namespace": "demo", "key": "review-1"}},
                                "subject": {{"kind": "claim", "namespace": "demo", "key": "claim-1"}},
                            }}]}})
                        elif manifest_mode == "closure-opaque-review-valid":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [], "edges": [{{
                                "relation": "evaluates",
                                "review": {{"namespace": "demo", "key": "review-1"}},
                                "subject": {{"kind": "claim", "namespace": "demo", "key": "claim-1"}},
                            }}]}})
                        elif manifest_mode == "closure-contract-subject-valid":
                            item = dict(evidence_item)
                            item["subject"] = {{"kind": "contract", "namespace": "demo", "key": "contract-1"}}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-missing":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [{{}}], "edges": []}})
                        elif manifest_mode == "evidence-method":
                            item = dict(evidence_item)
                            item["method"] = "unknown"
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-subject":
                            item = dict(evidence_item)
                            item["subject"] = {{"kind": "review", "namespace": "demo", "key": "review-1"}}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-execution":
                            item = dict(evidence_item)
                            item["execution"] = {{}}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-sampling":
                            item = dict(evidence_item)
                            item["execution"] = dict(evidence_item["execution"])
                            item["execution"]["sampling"] = dict(evidence_item["execution"]["sampling"])
                            item["execution"]["sampling"]["cases"] = -1
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-coverage-mismatch":
                            item = dict(evidence_item)
                            item["execution"] = dict(evidence_item["execution"])
                            item["execution"]["sampling"] = dict(evidence_item["execution"]["sampling"])
                            item["execution"]["sampling"]["coverage"] = {{"branch": 2}}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-coverage-overflow":
                            item = dict(evidence_item)
                            item["execution"] = dict(evidence_item["execution"])
                            item["execution"]["sampling"] = dict(evidence_item["execution"]["sampling"])
                            item["execution"]["sampling"]["coverage"] = {{
                                "first": 18446744073709551615,
                                "second": 1,
                            }}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-coverage-empty-valid":
                            item = dict(evidence_item)
                            item["execution"] = dict(evidence_item["execution"])
                            item["execution"]["sampling"] = dict(evidence_item["execution"]["sampling"])
                            item["execution"]["sampling"]["coverage"] = {{}}
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-provenance":
                            item = dict(evidence_item)
                            item["provenance"] = dict(evidence_item["provenance"])
                            item["provenance"]["producer"] = ""
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-extra":
                            item = dict(evidence_item)
                            item["extra"] = True
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [], "evidence": [item], "edges": []}})
                        elif manifest_mode == "evidence-valid":
                            manifest_output = json.dumps({{"schema_version": 1, "nodes": [
                                {{"kind": "claim", "namespace": "demo", "key": "claim-1", "text": "claim"}},
                            ], "evidence": [evidence_item], "edges": []}})
                        else:
                            manifest = {{"schema_version": 1, "nodes": [], "evidence": [], "edges": []}}
                            if manifest_identity is not None:
                                manifest["review_evidence_identity"] = manifest_identity
                            manifest_output = json.dumps(manifest)
                        output.write_text(manifest_output, encoding="utf-8")
                    if report_mode == "array":
                        report_output = "[]"
                    elif report_mode == "null":
                        report_output = "null"
                    elif report_mode == "malformed":
                        report_output = "{{"
                    elif report_mode == "duplicate":
                        report_output = (
                            '{{"status":"unknown","status":"unknown",'
                            '"trace_gaps":[],"open_questions":0,'
                            '"independent_reviews":0,"contradicting_observations":0,'
                            '"stale_reviews":0,"stale_evidence":0}}'
                        )
                    elif report_mode == "nonstandard":
                        report_output = (
                            '{{"status":"unknown","trace_gaps":[],"open_questions":NaN,'
                            '"independent_reviews":0,"contradicting_observations":0,'
                            '"stale_reviews":0,"stale_evidence":0}}'
                        )
                    else:
                        report_output = json.dumps(report)
                    print(report_output)
                    raise SystemExit(2)
                if args[:1] == ["fmt"]:
                    format_mode = os.environ.get("FAKE_NATIVE_FORMAT_MODE", "success")
                    if format_mode == "nonzero":
                        print("(formatted)")
                        sys.stderr.write("format diagnostic\\n")
                        raise SystemExit(7)
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

    def run_shim(self, program, payload, root, identity_mode=None, doc_output=None, stdlib_api_path=None, stdlib_path=None, wasmtime_path=None, manifest_mode=None, report_mode=None, check_mode=None, format_mode=None):
        environment = os.environ.copy()
        environment["FAKE_NATIVE_LOG"] = str(root / "native.log")
        environment["FAKE_WASMTIME_LOG"] = str(root / "wasmtime.log")
        environment.pop("LSHARP_WASMTIME", None)
        if identity_mode is not None:
            environment["FAKE_NATIVE_IDENTITY_MODE"] = identity_mode
        if doc_output is not None:
            environment["FAKE_NATIVE_DOC_OUTPUT"] = json.dumps(doc_output, ensure_ascii=False)
        if stdlib_api_path is not None:
            environment["LSHARP_STDLIB_API_PATH"] = str(stdlib_api_path)
        if stdlib_path is not None:
            environment["LSHARP_STDLIB_PATH"] = str(stdlib_path)
        if wasmtime_path is not None:
            environment["LSHARP_WASMTIME"] = str(wasmtime_path)
        if manifest_mode is not None:
            environment["FAKE_NATIVE_MANIFEST_MODE"] = manifest_mode
        if report_mode is not None:
            environment["FAKE_NATIVE_REPORT_MODE"] = report_mode
        if check_mode is not None:
            environment["FAKE_NATIVE_CHECK_MODE"] = check_mode
        if format_mode is not None:
            environment["FAKE_NATIVE_FORMAT_MODE"] = format_mode
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

    def test_stdlib_api_rejects_duplicate_artifact(self):
        assert_stdlib_api_rejects_duplicate_artifact(self)
    def test_stdlib_api_generates_from_native_doc(self):
        assert_stdlib_api_generates_from_native_doc(self)
    def test_stdlib_api_rejects_malformed_native_doc(self):
        assert_stdlib_api_rejects_malformed_native_doc(self)
    def test_compile_run_projects_source_and_external_runtime(self):
        assert_compile_run_projects_source_and_external_runtime(self)
    def test_compile_run_projects_file_without_mutating_input(self):
        assert_compile_run_projects_file_without_mutating_input(self)
    def test_compile_run_rejects_invalid_arguments_before_native(self):
        assert_compile_run_rejects_invalid_arguments_before_native(self)
    def test_compile_run_fails_closed_and_cleans_artifacts(self):
        assert_compile_run_fails_closed_and_cleans_artifacts(self)
    def test_compile_run_requires_explicit_runtime_without_host_fallback(self):
        assert_compile_run_requires_explicit_runtime_without_host_fallback(self)
    def test_hover_projects_native_lsp(self):
        assert_hover_projects_native_lsp(self)
    def test_hover_supports_file_and_col_alias(self):
        assert_hover_supports_file_and_col_alias(self)
    def test_hover_rejects_invalid_arguments_before_native(self):
        assert_hover_rejects_invalid_arguments_before_native(self)
    def test_hover_rejects_native_failures(self):
        assert_hover_rejects_native_failures(self)
    def test_definition_projects_native_lsp(self):
        assert_definition_projects_native_lsp(self)
    def test_definition_supports_file_and_col_alias(self):
        assert_definition_supports_file_and_col_alias(self)
    def test_definition_rejects_invalid_arguments_before_native(self):
        assert_definition_rejects_invalid_arguments_before_native(self)
    def test_definition_rejects_native_failures(self):
        assert_definition_rejects_native_failures(self)
    def test_references_projects_native_lsp(self):
        assert_references_projects_native_lsp(self)
    def test_references_supports_file_and_col_alias(self):
        assert_references_supports_file_and_col_alias(self)
    def test_references_projects_empty_native_result(self):
        assert_references_projects_empty_native_result(self)
    def test_references_rejects_invalid_arguments_before_native(self):
        assert_references_rejects_invalid_arguments_before_native(self)
    def test_references_rejects_native_failures(self):
        assert_references_rejects_native_failures(self)
    def test_completion_projects_native_lsp(self):
        assert_completion_projects_native_lsp(self)
    def test_lsp_position_alias_schema_is_exclusive(self):
        assert_lsp_position_alias_schema_is_exclusive(self)
    def test_lsp_rejects_both_position_aliases(self):
        assert_lsp_rejects_both_position_aliases(self)
    def test_completion_supports_file_and_col_alias(self):
        assert_completion_supports_file_and_col_alias(self)
    def test_completion_projects_empty_native_result(self):
        assert_completion_projects_empty_native_result(self)
    def test_completion_rejects_invalid_arguments_before_native(self):
        assert_completion_rejects_invalid_arguments_before_native(self)
    def test_completion_rejects_native_failures(self):
        assert_completion_rejects_native_failures(self)
    def test_package_api_generates_from_native_doc(self):
        assert_package_api_generates_from_native_doc(self)
    def test_package_api_rejects_malformed_native_doc(self):
        assert_package_api_rejects_malformed_native_doc(self)
    def test_search_ignores_non_directory_symlinks(self):
        assert_search_ignores_non_directory_symlinks(self)
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
            self.assertEqual(
                responses[0]["result"]["serverInfo"],
                {"name": "lsharp", "version": "0.1.0"},
            )
            self.assertEqual(
                [tool["name"] for tool in responses[1]["result"]["tools"]],
                [
                    "lsharp_check",
                    "lsharp_validate",
                    "lsharp_hover",
                    "lsharp_completion",
                    "lsharp_format",
                    "lsharp_definition",
                    "lsharp_references",
                    "lsharp_project_context",
                    "lsharp_package_api",
                    "lsharp_stdlib_api",
                    "lsharp_compile_run",
                    "lsharp_errors",
                    "lsharp_search",
                ],
            )
            tool_names = {tool["name"] for tool in responses[1]["result"]["tools"]}
            self.assertEqual(
                tool_names,
                {
                    "lsharp_check",
                    "lsharp_hover",
                    "lsharp_definition",
                    "lsharp_references",
                    "lsharp_completion",
                    "lsharp_compile_run",
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
            for field in (
                "open_questions",
                "independent_reviews",
                "contradicting_observations",
                "stale_reviews",
                "stale_evidence",
            ):
                self.assertEqual(
                    validate_output_schema["properties"][field]["maximum"],
                    18446744073709551615,
                )
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
            self.assertFalse(evidence_schema["additionalProperties"])
            self.assertEqual(
                evidence_schema["properties"]["subject"]["required"],
                ["kind", "namespace", "key"],
            )
            self.assertEqual(
                evidence_schema["properties"]["subject"]["properties"]["kind"]["enum"],
                ["intent", "claim", "contract"],
            )
            execution_schema = evidence_schema["properties"]["execution"]
            self.assertFalse(execution_schema["additionalProperties"])
            self.assertEqual(
                execution_schema["required"],
                ["runner", "target", "source_commit", "artifact_digest", "sampling"],
            )
            sampling_schema = execution_schema["properties"]["sampling"]
            self.assertFalse(sampling_schema["additionalProperties"])
            self.assertEqual(sampling_schema["required"], ["cases", "seed", "generator"])
            provenance_schema = evidence_schema["properties"]["provenance"]
            self.assertFalse(provenance_schema["additionalProperties"])
            self.assertEqual(
                provenance_schema["required"], ["producer", "tool_version", "timestamp"]
            )
            self.assertEqual(
                evidence_schema["properties"]["independence"]["enum"],
                ["same-author", "independent-review", "external-observation"],
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

    def test_jsonrpc_null_request_id_is_preserved_in_response_envelope(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            result = self.run_shim(program, request(None, "ping"), root)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertEqual(
                self.responses(result.stdout),
                [{"jsonrpc": "2.0", "id": None, "result": {}}],
            )
            self.assertFalse((root / "native.log").exists())

    def test_tools_call_missing_name_matches_rust_unknown_tool_result_envelope(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            result = self.run_shim(program, request(1, "tools/call"), root)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertEqual(
                self.responses(result.stdout),
                [
                    {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "result": {
                            "content": [{"type": "text", "text": "tool not found"}],
                            "isError": True,
                        },
                    }
                ],
            )
            self.assertFalse((root / "native.log").exists())

    def test_tools_call_non_object_params_matches_rust_unknown_tool_result_envelope(self):
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = pathlib.Path(temporary_directory)
            program = self.write_fake_program(root)
            result = self.run_shim(program, request(1, "tools/call", []), root)
            self.assertEqual(result.returncode, 0, result.stderr.decode())
            self.assertEqual(
                self.responses(result.stdout),
                [
                    {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "result": {
                            "content": [{"type": "text", "text": "tool not found"}],
                            "isError": True,
                        },
                    }
                ],
            )
            self.assertFalse((root / "native.log").exists())

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
    def test_validate_rejects_non_object_manifest_before_native_execution(self):
        assert_validate_rejects_non_object_manifest_before_native(self)

    def test_validate_rejects_non_object_manifest_file_before_native_execution(self):
        assert_validate_rejects_non_object_manifest_file_before_native(self)

    def test_validate_rejects_duplicate_manifest_input_before_native(self):
        assert_validate_rejects_duplicate_manifest_input_before_native(self)

    def test_validate_rejects_invalid_emitted_manifest(self):
        assert_validate_rejects_invalid_emitted_manifest(self)
    def test_validate_accepts_valid_emitted_manifest_items(self):
        assert_validate_accepts_valid_emitted_manifest_items(self)
    def test_validate_accepts_valid_emitted_manifest_edges(self):
        assert_validate_accepts_valid_emitted_manifest_edges(self)
    def test_validate_accepts_valid_emitted_manifest_evidence(self):
        assert_validate_accepts_valid_emitted_manifest_evidence(self)
    def test_validate_accepts_opaque_manifest_references(self):
        assert_validate_accepts_opaque_manifest_references(self)
    def test_validate_accepts_empty_sampling_coverage(self):
        assert_validate_accepts_empty_sampling_coverage(self)

    def test_validate_rejects_invalid_report(self):
        assert_validate_rejects_invalid_report(self)
    def test_validate_accepts_valid_nested_report(self):
        assert_validate_accepts_valid_nested_report(self)
    def test_validate_rejects_invalid_report_identity(self):
        assert_validate_rejects_invalid_report_identity(self)
    def test_validate_accepts_valid_report_identity(self):
        assert_validate_accepts_valid_report_identity(self)

    def test_check_rejects_invalid_output(self):
        assert_check_rejects_invalid_output(self)
    def test_check_accepts_valid_migration_diagnostics(self):
        assert_check_accepts_valid_migration_diagnostics(self)
    def test_check_rejects_invalid_arguments_before_native(self):
        assert_check_rejects_invalid_arguments_before_native(self)
    def test_source_input_schema_requires_non_empty_strings(self):
        assert_source_input_schema_requires_non_empty_strings(self)
    def test_check_rejects_blank_source_before_native(self):
        assert_check_rejects_blank_source_before_native(self)
    def test_check_format_input_schemas_are_closed(self):
        assert_check_format_input_schemas_are_closed(self)
    def test_format_output_schema_is_closed(self):
        assert_format_output_schema_is_closed(self)
    def test_format_rejects_invalid_arguments_before_native(self):
        assert_format_rejects_invalid_arguments_before_native(self)
    def test_format_rejects_blank_source_before_native(self):
        assert_format_rejects_blank_source_before_native(self)
    def test_format_rejects_native_failures(self):
        assert_format_rejects_native_failures(self)
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

    def test_provider_snapshot_semantic_state_is_rejected_without_native_verifier(self):
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
                    },
                },
            )

            result = self.run_shim(program, payload, root, report_mode="nested-valid")

            self.assertEqual(result.returncode, 0, result.stderr.decode())
            response = self.responses(result.stdout)[0]["result"]
            self.assertTrue(response["isError"])
            self.assertIn("semantic verification is unavailable", response["content"][0]["text"])

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

            duplicate_request = self.run_shim(
                program,
                b'{"jsonrpc":"2.0","id":1,"id":2,"method":"ping"}\n',
                root,
            )
            self.assertNotEqual(duplicate_request.returncode, 0)
            self.assertIn(b"duplicate JSON object key: id", duplicate_request.stderr)
            self.assertEqual(duplicate_request.stdout, b"")

            nonstandard_request = self.run_shim(
                program,
                b'{"jsonrpc":"2.0","id":Infinity,"method":"ping"}\n',
                root,
            )
            self.assertNotEqual(nonstandard_request.returncode, 0)
            self.assertIn(b"non-standard JSON constant: Infinity", nonstandard_request.stderr)
            self.assertEqual(nonstandard_request.stdout, b"")

            missing = self.run_shim(root / "missing", request(1, "ping"), root)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn(b"not a regular executable", missing.stderr)
            self.assertEqual(missing.stdout, b"")


if __name__ == "__main__":
    unittest.main(verbosity=2)
