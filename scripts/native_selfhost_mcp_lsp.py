"""Native LSP-backed projections for the MCP shim."""

import importlib.util
import json
import pathlib


class HoverLookupError(Exception):
    """A native LSP hover request failed or returned an invalid result."""


class DefinitionLookupError(Exception):
    """A native LSP definition request failed or returned an invalid result."""


class ReferencesLookupError(Exception):
    """A native LSP references request failed or returned an invalid result."""


HOVER_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["name", "type", "doc"],
    "properties": {
        "name": {"type": "string", "minLength": 1},
        "type": {"type": "string", "minLength": 1},
        "doc": {"type": ["string", "null"]},
    },
}

DEFINITION_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["start", "end"],
    "properties": {
        "start": {
            "type": "object",
            "additionalProperties": False,
            "required": ["line", "character"],
            "properties": {
                "line": {"type": "integer", "minimum": 0},
                "character": {"type": "integer", "minimum": 0},
            },
        },
        "end": {
            "type": "object",
            "additionalProperties": False,
            "required": ["line", "character"],
            "properties": {
                "line": {"type": "integer", "minimum": 0},
                "character": {"type": "integer", "minimum": 0},
            },
        },
    },
}

REFERENCES_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["count", "ranges"],
    "properties": {
        "count": {"type": "integer", "minimum": 0},
        "ranges": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["start", "end"],
                "properties": {
                    "start": {
                        "type": "object",
                        "additionalProperties": False,
                        "required": ["line", "character"],
                        "properties": {
                            "line": {"type": "integer", "minimum": 0},
                            "character": {"type": "integer", "minimum": 0},
                        },
                    },
                    "end": {
                        "type": "object",
                        "additionalProperties": False,
                        "required": ["line", "character"],
                        "properties": {
                            "line": {"type": "integer", "minimum": 0},
                            "character": {"type": "integer", "minimum": 0},
                        },
                    },
                },
            },
        },
    },
}

_NATIVE_LSP_HELPER = None


def _native_lsp_helper(error_type=HoverLookupError):
    global _NATIVE_LSP_HELPER
    if _NATIVE_LSP_HELPER is None:
        helper_path = pathlib.Path(__file__).resolve().with_name("native-selfhost-lsp-stdio.py")
        spec = importlib.util.spec_from_file_location("native_selfhost_lsp_stdio", helper_path)
        if spec is None or spec.loader is None:
            raise error_type(f"native LSP helper を読み込めません: {helper_path}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        _NATIVE_LSP_HELPER = module
    return _NATIVE_LSP_HELPER


def _require_source(arguments, temporary_directory, error_type=HoverLookupError):
    present = [name for name in ("source", "file") if name in arguments]
    if len(present) != 1:
        raise error_type("source または file のいずれか一つが必要です")
    name = present[0]
    value = arguments[name]
    if not isinstance(value, str) or not value.strip():
        raise error_type(f"{name} は空でない文字列が必要です")
    if name == "source":
        source = value
        uri = (pathlib.Path(temporary_directory) / "hover.ls").resolve().as_uri()
        return source, uri
    path = pathlib.Path(value)
    if not path.is_file():
        raise error_type(f"native MCP input file が見つかりません: {path}")
    try:
        source = path.read_text(encoding="utf-8")
    except OSError as error:
        raise error_type(f"native MCP input file の読み込みに失敗しました: {error}") from error
    return source, path.resolve().as_uri()


def _position(arguments, error_type=HoverLookupError):
    line = arguments.get("line")
    if type(line) is not int or line < 0:
        raise error_type("line は 0 以上の整数が必要です")
    if "character" in arguments:
        character = arguments["character"]
    elif "col" in arguments:
        character = arguments["col"]
    else:
        raise error_type("character が必要です")
    if type(character) is not int or character < 0:
        raise error_type("character は 0 以上の整数が必要です")
    return {"line": line, "character": character}


def _frame(body):
    return b"Content-Length: " + str(len(body)).encode("ascii") + b"\r\n\r\n" + body


def _request(request_id, method, params=None):
    payload = {"jsonrpc": "2.0", "id": request_id, "method": method}
    if params is not None:
        payload["params"] = params
    return _frame(json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))


