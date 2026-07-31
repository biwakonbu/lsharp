#!/usr/bin/env python3
"""Expose the deterministic native selfhost subset over MCP stdio.

The native compiler remains the only implementation authority.  This shim only
translates JSON-RPC requests into the existing ``check``, ``validate`` and
``fmt`` CLI contracts; it never calls cargo, rustc, host ``lsharp`` or a
provider/network helper.  Explicit provider snapshot paths are an offline
bytes-to-digest adapter; signature and lifecycle semantic verification remain
an external provider boundary until a native verifier is available.
"""

import argparse
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

from native_selfhost_mcp_schema import (
    edge_schema,
    evidence_schema,
    node_schema,
    review_evidence_identity_schema,
    review_registry_schema,
)


MCP_PROTOCOL_VERSION = "2025-11-25"
CANONICAL_UTC_TIMESTAMP_PATTERN = (
    r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"
)
CANONICAL_UTC_TIMESTAMP_RE = re.compile(CANONICAL_UTC_TIMESTAMP_PATTERN)


class ShimError(Exception):
    pass


class ToolError(Exception):
    pass


def tool_descriptor(
    name, description, properties, required, output_schema=None, input_schema_extra=None
):
    alternatives = (
        [{"required": required}]
        if required and isinstance(required[0], str)
        else [{"required": alternative} for alternative in required]
    )
    input_schema = {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": properties,
        "oneOf": alternatives,
    }
    if input_schema_extra:
        input_schema.update(input_schema_extra)
    descriptor = {
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    }
    if output_schema is not None:
        descriptor["outputSchema"] = output_schema
    return descriptor


CHECK_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "required": ["ok", "diagnostics", "migrationDiagnostics"],
    "properties": {
        "ok": {"type": "boolean"},
        "diagnostics": {"type": "array"},
        "migrationDiagnostics": {
            "type": "array",
            "items": {
                "type": "object",
                "required": ["code", "owner", "selectedSemantics", "disposition", "range"],
                "properties": {
                    "code": {
                        "type": "string",
                        "enum": ["LS2001", "LS2002", "LS2003"],
                    },
                    "owner": {"type": "string"},
                    "selectedSemantics": {
                        "type": "string",
                        "enum": [
                            "legacy-example-truthiness",
                            "legacy-invariant-deterministic-smoke",
                        ],
                    },
                    "disposition": {
                        "type": "string",
                        "enum": [
                            "docs-only-example",
                            "assertion",
                            "property-postcondition",
                            "manual-review",
                        ],
                    },
                    "range": {
                        "type": "object",
                        "required": ["start", "end"],
                        "properties": {
                            "start": {"$ref": "#/$defs/position"},
                            "end": {"$ref": "#/$defs/position"},
                        },
                    },
                    "message": {"type": "string"},
                },
            },
        },
    },
    "$defs": {
        "position": {
            "type": "object",
            "required": ["line", "character"],
            "properties": {
                "line": {"type": "integer", "minimum": 0},
                "character": {"type": "integer", "minimum": 0},
            },
        }
    },
}



MANIFEST_OUTPUT_SCHEMA = {
    "type": "object",
    "additionalProperties": False,
    "required": ["schema_version", "nodes", "evidence", "edges"],
    "properties": {
        "schema_version": {"type": "integer", "const": 1},
        "nodes": {"type": "array", "items": node_schema()},
        "reviews": review_registry_schema(),
        "review_evidence_identity": review_evidence_identity_schema(),
        "evidence": {"type": "array", "items": evidence_schema()},
        "edges": {"type": "array", "items": edge_schema()},
    },
}


