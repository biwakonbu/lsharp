"""Native compiler と外部 wasmtime の MCP compile/run 境界。"""

import os
import pathlib
import shutil
import subprocess


class CompileRunError(Exception):
    """Compile/run boundary failed or received invalid arguments."""


COMPILE_RUN_OUTPUT_SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "additionalProperties": False,
    "required": ["ok", "formatted", "stdout", "exit_code"],
    "properties": {
        "ok": {"type": "boolean", "const": True},
        "formatted": {"type": "string"},
        "stdout": {"type": "string"},
        "exit_code": {"type": "integer", "const": 0},
    },
}


def _input(arguments, temporary_directory):
    unknown = sorted(set(arguments).difference(("source", "file")))
    if unknown:
        raise CompileRunError(f"lsharp_compile_run の未知の引数: {', '.join(unknown)}")
    present = [name for name in ("source", "file") if name in arguments]
    if len(present) != 1:
        raise CompileRunError("source または file のいずれか一つが必要です")
    name = present[0]
    value = arguments[name]
    if not isinstance(value, str) or not value.strip():
        raise CompileRunError(f"{name} は空でない文字列が必要です")
    if name == "source":
        content = value
    else:
        path = pathlib.Path(value)
        if not path.is_file():
            raise CompileRunError(f"native MCP input file が見つかりません: {path}")
        try:
            content = path.read_text(encoding="utf-8")
        except OSError as error:
            raise CompileRunError(
                f"native MCP input file の読み込みに失敗しました: {error}"
            ) from error
    input_path = pathlib.Path(temporary_directory) / "Main.ls"
    try:
        input_path.write_text(content, encoding="utf-8")
    except OSError as error:
        raise CompileRunError(
            f"native MCP source の一時 file 作成に失敗しました: {error}"
        ) from error
    return input_path, content


def _wasmtime_path():
    configured = os.environ.get("LSHARP_WASMTIME")
    if configured:
        candidate = pathlib.Path(configured).expanduser()
    else:
        discovered = shutil.which("wasmtime")
        if discovered is None:
            raise CompileRunError(
                "wasmtime が見つかりません。LSHARP_WASMTIME または PATH を指定してください"
            )
        candidate = pathlib.Path(discovered)
    try:
        resolved = candidate.resolve()
    except OSError as error:
        raise CompileRunError(f"wasmtime path の解決に失敗しました: {candidate}: {error}") from error
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        raise CompileRunError(f"wasmtime が実行可能な file ではありません: {candidate}")
    return resolved


def _run(command, label):
    try:
        return subprocess.run(
            command,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            check=False,
        )
    except OSError as error:
        raise CompileRunError(f"{label} の起動に失敗しました: {error}") from error


def call_compile_run(program, arguments, temporary_directory):
    input_path, _ = _input(arguments, temporary_directory)
    wasmtime = _wasmtime_path()
    output_path = pathlib.Path(temporary_directory) / "Main.wasm"

    compile_result = _run(
        [program, "compile", str(input_path), "-o", str(output_path)],
        "native compile",
    )
    if compile_result.returncode != 0:
        detail = compile_result.stderr.strip() or compile_result.stdout.strip()
        message = f"native compile failed with exit code {compile_result.returncode}"
        if detail:
            message += f": {detail}"
        raise CompileRunError(message)
    if not output_path.is_file() or output_path.stat().st_size == 0:
        raise CompileRunError("native compile succeeded but did not produce a non-empty Wasm artifact")

    runtime_result = _run([str(wasmtime), str(output_path)], "wasmtime")
    if runtime_result.returncode != 0:
        detail = runtime_result.stderr.strip() or runtime_result.stdout.strip()
        message = f"wasmtime runtime failed with exit code {runtime_result.returncode}"
        if detail:
            message += f": {detail}"
        raise CompileRunError(message)

    try:
        formatted = input_path.read_text(encoding="utf-8")
    except OSError as error:
        raise CompileRunError(
            f"native MCP formatted source の読み込みに失敗しました: {error}"
        ) from error

    return {
        "ok": True,
        "formatted": formatted,
        "stdout": runtime_result.stdout,
        "exit_code": 0,
    }