def _notification(method, params=None):
    payload = {"jsonrpc": "2.0", "method": method}
    if params is not None:
        payload["params"] = params
    return _frame(json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8"))


def _frame_body(raw_frame, error_type=HoverLookupError):
    header_end = raw_frame.find(b"\r\n\r\n")
    if header_end < 0:
        raise error_type("native LSP frame header が不正です")
    length = None
    for raw_line in raw_frame[:header_end].split(b"\r\n"):
        if b":" not in raw_line:
            raise error_type("native LSP frame header が不正です")
        name, value = raw_line.split(b":", 1)
        if name.lower() == b"content-length":
            if length is not None:
                raise error_type("native LSP frame の Content-Length が重複しています")
            try:
                length = int(value.strip())
            except ValueError as error:
                raise error_type("native LSP frame の Content-Length が不正です") from error
    if length is None:
        raise error_type("native LSP frame の Content-Length がありません")
    body_start = header_end + 4
    body = raw_frame[body_start:]
    if len(body) != length:
        raise error_type("native LSP frame body の長さが不一致です")
    return body


def _contents_text(result):
    contents = result.get("contents") if isinstance(result, dict) else result
    if isinstance(contents, str):
        return contents
    if isinstance(contents, dict) and isinstance(contents.get("value"), str):
        return contents["value"]
    if isinstance(contents, list):
        values = []
        for item in contents:
            if isinstance(item, str):
                values.append(item)
            elif isinstance(item, dict) and isinstance(item.get("value"), str):
                values.append(item["value"])
        if values:
            return "\n".join(values)
    raise HoverLookupError("native LSP hover contents を文字列へ変換できませんでした")


def _project_hover(result):
    text = _contents_text(result)
    lines = text.splitlines()
    if not lines or " : " not in lines[0]:
        raise HoverLookupError("native LSP hover signature が不正です")
    name, type_text = lines[0].split(" : ", 1)
    if not name.strip() or not type_text.strip():
        raise HoverLookupError("native LSP hover signature が空です")
    doc = "\n".join(line for line in lines[1:] if line.strip()) or None
    return {"name": name, "type": type_text, "doc": doc}


def _run_lsp_request(
    program,
    arguments,
    temporary_directory,
    method,
    label,
    error_type,
    request_extra=None,
    allow_none=False,
):
    source, uri = _require_source(arguments, temporary_directory, error_type)
    position = _position(arguments, error_type)
    request_params = {"textDocument": {"uri": uri}, "position": position}
    if request_extra:
        request_params.update(request_extra)
    aggregate = b"".join(
        [
            _request(1, "initialize", {"capabilities": {}, "rootUri": None}),
            _notification("initialized", {}),
            _notification(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "lsharp",
                        "version": 1,
                        "text": source,
                    }
                },
            ),
            _request(
                2,
                method,
                request_params,
            ),
        ]
    )
    helper = _native_lsp_helper(error_type)
    try:
        frames = helper.run_program(program, [], aggregate)
    except helper.ShimError as error:
        detail = error.message
        if error.child_stderr:
            stderr = error.child_stderr.decode("utf-8", "replace").strip()
            if stderr:
                detail = f"{detail}: {stderr}"
        raise error_type(f"native LSP の実行に失敗しました: {detail}") from error
    for raw_frame in frames:
        try:
            response = json.loads(_frame_body(raw_frame, error_type).decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise error_type(f"native LSP response が JSON ではありません: {error}") from error
        if not isinstance(response, dict):
            raise error_type("native LSP response が object ではありません")
        if response.get("id") != 2:
            continue
        if response.get("error") is not None:
            error = response["error"]
            message = error.get("message") if isinstance(error, dict) else str(error)
            raise error_type(f"native LSP {label} error: {message}")
        if response.get("result") is None:
            if allow_none:
                return None
            raise error_type(f"{label} を解決できませんでした")
        return response["result"]
    raise error_type(f"native LSP {label} response がありません")


def call_hover(program, arguments, temporary_directory):
    allowed = {"source", "file", "line", "character", "col"}
    unknown = sorted(set(arguments).difference(allowed))
    if unknown:
        raise HoverLookupError(f"lsharp_hover の未知の引数: {', '.join(unknown)}")
    result = _run_lsp_request(
        program,
        arguments,
        temporary_directory,
        "textDocument/hover",
        "hover",
        HoverLookupError,
    )
    return _project_hover(result)


def _project_definition(result):
    if isinstance(result, list):
        if len(result) != 1:
            raise DefinitionLookupError("native LSP definition location が一つではありません")
        result = result[0]
    if not isinstance(result, dict) or not isinstance(result.get("range"), dict):
        raise DefinitionLookupError("native LSP definition range が不正です")
    location_range = result["range"]
    projected = {}
    for name in ("start", "end"):
        position = location_range.get(name)
        if (
            not isinstance(position, dict)
            or type(position.get("line")) is not int
            or position["line"] < 0
            or type(position.get("character")) is not int
            or position["character"] < 0
        ):
            raise DefinitionLookupError(f"native LSP definition range.{name} が不正です")
        projected[name] = {
            "line": position["line"],
            "character": position["character"],
        }
    return projected


def call_definition(program, arguments, temporary_directory):
    allowed = {"source", "file", "line", "character", "col"}
    unknown = sorted(set(arguments).difference(allowed))
    if unknown:
        raise DefinitionLookupError(f"lsharp_definition の未知の引数: {', '.join(unknown)}")
    result = _run_lsp_request(
        program,
        arguments,
        temporary_directory,
        "textDocument/definition",
        "definition",
        DefinitionLookupError,
    )
    return _project_definition(result)


def _project_references(result):
    if result is None:
        result = []
    if not isinstance(result, list):
        raise ReferencesLookupError("native LSP references result が配列ではありません")
    ranges = []
    for index, location in enumerate(result):
        if not isinstance(location, dict) or not isinstance(location.get("range"), dict):
            raise ReferencesLookupError(f"native LSP references[{index}] range が不正です")
        location_range = location["range"]
        projected = {}
        for name in ("start", "end"):
            position = location_range.get(name)
            if (
                not isinstance(position, dict)
                or type(position.get("line")) is not int
                or position["line"] < 0
                or type(position.get("character")) is not int
                or position["character"] < 0
            ):
                raise ReferencesLookupError(
                    f"native LSP references[{index}] range.{name} が不正です"
                )
            projected[name] = {
                "line": position["line"],
                "character": position["character"],
            }
        ranges.append(projected)
    return {"count": len(ranges), "ranges": ranges}


def call_references(program, arguments, temporary_directory):
    allowed = {"source", "file", "line", "character", "col"}
    unknown = sorted(set(arguments).difference(allowed))
    if unknown:
        raise ReferencesLookupError(f"lsharp_references の未知の引数: {', '.join(unknown)}")
    result = _run_lsp_request(
        program,
        arguments,
        temporary_directory,
        "textDocument/references",
        "references",
        ReferencesLookupError,
        request_extra={"context": {"includeDeclaration": True}},
        allow_none=True,
    )
    return _project_references(result)
