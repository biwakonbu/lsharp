#!/usr/bin/env python3
"""native selfhost compilerを外部REPL境界として行指向で実行する。"""

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile


PROMPT = "lsharp> "


def parse_arguments():
    parser = argparse.ArgumentParser(
        description="Evaluate L# expressions through native compile and external wasmtime."
    )
    parser.add_argument(
        "--program",
        required=True,
        type=pathlib.Path,
        help="path to the native program.native compiler",
    )
    parser.add_argument(
        "--wasmtime",
        type=pathlib.Path,
        help="path to an external wasmtime executable",
    )
    parser.add_argument(
        "--stdin",
        action="store_true",
        help="read one expression per stdin line without an interactive prompt",
    )
    return parser.parse_args()


def executable_path(path, label):
    resolved = path.expanduser().resolve()
    if not resolved.is_file() or not os.access(resolved, os.X_OK):
        print(f"error: {label} is not an executable file: {path}", file=sys.stderr)
        return None
    return resolved


def resolve_wasmtime(configured_path):
    if configured_path is not None:
        return executable_path(configured_path, "wasmtime")

    discovered_path = shutil.which("wasmtime")
    if discovered_path is None:
        print(
            "error: wasmtime was not found in PATH; pass --wasmtime PATH",
            file=sys.stderr,
        )
        return None
    return executable_path(pathlib.Path(discovered_path), "wasmtime")


def run_expression(program, wasmtime, expression):
    source_text = f"(defn main [] {expression})"
    try:
        with tempfile.TemporaryDirectory(prefix="lsharp-native-selfhost-repl-") as directory:
            temporary_directory = pathlib.Path(directory)
            source_path = temporary_directory / "expression.ls"
            wasm_path = temporary_directory / "expression.wasm"
            source_path.write_text(source_text, encoding="utf-8")

            try:
                compile_result = subprocess.run(
                    [str(program), "compile", str(source_path), "-o", str(wasm_path)],
                    check=False,
                )
            except OSError as error:
                print(
                    f"error: could not start native compiler {program}: {error}",
                    file=sys.stderr,
                )
                return False
            if compile_result.returncode != 0:
                print(
                    f"error: compile failed with exit code {compile_result.returncode}",
                    file=sys.stderr,
                )
                return False
            if not wasm_path.is_file():
                print(
                    "error: compile succeeded but did not produce a Wasm artifact",
                    file=sys.stderr,
                )
                return False

            try:
                runtime_result = subprocess.run([str(wasmtime), str(wasm_path)], check=False)
            except OSError as error:
                print(
                    f"error: could not start wasmtime {wasmtime}: {error}",
                    file=sys.stderr,
                )
                return False
            if runtime_result.returncode != 0:
                print(
                    f"error: runtime failed with exit code {runtime_result.returncode}",
                    file=sys.stderr,
                )
                return False
    except OSError as error:
        print(f"error: could not create REPL temporary artifacts: {error}", file=sys.stderr)
        return False
    return True


def read_expressions(program, wasmtime, interactive):
    has_failure = False
    while True:
        if interactive:
            sys.stdout.write(PROMPT)
            sys.stdout.flush()
        line = sys.stdin.readline()
        if line == "":
            break

        expression = line.strip()
        if not expression:
            continue
        if expression == ":quit":
            break
        if not run_expression(program, wasmtime, expression):
            has_failure = True
    return 1 if has_failure else 0


def main():
    arguments = parse_arguments()
    program = executable_path(arguments.program, "native program")
    if program is None:
        return 2
    wasmtime = resolve_wasmtime(arguments.wasmtime)
    if wasmtime is None:
        return 2

    if not arguments.stdin and not sys.stdin.isatty():
        print("error: interactive mode requires a TTY; use --stdin for piped input", file=sys.stderr)
        return 2
    return read_expressions(program, wasmtime, interactive=not arguments.stdin)


if __name__ == "__main__":
    raise SystemExit(main())