VALIDATE_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": [
        "status",
        "trace_gaps",
        "open_questions",
        "independent_reviews",
        "contradicting_observations",
        "stale_reviews",
        "stale_evidence",
    ],
    "properties": {
        "status": {"type": "string", "enum": ["pass", "fail", "unknown"]},
        "trace_gaps": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["code", "subject_id"],
                "properties": {
                    "code": {
                        "type": "string",
                        "enum": [
                            "trace-gap.intent-without-claim",
                            "trace-gap.claim-without-test",
                        ],
                    },
                    "subject_id": {"type": "string", "minLength": 1},
                },
            },
        },
        "open_questions": {"type": "integer", "minimum": 0, "maximum": 18446744073709551615},
        "independent_reviews": {
            "type": "integer",
            "minimum": 0,
            "maximum": 18446744073709551615,
        },
        "contradicting_observations": {
            "type": "integer",
            "minimum": 0,
            "maximum": 18446744073709551615,
        },
        "stale_reviews": {"type": "integer", "minimum": 0, "maximum": 18446744073709551615},
        "stale_evidence": {"type": "integer", "minimum": 0, "maximum": 18446744073709551615},
        "review_evidence_identity": {
            "type": "object",
            "additionalProperties": False,
            "required": [
                "subject_digest",
                "source_commit",
                "artifact_digest",
                "trust_store_digest",
                "lifecycle_digest",
                "now",
            ],
            "properties": {
                "subject_digest": {"type": "string", "minLength": 1},
                "source_commit": {"type": "string", "minLength": 1},
                "artifact_digest": {"type": "string", "minLength": 1},
                "trust_store_digest": {"type": ["string", "null"], "minLength": 1},
                "lifecycle_digest": {"type": ["string", "null"], "minLength": 1},
                "now": {"type": "string", "minLength": 1},
            },
        },
        "review_verifications": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["review_id", "state"],
                "properties": {
                    "review_id": {
                        "type": "string",
                        "pattern": r"^review:[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$",
                    },
                    "state": {
                        "type": "string",
                        "enum": ["verified", "unverified", "stale", "revoked"],
                    },
                },
            },
        },
        "manifest": MANIFEST_OUTPUT_SCHEMA,
    },
}


SOURCE_PROPERTIES = {
    "source": {"type": "string"},
    "file": {"type": "string", "minLength": 1},
}

VALIDATE_ARGUMENT_NAMES = frozenset(
    {
        "source",
        "file",
        "manifest",
        "manifest_file",
        "include_manifest",
        "trust_store",
        "review_lifecycle",
        "review_subject_digest",
        "review_source_commit",
        "review_artifact_digest",
        "review_trust_store_digest",
        "review_lifecycle_digest",
        "review_now",
    }
)


TOOLS = [
    tool_descriptor(
        "lsharp_check",
        "L# source を型チェックする (native selfhost subset)",
        SOURCE_PROPERTIES,
        [["source"], ["file"]],
        CHECK_OUTPUT_SCHEMA,
    ),
    tool_descriptor(
        "lsharp_validate",
        (
            "L# source の intent/evidence graph を検証する (native selfhost subset)。"
            "明示した provider snapshot は raw bytes の digest に変換する"
        ),
        {
            **SOURCE_PROPERTIES,
            "manifest": {
                "oneOf": [MANIFEST_OUTPUT_SCHEMA, {"type": "string", "minLength": 1}]
            },
            "manifest_file": {"type": "string", "minLength": 1},
            "include_manifest": {"type": "boolean"},
            "trust_store": {"type": "string", "minLength": 1},
            "review_lifecycle": {"type": "string", "minLength": 1},
            "review_subject_digest": {"type": "string", "minLength": 1},
            "review_source_commit": {"type": "string", "minLength": 1},
            "review_artifact_digest": {"type": "string", "minLength": 1},
            "review_trust_store_digest": {"type": "string", "minLength": 1},
            "review_lifecycle_digest": {"type": "string", "minLength": 1},
            "review_now": {
                "type": "string",
                "minLength": 1,
                "pattern": CANONICAL_UTC_TIMESTAMP_PATTERN,
            },
        },
        [["source"], ["file"], ["manifest"], ["manifest_file"]],
        VALIDATE_OUTPUT_SCHEMA,
        {
            "additionalProperties": False,
            "dependentRequired": {
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
            }
        },
    ),
    tool_descriptor(
        "lsharp_format",
        "L# source を整形する (native selfhost subset)",
        SOURCE_PROPERTIES,
        [["source"], ["file"]],
        {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["formatted"],
            "properties": {"formatted": {"type": "string"}},
        },
    ),
]
TOOL_NAMES = {tool["name"] for tool in TOOLS}


def validate_program(program_value):
    program = pathlib.Path(program_value)
    if not program.is_file() or not os.access(program, os.X_OK):
        raise ShimError(f"program is not a regular executable: {program}")
    return str(program.resolve())


