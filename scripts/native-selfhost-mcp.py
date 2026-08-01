#!/usr/bin/env python3
"""Expose the deterministic native selfhost subset over MCP stdio.

The native compiler remains the only implementation authority.  This shim only
translates JSON-RPC requests into the existing ``check``, ``validate``, ``fmt``,
``compile`` and ``lsp --stdio`` CLI contracts; compile/run uses only an explicitly
configured external ``wasmtime`` runtime. It never calls cargo, rustc, host ``lsharp`` or a
provider/network helper.  ``lsharp_errors`` is a read-only documentation
lookup over the canonical Rust error-code table and never executes a host
compiler.  ``lsharp_search`` is an offline projection of local
``.lsharp/packages`` metadata and never accesses a registry.  ``lsharp_project_context``
is an offline projection of local ``lsharp.toml`` and installed package metadata.
``lsharp_package_api`` reads an installed package's existing ``docs/api.json``;
when it is absent, the native program's read-only ``doc --json`` contract
generates the same in-memory API projection without mutating package files.
``lsharp_stdlib_api`` reads the generated repository ``stdlib/api.json``;
when it is absent, the native program's read-only ``doc --json`` contract
generates the same in-memory projection from direct ``stdlib/*.ls`` files
without mutating the standard library.
Explicit provider snapshot paths are an offline bytes-to-digest adapter; signature and lifecycle
semantic verification remain an external provider boundary until a native
verifier is available.
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
    U64_MAX,
    edge_schema,
    evidence_schema,
    node_schema,
    review_evidence_identity_schema,
    review_registry_schema,
)
from native_selfhost_errors import ERRORS_OUTPUT_SCHEMA, ErrorLookupError, call_errors
from native_selfhost_mcp_compile import (
    COMPILE_RUN_OUTPUT_SCHEMA,
    CompileRunError,
    call_compile_run,
)
from native_selfhost_mcp_lsp import (
    COMPLETION_OUTPUT_SCHEMA,
    DEFINITION_OUTPUT_SCHEMA,
    HOVER_OUTPUT_SCHEMA,
    REFERENCES_OUTPUT_SCHEMA,
    CompletionLookupError,
    DefinitionLookupError,
    HoverLookupError,
    ReferencesLookupError,
    call_completion,
    call_definition,
    call_hover,
    call_references,
)
from native_selfhost_mcp_packages import (
    PackageLookupError,
    PACKAGE_API_OUTPUT_SCHEMA,
    PROJECT_CONTEXT_OUTPUT_SCHEMA,
    SEARCH_OUTPUT_SCHEMA,
    call_package_api,
    call_project_context,
    call_search,
    call_stdlib_api,
)
from native_selfhost_json import strict_json_loads

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

FORMAT_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["formatted"],
    "properties": {"formatted": {"type": "string"}},
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
    "source": {"type": "string", "minLength": 1},
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
        "lsharp_hover",
        "native LSP からカーソル位置の型と :doc を返す",
        {
            **SOURCE_PROPERTIES,
            "line": {"type": "integer", "minimum": 0},
            "character": {"type": "integer", "minimum": 0},
            "col": {"type": "integer", "minimum": 0},
        },
        [["line", "character"], ["line", "col"]],
        HOVER_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_definition",
        "native LSP からカーソル位置の定義範囲を返す",
        {
            **SOURCE_PROPERTIES,
            "line": {"type": "integer", "minimum": 0},
            "character": {"type": "integer", "minimum": 0},
            "col": {"type": "integer", "minimum": 0},
        },
        [["line", "character"], ["line", "col"]],
        DEFINITION_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_references",
        "native LSP からカーソル位置の参照範囲を返す",
        {
            **SOURCE_PROPERTIES,
            "line": {"type": "integer", "minimum": 0},
            "character": {"type": "integer", "minimum": 0},
            "col": {"type": "integer", "minimum": 0},
        },
        [["line", "character"], ["line", "col"]],
        REFERENCES_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_completion",
        "native LSP からカーソル位置の補完候補を返す",
        {
            **SOURCE_PROPERTIES,
            "line": {"type": "integer", "minimum": 0},
            "character": {"type": "integer", "minimum": 0},
            "col": {"type": "integer", "minimum": 0},
        },
        [["line", "character"], ["line", "col"]],
        COMPLETION_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_check",
        "L# source を型チェックする (native selfhost subset)",
        SOURCE_PROPERTIES,
        [["source"], ["file"]],
        CHECK_OUTPUT_SCHEMA,
        {"additionalProperties": False},
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
        FORMAT_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_compile_run",
        "native compiler で compile し、外部 wasmtime で実行する (Rust-free boundary)",
        SOURCE_PROPERTIES,
        [["source"], ["file"]],
        COMPILE_RUN_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_errors",
        "L# error code reference を返す (canonical Rust table lookup)",
        {"error_code": {"type": "string", "minLength": 1}},
        ["error_code"],
        ERRORS_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_search",
        "ローカルにインストール済みの L# package を検索する (native offline subset)",
        {
            "project_dir": {"type": "string", "minLength": 1},
            "query": {"type": "string"},
        },
        [[]],
        SEARCH_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_project_context",
        "L# project と local package context を返す (native offline subset)",
        {"project_dir": {"type": "string", "minLength": 1}},
        [[]],
        PROJECT_CONTEXT_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_package_api",
        "インストール済み L# package の docs/api.json または native doc 生成結果を返す",
        {
            "name": {"type": "string", "minLength": 1},
            "project_dir": {"type": "string", "minLength": 1},
        },
        ["name"],
        PACKAGE_API_OUTPUT_SCHEMA,
        {"additionalProperties": False},
    ),
    tool_descriptor(
        "lsharp_stdlib_api",
        "生成済み stdlib API または native doc 生成結果を返す (native offline subset)",
        {"module": {"type": "string", "minLength": 1}},
        [[]],
        PACKAGE_API_OUTPUT_SCHEMA,
        {"additionalProperties": False},
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
        return strict_json_loads(completed.stdout)
    except (json.JSONDecodeError, ValueError) as error:
        raise ToolError(f"malformed native JSON: {error}") from error


def validate_check_position(value, label):
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    for name in ("line", "character"):
        if name not in value:
            raise ToolError(f"{label} is missing field: {name}")
        coordinate = value[name]
        if isinstance(coordinate, bool) or not isinstance(coordinate, int) or coordinate < 0:
            raise ToolError(f"{label}.{name} must be a non-negative integer")


def validate_check_migration_diagnostic(value, index):
    label = f"native check migrationDiagnostics[{index}]"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    required = ("code", "owner", "selectedSemantics", "disposition", "range")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")

    code = value["code"]
    if code not in ("LS2001", "LS2002", "LS2003"):
        raise ToolError(f"{label}.code must be one of LS2001, LS2002, LS2003")
    if not isinstance(value["owner"], str):
        raise ToolError(f"{label}.owner must be a string")
    if value["selectedSemantics"] not in (
        "legacy-example-truthiness",
        "legacy-invariant-deterministic-smoke",
    ):
        raise ToolError(f"{label}.selectedSemantics has an invalid value")
    if value["disposition"] not in (
        "docs-only-example",
        "assertion",
        "property-postcondition",
        "manual-review",
    ):
        raise ToolError(f"{label}.disposition has an invalid value")

    range_value = value["range"]
    if not isinstance(range_value, dict):
        raise ToolError(f"{label}.range must be an object")
    for name in ("start", "end"):
        if name not in range_value:
            raise ToolError(f"{label}.range is missing field: {name}")
        validate_check_position(range_value[name], f"{label}.range.{name}")
    if "message" in value and not isinstance(value["message"], str):
        raise ToolError(f"{label}.message must be a string")


def validate_check_output(value):
    if not isinstance(value, dict):
        raise ToolError("native check report root must be a JSON object")
    required = ("ok", "diagnostics", "migrationDiagnostics")
    unknown = sorted(set(value).difference(required))
    if unknown:
        raise ToolError(f"native check report has unknown field: {unknown[0]}")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"native check report is missing field: {missing[0]}")
    if not isinstance(value["ok"], bool):
        raise ToolError("native check report ok must be a boolean")
    for name in ("diagnostics", "migrationDiagnostics"):
        if not isinstance(value[name], list):
            raise ToolError(f"native check report {name} must be an array")
    for index, diagnostic in enumerate(value["migrationDiagnostics"]):
        validate_check_migration_diagnostic(diagnostic, index)


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


def require_manifest_object(path, label):
    try:
        content = path.read_text(encoding="utf-8")
        value = strict_json_loads(content)
    except (OSError, json.JSONDecodeError, ValueError) as error:
        raise ToolError(f"{label} は有効な JSON object が必要です: {error}") from error
    if not isinstance(value, dict):
        raise ToolError(f"{label} は JSON object が必要です")


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
        path = path.resolve()
        require_manifest_object(path, "manifest_file")
        return path
    value = arguments[name]
    if isinstance(value, dict):
        content = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    elif isinstance(value, str) and value.strip():
        try:
            parsed = strict_json_loads(value)
        except (json.JSONDecodeError, ValueError) as error:
            raise ToolError(f"manifest は有効な JSON object が必要です: {error}") from error
        if not isinstance(parsed, dict):
            raise ToolError("manifest は JSON object が必要です")
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


def validate_manifest_output(value):
    if not isinstance(value, dict):
        raise ToolError("native emitted manifest root must be a JSON object")
    allowed = {"schema_version", "nodes", "reviews", "review_evidence_identity", "evidence", "edges"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"native emitted manifest has unknown field: {unknown[0]}")
    required = ("schema_version", "nodes", "evidence", "edges")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"native emitted manifest is missing field: {missing[0]}")
    if isinstance(value["schema_version"], bool) or value["schema_version"] != 1:
        raise ToolError("native emitted manifest schema_version must be 1")
    for name in ("nodes", "evidence", "edges"):
        if not isinstance(value[name], list):
            raise ToolError(f"native emitted manifest {name} must be an array")
    for index, node in enumerate(value["nodes"]):
        validate_manifest_node(node, index)
    if "reviews" in value:
        if not isinstance(value["reviews"], list):
            raise ToolError("native emitted manifest reviews must be an array")
        for index, review in enumerate(value["reviews"]):
            validate_manifest_review(review, index)
    for index, edge in enumerate(value["edges"]):
        validate_manifest_edge(edge, index)
    for index, evidence in enumerate(value["evidence"]):
        validate_manifest_evidence(evidence, index)
    validate_manifest_referential_integrity(value)


def validate_manifest_non_empty_string(value, label):
    if not isinstance(value, str) or len(value) < 1:
        raise ToolError(f"{label} must be a non-empty string")


def validate_manifest_identifier(value, label):
    if not isinstance(value, str) or re.fullmatch(r"[A-Za-z0-9_.-]+", value) is None:
        raise ToolError(f"{label} must match the identifier pattern")


def validate_manifest_integer(value, label):
    if (
        isinstance(value, bool)
        or not isinstance(value, int)
        or value < 0
        or value > 18446744073709551615
    ):
        raise ToolError(f"{label} must be a non-negative integer")


def validate_manifest_span(value, label):
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"start", "end"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    missing = [name for name in ("start", "end") if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    for name in ("start", "end"):
        validate_manifest_integer(value[name], f"{label}.{name}")


def validate_manifest_node(value, index):
    label = f"native emitted manifest nodes[{index}]"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"kind", "namespace", "key", "text", "span"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    required = ("kind", "namespace", "key", "text")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    if value["kind"] not in ("intent", "claim", "assumption", "open-question"):
        raise ToolError(f"{label}.kind has invalid value")
    validate_manifest_identifier(value["namespace"], f"{label}.namespace")
    validate_manifest_identifier(value["key"], f"{label}.key")
    validate_manifest_non_empty_string(value["text"], f"{label}.text")
    if "span" in value:
        validate_manifest_span(value["span"], f"{label}.span")


def validate_manifest_review(value, index):
    label = f"native emitted manifest reviews[{index}]"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {
        "namespace",
        "key",
        "provenance_digest",
        "visibility",
        "verification_state",
    }
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    required = ("namespace", "key", "provenance_digest", "visibility")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    validate_manifest_identifier(value["namespace"], f"{label}.namespace")
    validate_manifest_identifier(value["key"], f"{label}.key")
    validate_manifest_non_empty_string(
        value["provenance_digest"], f"{label}.provenance_digest"
    )
    if value["visibility"] not in ("public", "redacted"):
        raise ToolError(f"{label}.visibility has invalid value")
    if "verification_state" in value and value["verification_state"] not in (
        "verified",
        "unverified",
        "stale",
        "revoked",
    ):
        raise ToolError(f"{label}.verification_state has invalid value")


def validate_manifest_id(value, label):
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"namespace", "key"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    missing = [name for name in ("namespace", "key") if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    validate_manifest_identifier(value["namespace"], f"{label}.namespace")
    validate_manifest_identifier(value["key"], f"{label}.key")


def validate_manifest_subject(value, label, kinds):
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"kind", "namespace", "key"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    missing = [name for name in ("kind", "namespace", "key") if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    if value["kind"] not in kinds:
        raise ToolError(f"{label}.kind has invalid value")
    validate_manifest_identifier(value["namespace"], f"{label}.namespace")
    validate_manifest_identifier(value["key"], f"{label}.key")


def validate_manifest_edge(value, index):
    label = f"native emitted manifest edges[{index}]"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    if "relation" not in value:
        raise ToolError(f"{label} is missing field: relation")

    relation = value["relation"]
    relation_fields = {
        "motivates": (("intent", "claim"), None),
        "constrained-by": (("claim", "assumption"), None),
        "tested-by": (("claim", "contract"), None),
        "supports": (("observation", "claim"), None),
        "contradicts": (("observation", "claim"), None),
        "evaluates": (("review", "subject"), ("intent", "claim", "evidence")),
        "invalidates": (("change", "subject"), ("evidence", "review")),
    }
    if relation not in relation_fields:
        raise ToolError(f"{label}.relation has invalid value")
    fields, subject_kinds = relation_fields[relation]
    allowed = {"relation", *fields}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    missing = [name for name in fields if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    for name in fields:
        if name == "subject":
            validate_manifest_subject(value[name], f"{label}.{name}", subject_kinds)
        else:
            validate_manifest_id(value[name], f"{label}.{name}")


def validate_manifest_sampling(value, label):
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"cases", "seed", "generator", "shrinks", "coverage"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    required = ("cases", "seed", "generator")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    validate_manifest_integer(value["cases"], f"{label}.cases")
    validate_manifest_integer(value["seed"], f"{label}.seed")
    validate_manifest_non_empty_string(value["generator"], f"{label}.generator")
    if "shrinks" in value:
        if not isinstance(value["shrinks"], list):
            raise ToolError(f"{label}.shrinks must be an array")
        for index, shrink in enumerate(value["shrinks"]):
            validate_manifest_integer(shrink, f"{label}.shrinks[{index}]")
    if "coverage" in value:
        coverage = value["coverage"]
        if not isinstance(coverage, dict):
            raise ToolError(f"{label}.coverage must be an object")
        covered = 0
        for name, count in coverage.items():
            if re.search(r"\S", name) is None:
                raise ToolError(f"{label}.coverage has an invalid property name")
            validate_manifest_integer(count, f"{label}.coverage[{name}]")
            if covered > 18446744073709551615 - count:
                raise ToolError(f"{label}.coverage total exceeds u64 range")
            covered += count
        if coverage and covered != value["cases"]:
            raise ToolError(
                f"{label}.coverage total must equal cases: "
                f"cases={value['cases']}, covered={covered}"
            )


def validate_manifest_execution(value, label):
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"runner", "target", "source_commit", "artifact_digest", "sampling"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    required = ("runner", "target", "source_commit", "artifact_digest", "sampling")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    for name in ("runner", "target", "source_commit", "artifact_digest"):
        validate_manifest_non_empty_string(value[name], f"{label}.{name}")
    validate_manifest_sampling(value["sampling"], f"{label}.sampling")


def validate_manifest_provenance(value, label):
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"producer", "tool_version", "timestamp"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    required = ("producer", "tool_version", "timestamp")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    for name in required:
        validate_manifest_non_empty_string(value[name], f"{label}.{name}")


def validate_manifest_evidence(value, index):
    label = f"native emitted manifest evidence[{index}]"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {
        "namespace",
        "key",
        "method",
        "subject",
        "outcome",
        "execution",
        "provenance",
        "independence",
    }
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    required = (
        "namespace",
        "key",
        "method",
        "subject",
        "outcome",
        "execution",
        "provenance",
        "independence",
    )
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    validate_manifest_identifier(value["namespace"], f"{label}.namespace")
    validate_manifest_identifier(value["key"], f"{label}.key")
    if value["method"] not in (
        "example",
        "case",
        "assert",
        "property",
        "production",
        "reference",
        "proof",
        "review",
    ):
        raise ToolError(f"{label}.method has invalid value")
    validate_manifest_subject(
        value["subject"], f"{label}.subject", ("intent", "claim", "contract")
    )
    if value["outcome"] not in ("pass", "fail", "contradicted", "unknown", "stale"):
        raise ToolError(f"{label}.outcome has invalid value")
    validate_manifest_execution(value["execution"], f"{label}.execution")
    validate_manifest_provenance(value["provenance"], f"{label}.provenance")
    if value["independence"] not in (
        "same-author",
        "independent-review",
        "external-observation",
    ):
        raise ToolError(f"{label}.independence has invalid value")


def validate_manifest_referential_integrity(value):
    """emitted manifest の typed graph endpoint を同じ manifest 内へ閉じ込める。"""
    nodes = {}
    for index, node in enumerate(value["nodes"]):
        identifier = (node["namespace"], node["key"])
        if identifier in nodes:
            raise ToolError(
                f"native emitted manifest nodes[{index}] duplicates ID: "
                f"{format_manifest_identifier(identifier)}"
            )
        nodes[identifier] = node["kind"]

    evidence = {}
    for index, item in enumerate(value["evidence"]):
        identifier = (item["namespace"], item["key"])
        if identifier in evidence:
            raise ToolError(
                f"native emitted manifest evidence[{index}] duplicates ID: "
                f"{format_manifest_identifier(identifier)}"
            )
        evidence[identifier] = True

    reviews = None
    if "reviews" in value:
        reviews = {}
        for index, review in enumerate(value["reviews"]):
            identifier = (review["namespace"], review["key"])
            if identifier in reviews:
                raise ToolError(
                    f"native emitted manifest reviews[{index}] duplicates ID: "
                    f"{format_manifest_identifier(identifier)}"
                )
            reviews[identifier] = True

    for index, item in enumerate(value["evidence"]):
        subject = item["subject"]
        if subject["kind"] in ("intent", "claim"):
            require_manifest_node_reference(
                nodes,
                subject,
                subject["kind"],
                f"native emitted manifest evidence[{index}].subject",
            )

    for index, edge in enumerate(value["edges"]):
        label = f"native emitted manifest edges[{index}]"
        relation = edge["relation"]
        if relation == "motivates":
            require_manifest_node_reference(nodes, edge["intent"], "intent", f"{label}.intent")
            require_manifest_node_reference(nodes, edge["claim"], "claim", f"{label}.claim")
        elif relation == "constrained-by":
            require_manifest_node_reference(nodes, edge["claim"], "claim", f"{label}.claim")
            require_manifest_node_reference(
                nodes, edge["assumption"], "assumption", f"{label}.assumption"
            )
        elif relation == "tested-by":
            require_manifest_node_reference(nodes, edge["claim"], "claim", f"{label}.claim")
        elif relation in ("supports", "contradicts"):
            require_manifest_evidence_reference(
                evidence, edge["observation"], f"{label}.observation"
            )
            require_manifest_node_reference(nodes, edge["claim"], "claim", f"{label}.claim")
        elif relation == "evaluates":
            if reviews is not None:
                require_manifest_review_reference(reviews, edge["review"], f"{label}.review")
            require_manifest_subject_reference(nodes, evidence, edge["subject"], f"{label}.subject")
        elif relation == "invalidates":
            require_manifest_subject_reference(nodes, evidence, edge["subject"], f"{label}.subject")
            if reviews is not None and edge["subject"]["kind"] == "review":
                require_manifest_review_reference(
                    reviews, edge["subject"], f"{label}.subject"
                )


def format_manifest_identifier(identifier):
    return f"{identifier[0]}/{identifier[1]}"


def require_manifest_node_reference(nodes, reference, expected_kind, label):
    identifier = (reference["namespace"], reference["key"])
    actual_kind = nodes.get(identifier)
    if actual_kind is None:
        raise ToolError(
            f"{label} references missing node: {format_manifest_identifier(identifier)}"
        )
    if actual_kind != expected_kind:
        raise ToolError(
            f"{label} references node kind {actual_kind}, expected {expected_kind}: "
            f"{format_manifest_identifier(identifier)}"
        )


def require_manifest_evidence_reference(evidence, reference, label):
    identifier = (reference["namespace"], reference["key"])
    if identifier not in evidence:
        raise ToolError(
            f"{label} references missing evidence: {format_manifest_identifier(identifier)}"
        )


def require_manifest_review_reference(reviews, reference, label):
    identifier = (reference["namespace"], reference["key"])
    if identifier not in reviews:
        raise ToolError(
            f"{label} references missing review: {format_manifest_identifier(identifier)}"
        )


def require_manifest_subject_reference(nodes, evidence, subject, label):
    kind = subject["kind"]
    if kind in ("intent", "claim"):
        require_manifest_node_reference(nodes, subject, kind, label)
    elif kind == "evidence":
        require_manifest_evidence_reference(evidence, subject, label)


def validate_trace_gap(value, index):
    label = f"native validate report trace_gaps[{index}]"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"code", "subject_id"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    missing = [name for name in ("code", "subject_id") if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    if value["code"] not in (
        "trace-gap.intent-without-claim",
        "trace-gap.claim-without-test",
    ):
        raise ToolError(f"{label}.code must be one of trace-gap.intent-without-claim, trace-gap.claim-without-test")
    subject_id = value["subject_id"]
    if not isinstance(subject_id, str) or len(subject_id) < 1:
        raise ToolError(f"{label}.subject_id must be a non-empty string")


def validate_review_verification(value, index):
    label = f"native validate report review_verifications[{index}]"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    allowed = {"review_id", "state"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    missing = [name for name in ("review_id", "state") if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    review_id = value["review_id"]
    if not isinstance(review_id, str) or re.fullmatch(
        r"review:[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", review_id
    ) is None:
        raise ToolError(f"{label}.review_id has invalid format")
    if value["state"] not in ("verified", "unverified", "stale", "revoked"):
        raise ToolError(f"{label}.state must be one of verified, unverified, stale, revoked")


def validate_review_evidence_identity(value):
    label = "native validate report review_evidence_identity"
    if not isinstance(value, dict):
        raise ToolError(f"{label} must be an object")
    required = (
        "subject_digest",
        "source_commit",
        "artifact_digest",
        "trust_store_digest",
        "lifecycle_digest",
        "now",
    )
    allowed = set(required)
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"{label} has unknown field: {unknown[0]}")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"{label} is missing field: {missing[0]}")
    for name in required:
        identity_value = value[name]
        if name in ("trust_store_digest", "lifecycle_digest") and identity_value is None:
            continue
        if not isinstance(identity_value, str) or len(identity_value) < 1:
            if name in ("trust_store_digest", "lifecycle_digest"):
                raise ToolError(f"{label}.{name} must be a string or null")
            raise ToolError(f"{label}.{name} must be a non-empty string")


def validate_report_counter(value, name):
    if isinstance(value, bool) or not isinstance(value, int) or value < 0 or value > U64_MAX:
        raise ToolError(f"native validate report {name} must be a non-negative integer")


def validate_report_output(value):
    if not isinstance(value, dict):
        raise ToolError("native validate report root must be a JSON object")
    required = (
        "status",
        "trace_gaps",
        "open_questions",
        "independent_reviews",
        "contradicting_observations",
        "stale_reviews",
        "stale_evidence",
    )
    allowed = set(required) | {"review_evidence_identity", "review_verifications", "manifest"}
    unknown = sorted(set(value).difference(allowed))
    if unknown:
        raise ToolError(f"native validate report has unknown field: {unknown[0]}")
    missing = [name for name in required if name not in value]
    if missing:
        raise ToolError(f"native validate report is missing field: {missing[0]}")
    if value["status"] not in {"pass", "fail", "unknown"}:
        raise ToolError("native validate report status is invalid")
    for name in ("trace_gaps", "review_verifications"):
        if name in value and not isinstance(value[name], list):
            raise ToolError(f"native validate report {name} must be an array")
    for index, gap in enumerate(value["trace_gaps"]):
        validate_trace_gap(gap, index)
    for index, verification in enumerate(value.get("review_verifications", [])):
        validate_review_verification(verification, index)
    for name in (
        "open_questions",
        "independent_reviews",
        "contradicting_observations",
        "stale_reviews",
        "stale_evidence",
    ):
        validate_report_counter(value[name], name)
    if "review_evidence_identity" in value and not isinstance(value["review_evidence_identity"], dict):
        raise ToolError("native validate report review_evidence_identity must be an object")
    if "review_evidence_identity" in value:
        validate_review_evidence_identity(value["review_evidence_identity"])
    if "manifest" in value:
        validate_manifest_output(value["manifest"])


def reject_provider_semantic_states(report, provider_digests):
    if not provider_digests:
        return
    semantic_states = [
        verification["state"]
        for verification in report.get("review_verifications", [])
        if verification["state"] != "unverified"
    ]
    if semantic_states:
        raise ToolError(
            "provider semantic verification is unavailable; "
            "review_verifications must remain unverified"
        )


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
    reject_unknown_arguments(arguments, {"source", "file"}, "lsharp_check")
    path = input_file(arguments, temporary_directory)
    completed = run_native(program, ["check", str(path), "--format", "json"])
    value = parse_json_output(completed)
    validate_check_output(value)
    return value


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
    validate_report_output(report)
    reject_provider_semantic_states(report, provider_digests)
    if include_manifest:
        if manifest_path is None or not manifest_path.is_file():
            raise ToolError("native validate が manifest を生成しませんでした")
        try:
            manifest = strict_json_loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError, ValueError) as error:
            raise ToolError(f"native emitted manifest が不正です: {error}") from error
        validate_manifest_output(manifest)
        report["manifest"] = manifest
        validate_report_output(report)
    verify_identity_projection(
        report,
        expected_review_identity(arguments, provider_digests),
        include_manifest,
        "manifest" in arguments or "manifest_file" in arguments,
    )
    return report


def call_format(program, arguments, temporary_directory):
    reject_unknown_arguments(arguments, {"source", "file"}, "lsharp_format")
    path = input_file(arguments, temporary_directory)
    completed = run_native(program, ["fmt", str(path)])
    if completed.returncode:
        detail = completed.stderr.strip()
        raise ToolError(
            f"native format exited with status {completed.returncode}"
            + (f": {detail}" if detail else "")
        )
    return {"formatted": completed.stdout}


def call_tool(program, name, arguments):
    if name not in TOOL_NAMES:
        return {"content": [{"type": "text", "text": "tool not found"}], "isError": True}
    if not isinstance(arguments, dict):
        return {"content": [{"type": "text", "text": "arguments は object が必要です"}], "isError": True}
    with tempfile.TemporaryDirectory(prefix="lsharp-native-mcp-") as temporary_directory:
        try:
            if name == "lsharp_hover":
                try:
                    value = call_hover(program, arguments, temporary_directory)
                except HoverLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_definition":
                try:
                    value = call_definition(program, arguments, temporary_directory)
                except DefinitionLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_references":
                try:
                    value = call_references(program, arguments, temporary_directory)
                except ReferencesLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_completion":
                try:
                    value = call_completion(program, arguments, temporary_directory)
                except CompletionLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_check":
                value = call_check(program, arguments, temporary_directory)
            elif name == "lsharp_validate":
                value = call_validate(program, arguments, temporary_directory)
            elif name == "lsharp_errors":
                try:
                    value = call_errors(arguments)
                except ErrorLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_search":
                try:
                    value = call_search(arguments)
                except PackageLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_project_context":
                try:
                    value = call_project_context(arguments)
                except PackageLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_package_api":
                try:
                    value = call_package_api(program, arguments)
                except PackageLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_stdlib_api":
                try:
                    value = call_stdlib_api(program, arguments)
                except PackageLookupError as error:
                    raise ToolError(str(error)) from error
            elif name == "lsharp_compile_run":
                try:
                    value = call_compile_run(program, arguments, temporary_directory)
                except CompileRunError as error:
                    raise ToolError(str(error)) from error
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
                "serverInfo": {"name": "lsharp", "version": "0.1.0"},
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
                request = strict_json_loads(line)
            except (json.JSONDecodeError, ValueError) as error:
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
