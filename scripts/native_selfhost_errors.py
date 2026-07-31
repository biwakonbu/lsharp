"""Canonical Rust error-code projection used by the native MCP shim."""

import json
import pathlib
import re


ERROR_REFERENCE_DOC = "docs/guides/error-reference.md"
ERROR_CODE_SOURCE = pathlib.Path(__file__).resolve().parent.parent / "crates/lsharp-driver/src/error_codes.rs"
RUST_ENTRY_RE = re.compile(r"ErrorCodeEntry\s*\{(?P<body>.*?)\n\s*\},", re.DOTALL)
RUST_STRING_FIELD_RE = re.compile(
    r"\b(?P<field>code|legacy_code|name|summary|detail|fix):\s*"
    r"(?:(?:Some)\s*\(\s*)?\"(?P<value>(?:\\.|[^\"\\])*)\""
)


class ErrorLookupError(Exception):
    """Canonical table failures are reported as MCP tool errors."""


ERRORS_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["code", "name", "description", "fix", "doc"],
    "properties": {
        "code": {"type": "string", "minLength": 1},
        "legacy_code": {"type": ["string", "null"], "minLength": 1},
        "name": {"type": "string", "minLength": 1},
        "description": {"type": "string", "minLength": 1},
        "detail": {"type": "string", "minLength": 1},
        "fix": {"type": "string", "minLength": 1},
        "doc": {"type": "string", "const": ERROR_REFERENCE_DOC},
    },
}


def _decode_rust_string(value):
    try:
        return json.loads('"' + value + '"')
    except json.JSONDecodeError as error:
        raise ErrorLookupError(f"canonical Rust error table contains invalid string escape: {error}") from error


def load_error_table():
    try:
        source = ERROR_CODE_SOURCE.read_text(encoding="utf-8")
    except OSError as error:
        raise ErrorLookupError(
            f"canonical Rust error table is unavailable: {ERROR_CODE_SOURCE}: {error}"
        ) from error

    entries = {}
    for match in RUST_ENTRY_RE.finditer(source):
        fields = {
            field_match.group("field"): _decode_rust_string(field_match.group("value"))
            for field_match in RUST_STRING_FIELD_RE.finditer(match.group("body"))
        }
        if "code" not in fields:
            continue
        legacy_match = re.search(
            r"\blegacy_code:\s*(?:Some\s*\(\s*)?\"(?P<value>(?:\\.|[^\"\\])*)\"",
            match.group("body"),
        )
        fields["legacy_code"] = (
            _decode_rust_string(legacy_match.group("value")) if legacy_match else None
        )
        if {"code", "name", "summary", "detail", "fix"}.issubset(fields):
            entries[fields["code"]] = fields
    if not entries:
        raise ErrorLookupError(f"canonical Rust error table is empty: {ERROR_CODE_SOURCE}")
    return entries


def call_errors(arguments):
    unknown = sorted(set(arguments).difference({"error_code"}))
    if unknown:
        raise ErrorLookupError(f"lsharp_errors の未知の引数: {', '.join(unknown)}")
    code = arguments.get("error_code")
    if not isinstance(code, str) or not code.strip():
        raise ErrorLookupError("error_code は空でない文字列が必要です")
    entries = load_error_table()
    entry = entries.get("LS1002" if code == "E0003" else code)
    if entry is None:
        entry = next((candidate for candidate in entries.values() if candidate["legacy_code"] == code), None)
    if entry is None:
        return {
            "code": code,
            "name": "unknown",
            "description": "未知のエラーコードです",
            "fix": "最新版ドキュメントを確認してください",
            "doc": ERROR_REFERENCE_DOC,
        }
    return {
        "code": entry["code"],
        "legacy_code": entry["legacy_code"],
        "name": entry["name"],
        "description": entry["summary"],
        "detail": entry["detail"],
        "fix": entry["fix"],
        "doc": ERROR_REFERENCE_DOC,
    }