def run_native(program, arguments):
    try:
        completed = subprocess.run(
            [program, *arguments],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
    except OSError as error:
        raise ToolError(f"failed to execute native program: {error}") from error
    if not completed.stdout.strip():
        detail = completed.stderr.strip()
        if completed.returncode:
            raise ToolError(
                f"native program exited with status {completed.returncode}"
                + (f": {detail}" if detail else "")
            )
        raise ToolError("native program returned empty stdout")
    return completed


def parse_json_output(completed):
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise ToolError(f"malformed native JSON: {error}") from error


def require_string(arguments, name):
    value = arguments.get(name)
    if not isinstance(value, str) or not value.strip():
        raise ToolError(f"{name} は空でない文字列が必要です")
    return value


def input_file(arguments, temporary_directory, names=("source", "file")):
    present = [name for name in names if name in arguments]
    if len(present) != 1:
        raise ToolError(f"{' または '.join(names)} のいずれか一つが必要です")
    name = present[0]
    value = require_string(arguments, name)
    if name == "source":
        path = pathlib.Path(temporary_directory) / "input.ls"
        try:
            path.write_text(value, encoding="utf-8")
        except OSError as error:
            raise ToolError(f"native MCP source の一時 file 作成に失敗しました: {error}") from error
        return path
    path = pathlib.Path(value)
    if not path.is_file():
        raise ToolError(f"native MCP input file が見つかりません: {path}")
    return path.resolve()


def validate_input_file(arguments, temporary_directory):
    present = [name for name in ("source", "file", "manifest", "manifest_file") if name in arguments]
    if len(present) != 1:
        raise ToolError("lsharp_validate は source、file、manifest、manifest_file のいずれか一つが必要です")
    name = present[0]
    if name in ("source", "file"):
        return input_file(arguments, temporary_directory, (name,))
    if name == "manifest_file":
        path = pathlib.Path(require_string(arguments, name))
        if not path.is_file():
            raise ToolError(f"native MCP manifest file が見つかりません: {path}")
        return path.resolve()
    value = arguments[name]
    if isinstance(value, dict):
        content = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    elif isinstance(value, str) and value.strip():
        content = value
    else:
        raise ToolError("manifest は JSON object または空でない JSON string が必要です")
    path = pathlib.Path(temporary_directory) / "manifest.json"
    try:
        path.write_text(content, encoding="utf-8")
    except OSError as error:
        raise ToolError(f"native MCP manifest の一時 file 作成に失敗しました: {error}") from error
    return path


def reject_unknown_arguments(arguments, allowed, command_name):
    unknown = sorted(set(arguments).difference(allowed))
    if unknown:
        raise ToolError(f"{command_name} の未知の引数: {', '.join(unknown)}")


def provider_snapshot_arguments(arguments):
    path_names = ("trust_store", "review_lifecycle")
    present = [name for name in path_names if name in arguments]
    if not present:
        return [], set(), {}
    if len(present) != len(path_names):
        raise ToolError("trust_store と review_lifecycle は同時指定が必要です")

    provider_inputs = (
        ("trust_store", "review_trust_store_digest", "--review-trust-store-digest"),
        ("review_lifecycle", "review_lifecycle_digest", "--review-lifecycle-digest"),
    )
    flags = []
    excluded = set()
    digests = {}
    for path_name, digest_name, flag in provider_inputs:
        path = pathlib.Path(require_string(arguments, path_name))
        if path.is_symlink() or not path.is_file():
            raise ToolError(f"native MCP {path_name} snapshot file が見つかりません: {path}")
        try:
            snapshot = path.read_bytes()
        except OSError as error:
            raise ToolError(
                f"native MCP {path_name} snapshot file の読み込みに失敗しました: {error}"
            ) from error
        if not snapshot:
            raise ToolError(f"native MCP {path_name} snapshot file は empty にできません: {path}")
        digest = f"sha256:{hashlib.sha256(snapshot).hexdigest()}"
        if digest_name in arguments:
            expected = require_string(arguments, digest_name)
            if expected != digest:
                raise ToolError(
                    f"{digest_name} digest mismatch: expected {expected}, computed {digest}"
                )
        flags.extend((flag, digest))
        excluded.add(digest_name)
        digests[digest_name] = digest
    return flags, excluded, digests


def identity_arguments(arguments, excluded=()):
    identity_names = (
        "review_subject_digest",
        "review_source_commit",
        "review_artifact_digest",
        "review_now",
    )
    present = [name for name in identity_names if name in arguments]
    if present and len(present) != len(identity_names):
        raise ToolError(
            "review identity requires --review-subject-digest --review-source-commit "
            "--review-artifact-digest --review-now"
        )
    if "review_now" in arguments:
        review_now = require_string(arguments, "review_now")
        if CANONICAL_UTC_TIMESTAMP_RE.fullmatch(review_now) is None:
            raise ToolError(
                "review_now must be a canonical UTC timestamp (YYYY-MM-DDTHH:MM:SSZ)"
            )
    flags = (
        ("review_subject_digest", "--review-subject-digest"),
        ("review_source_commit", "--review-source-commit"),
        ("review_artifact_digest", "--review-artifact-digest"),
        ("review_trust_store_digest", "--review-trust-store-digest"),
        ("review_lifecycle_digest", "--review-lifecycle-digest"),
        ("review_now", "--review-now"),
    )
    result = []
    for name, flag in flags:
        if name in excluded:
            continue
        if name in arguments:
            result.extend((flag, require_string(arguments, name)))
    return result


def expected_review_identity(arguments, provider_digests):
    identity_names = (
        "review_subject_digest",
        "review_source_commit",
        "review_artifact_digest",
        "review_now",
    )
    if not any(name in arguments for name in identity_names):
        return None
    identity = {
        "subject_digest": require_string(arguments, "review_subject_digest"),
        "source_commit": require_string(arguments, "review_source_commit"),
        "artifact_digest": require_string(arguments, "review_artifact_digest"),
    }
    identity["trust_store_digest"] = provider_digests.get(
        "review_trust_store_digest",
        require_string(arguments, "review_trust_store_digest")
        if "review_trust_store_digest" in arguments
        else None,
    )
    identity["lifecycle_digest"] = provider_digests.get(
        "review_lifecycle_digest",
        require_string(arguments, "review_lifecycle_digest")
        if "review_lifecycle_digest" in arguments
        else None,
    )
    identity["now"] = require_string(arguments, "review_now")
    return identity


def verify_identity_projection(
    report, expected_identity, include_manifest, allow_existing_manifest_identity
):
    if expected_identity is None:
        report_has_identity = "review_evidence_identity" in report
        manifest = report.get("manifest") if include_manifest else None
        manifest_has_identity = isinstance(manifest, dict) and "review_evidence_identity" in manifest
        if not allow_existing_manifest_identity and (report_has_identity or manifest_has_identity):
            raise ToolError(
                "native validate returned implicit review_evidence_identity without explicit review context"
            )
        return
    actual_identity = report.get("review_evidence_identity")
    if not isinstance(actual_identity, dict):
        raise ToolError(
            "native validate report review_evidence_identity is missing for explicit review identity"
        )
    if list(actual_identity) != list(expected_identity):
        raise ToolError("native validate report review_evidence_identity field order mismatch")
    if actual_identity != expected_identity:
        raise ToolError("native validate report review_evidence_identity mismatch")
    if include_manifest:
        manifest = report.get("manifest")
        manifest_identity = manifest.get("review_evidence_identity") if isinstance(manifest, dict) else None
        if not isinstance(manifest_identity, dict):
            raise ToolError("native emitted manifest review_evidence_identity is missing")
        if list(manifest_identity) != list(expected_identity):
            raise ToolError("native emitted manifest review_evidence_identity field order mismatch")
        if manifest_identity != expected_identity:
            raise ToolError("native emitted manifest review_evidence_identity mismatch")


def call_check(program, arguments, temporary_directory):
    path = input_file(arguments, temporary_directory)
    completed = run_native(program, ["check", str(path), "--format", "json"])
    return parse_json_output(completed)


def call_validate(program, arguments, temporary_directory):
    reject_unknown_arguments(arguments, VALIDATE_ARGUMENT_NAMES, "lsharp_validate")
    path = validate_input_file(arguments, temporary_directory)
    include_manifest = arguments.get("include_manifest", False)
    if not isinstance(include_manifest, bool):
        raise ToolError("include_manifest は boolean が必要です")
    command = ["validate", "--source", str(path), "--format", "json"]
    provider_flags, provider_digest_names, provider_digests = provider_snapshot_arguments(arguments)
    command.extend(identity_arguments(arguments, provider_digest_names))
    command.extend(provider_flags)
    manifest_path = None
    if include_manifest:
        manifest_path = pathlib.Path(temporary_directory) / "emitted-manifest.json"
        command.extend(("--emit-manifest", str(manifest_path)))
    completed = run_native(program, command)
    report = parse_json_output(completed)
    if include_manifest:
        if manifest_path is None or not manifest_path.is_file():
            raise ToolError("native validate が manifest を生成しませんでした")
        try:
            report["manifest"] = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise ToolError(f"native emitted manifest が不正です: {error}") from error
    verify_identity_projection(
        report,
        expected_review_identity(arguments, provider_digests),
        include_manifest,
        "manifest" in arguments or "manifest_file" in arguments,
    )
    return report


def call_format(program, arguments, temporary_directory):
    path = input_file(arguments, temporary_directory)
    completed = run_native(program, ["fmt", str(path)])
    return {"formatted": completed.stdout}


def call_tool(program, name, arguments):
    if name not in TOOL_NAMES:
        return {"content": [{"type": "text", "text": "tool not found"}], "isError": True}
    if not isinstance(arguments, dict):
        return {"content": [{"type": "text", "text": "arguments は object が必要です"}], "isError": True}
    with tempfile.TemporaryDirectory(prefix="lsharp-native-mcp-") as temporary_directory:
        try:
            if name == "lsharp_check":
                value = call_check(program, arguments, temporary_directory)
            elif name == "lsharp_validate":
                value = call_validate(program, arguments, temporary_directory)
            else:
                value = call_format(program, arguments, temporary_directory)
        except ToolError as error:
            return {"content": [{"type": "text", "text": str(error)}], "isError": True}
    return {
        "content": [{"type": "text", "text": json.dumps(value, ensure_ascii=False)}],
        "structuredContent": value,
        "isError": False,
    }


def jsonrpc_result(request_id, result):
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def jsonrpc_error(request_id, code, message):
    return {"jsonrpc": "2.0", "id": request_id, "error": {"code": code, "message": message}}


def handle_request(program, request):
    if not isinstance(request, dict):
        raise ShimError("MCP request must be a JSON object")
    if request.get("jsonrpc") != "2.0":
        raise ShimError("MCP request jsonrpc must be 2.0")
    request_id = request.get("id")
    method = request.get("method")
    if not isinstance(method, str):
        raise ShimError("MCP request method must be a string")
    if request_id is None:
        return None
    if method == "initialize":
        return jsonrpc_result(
            request_id,
            {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {"tools": {"listChanged": False}},
                "serverInfo": {"name": "lsharp-native", "version": "0.1.0"},
            },
        )
    if method == "ping":
        return jsonrpc_result(request_id, {})
    if method == "tools/list":
        return jsonrpc_result(request_id, {"tools": TOOLS})
    if method == "tools/call":
        params = request.get("params", {})
        if not isinstance(params, dict):
            return jsonrpc_result(
                request_id,
                {"content": [{"type": "text", "text": "params は object が必要です"}], "isError": True},
            )
        name = params.get("name")
        if not isinstance(name, str):
            return jsonrpc_result(
                request_id,
                {"content": [{"type": "text", "text": "tool name が必要です"}], "isError": True},
            )
        return jsonrpc_result(request_id, call_tool(program, name, params.get("arguments", {})))
    return jsonrpc_error(request_id, -32601, f"Method not found: {method}")


def parse_args(argv):
    parser = argparse.ArgumentParser(description="Expose native selfhost tools over MCP stdio")
    parser.add_argument("--program", required=True, metavar="PATH")
    return parser.parse_args(argv)


def write_error(error):
    sys.stderr.write(f"native-selfhost-mcp: {error}\n")
    sys.stderr.flush()


def main(argv=None):
    args = parse_args(argv)
    try:
        program = validate_program(args.program)
        for line in sys.stdin:
            if not line.strip():
                continue
            try:
                request = json.loads(line)
            except json.JSONDecodeError as error:
                raise ShimError(f"invalid JSON: {error}") from error
            response = handle_request(program, request)
            if response is not None:
                sys.stdout.write(json.dumps(response, ensure_ascii=False, separators=(",", ":")) + "\n")
                sys.stdout.flush()
    except (ShimError, OSError) as error:
        write_error(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
