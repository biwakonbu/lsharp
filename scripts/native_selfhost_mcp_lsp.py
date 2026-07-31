"""Native LSP-backed projections for the MCP shim."""

import importlib.util
import json
import pathlib


class HoverLookupError(Exception):
    """A native LSP hover request failed or returned an invalid result."""


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

_NATIVE_LSP_HELPER = None


def _native_lsp_helper():
    global _NATIVE_LSP_HELPER
    if _NATIVE_LSP_HELPER is None:
        helper_path = pathlib.Path(__file__).resolve().with_name("native-selfhost-lsp-stdio.py")
        spec = importlib.util.spec_from_file_location("native_selfhost_lsp_stdio", helper_path)
        if spec is None or spec.loader is None:
            raise HoverLookupError(f"native LSP helper を読み込めません: {helper_path}")
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        _NATIVE_LSP_HELPER = module
    return _NATIVE_LSP_HELPER


def _require_source(arguments, temporary_directory):
    present = [name for name in ("source", "file") if name in arguments]
    if len(present) != 1:
        raise HoverLookupError("source または file のいずれか一つが必要です")
    name = present[0]
    value = arguments[name]
    if not isinstance(value, str) or not value.strip():
        raise HoverLookupError(f"{name} は空でない文字列が必要です")
    if name == "source":
        source = value
        uri = (pathlib.Path(temporary_directory) / "hover.ls").resolve().as_uri()
        return source, uri
    path = pathlib.Path(value)
    if not path.is_file():
        raise HoverLookupError(f"native MCP input file が見つかりません: {path}")
    try:
        source = path.read_text(encoding="utf-8")
    except OSError as error:
        raise HoverLookupError(f"native MCP input file の読み込みに失敗しました: {error}") from error
    return source, path.resolve().as_uri()


def _position(arguments):
    line = arguments.get("line")
    if type(line) is not int or line < 0:
        raise HoverLookupError("line は 0 以上の整数が必要です")
    if "character" in arguments:
        character = arguments["character"]
    elif "col" in arguments:
        character = arguments["col"]
    else:
        raise HoverLookupError("character が必要です")
    if type(character) is not int or character < 0:
        raise HoverLookupError("character は 0 以上の整数が必要です")
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


def _frame_body(raw_frame):
    header_end = raw_frame.find(b"\r\n\r\n")
    if header_end < 0:
        raise HoverLookupError("native LSP frame header が不正です")
    length = None
    for raw_line in raw_frame[:header_end].split(b"\r\n"):
        if b":" not in raw_line:
            raise HoverLookupError("native LSP frame header が不正です")
        name, value = raw_line.split(b":", 1)
        if name.lower() == b"content-length":
            if length is not None:
                raise HoverLookupError("native LSP frame の Content-Length が重複しています")
            try:
                length = int(value.strip())
            except ValueError as error:
                raise HoverLookupError("native LSP frame の Content-Length が不正です") from error
    if length is None:
        raise HoverLookupError("native LSP frame の Content-Length がありません")
    body_start = header_end + 4
    body = raw_frame[body_start:]
    if len(body) != length:
        raise HoverLookupError("native LSP frame body の長さが不一致です")
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


def call_hover(program, arguments, temporary_directory):
    allowed = {"source", "file", "line", "character", "col"}
    unknown = sorted(set(arguments).difference(allowed))
    if unknown:
        raise HoverLookupError(f"lsharp_hover の未知の引数: {', '.join(unknown)}")
    source, uri = _require_source(arguments, temporary_directory)
    position = _position(arguments)
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
                "textDocument/hover",
                {"textDocument": {"uri": uri}, "position": position},
            ),
        ]
    )
    helper = _native_lsp_helper()
    try:
        frames = helper.run_program(program, [], aggregate)
    except helper.ShimError as error:
        detail = error.message
        if error.child_stderr:
            stderr = error.child_stderr.decode("utf-8", "replace").strip()
            if stderr:
                detail = f"{detail}: {stderr}"
        raise HoverLookupError(f"native LSP の実行に失敗しました: {detail}") from error
    for raw_frame in frames:
        try:
            response = json.loads(_frame_body(raw_frame).decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise HoverLookupError(f"native LSP response が JSON ではありません: {error}") from error
        if response.get("id") != 2:
            continue
        if response.get("error") is not None:
            error = response["error"]
            message = error.get("message") if isinstance(error, dict) else str(error)
            raise HoverLookupError(f"native LSP hover error: {message}")
        if response.get("result") is None:
            raise HoverLookupError("hover を解決できませんでした")
        return _project_hover(response["result"])
    raise HoverLookupError("native LSP hover response がありません")
