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
import subprocess
import sys
import tempfile


MCP_PROTOCOL_VERSION = "2025-11-25"


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
        "migrationDiagnostics": {"type": "array"},
    },
}

VALIDATE_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
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
        "trace_gaps": {"type": "array"},
        "open_questions": {"type": "integer", "minimum": 0},
        "independent_reviews": {"type": "integer", "minimum": 0},
        "contradicting_observations": {"type": "integer", "minimum": 0},
        "stale_reviews": {"type": "integer", "minimum": 0},
        "stale_evidence": {"type": "integer", "minimum": 0},
    },
}


SOURCE_PROPERTIES = {
    "source": {"type": "string"},
    "file": {"type": "string", "minLength": 1},
}


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
            "manifest": {"oneOf": [{"type": "object"}, {"type": "string"}]},
            "manifest_file": {"type": "string", "minLength": 1},
            "include_manifest": {"type": "boolean"},
            "trust_store": {"type": "string", "minLength": 1},
            "review_lifecycle": {"type": "string", "minLength": 1},
            "review_subject_digest": {"type": "string", "minLength": 1},
            "review_source_commit": {"type": "string", "minLength": 1},
            "review_artifact_digest": {"type": "string", "minLength": 1},
            "review_trust_store_digest": {"type": "string", "minLength": 1},
            "review_lifecycle_digest": {"type": "string", "minLength": 1},
            "review_now": {"type": "string", "minLength": 1},
        },
        [["source"], ["file"], ["manifest"], ["manifest_file"]],
        VALIDATE_OUTPUT_SCHEMA,
        {
            "dependentRequired": {
                "trust_store": ["review_lifecycle"],
                "review_lifecycle": ["trust_store"],
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


def provider_snapshot_arguments(arguments):
    path_names = ("trust_store", "review_lifecycle")
    present = [name for name in path_names if name in arguments]
    if not present:
        return [], set()
    if len(present) != len(path_names):
        raise ToolError("trust_store と review_lifecycle は同時指定が必要です")

    provider_inputs = (
        ("trust_store", "review_trust_store_digest", "--review-trust-store-digest"),
        ("review_lifecycle", "review_lifecycle_digest", "--review-lifecycle-digest"),
    )
    flags = []
    excluded = set()
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
    return flags, excluded


def identity_arguments(arguments, excluded=()):
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


def call_check(program, arguments, temporary_directory):
    path = input_file(arguments, temporary_directory)
    completed = run_native(program, ["check", str(path), "--format", "json"])
    return parse_json_output(completed)


def call_validate(program, arguments, temporary_directory):
    path = validate_input_file(arguments, temporary_directory)
    include_manifest = arguments.get("include_manifest", False)
    if not isinstance(include_manifest, bool):
        raise ToolError("include_manifest は boolean が必要です")
    command = ["validate", "--source", str(path), "--format", "json"]
    provider_flags, provider_digest_names = provider_snapshot_arguments(arguments)
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
