#!/usr/bin/env python3
"""Check native selfhost I/O builtins through an external WASI runtime."""

import argparse
import pathlib
import subprocess
import tempfile


READ_STDIN_SOURCE = "(defn main [] (print-string (read-stdin)))\n"
READ_STDIN_4096 = b"a" * 4095 + b"b"
READ_STDIN_OVER_4096 = b"a" * 4096 + b"b"
READ_FILE_OVER_4096 = b"a" * 4096 + b"b"
PRINT_ZERO_SOURCE = "(defn main [] (print 0))\n"
COMMAND_LINE_SOURCE = """(defn main []
  (do
    (print-string (command-line-arg 0))
    (print-string (command-line-arg 1))
    (print-string (command-line-arg 2))
    (print (command-line-args))))
"""


CASES = (
    {
        "name": "print-string",
        "source": '(defn main [] (print-string "hello"))\n',
        "stdout": b"hello",
        "exit_code": 0,
    },
    {
        "name": "read-stdin",
        "source": READ_STDIN_SOURCE,
        "stdin": b"payload",
        "stdout": b"payload",
        "exit_code": 0,
    },
    {
        "name": "read-stdin-empty",
        "source": READ_STDIN_SOURCE,
        "stdin": b"",
        "stdout": b"",
        "exit_code": 0,
    },
    {
        "name": "read-stdin-4096",
        "source": READ_STDIN_SOURCE,
        "stdin": READ_STDIN_4096,
        "stdout": READ_STDIN_4096,
        "exit_code": 0,
    },
    {
        "name": "read-stdin-over-4096",
        "source": READ_STDIN_SOURCE,
        "stdin": READ_STDIN_OVER_4096,
        "stdout": READ_STDIN_OVER_4096,
        "exit_code": 0,
    },
    {
        "name": "read-file",
        "source": '(defn main [] (print-string (read-file "input.txt")))\n',
        "files": (("input.txt", b"payload"),),
        "stdout": b"payload",
        "exit_code": 0,
    },
    {
        "name": "read-file-empty",
        "source": '(defn main [] (print-string (read-file "input.txt")))\n',
        "files": (("input.txt", b""),),
        "stdout": b"",
        "exit_code": 0,
    },
    {
        "name": "read-file-over-4096",
        "source": '(defn main [] (print-string (read-file "input.txt")))\n',
        "files": (("input.txt", READ_FILE_OVER_4096),),
        "stdout": READ_FILE_OVER_4096,
        "exit_code": 0,
    },
    {
        "name": "read-file-missing",
        "source": '(defn main [] (print-string (read-file "missing.txt")))\n',
        "stdout": b"",
        "exit_code": 0,
    },
    {
        "name": "command-line",
        "source": COMMAND_LINE_SOURCE,
        "args": ["alpha", "beta"],
        "stdout": b"alphabeta2\n",
        "exit_code": 0,
    },
    {
        "name": "command-line-empty-argv0",
        "source": COMMAND_LINE_SOURCE,
        "args": [],
        "stdout": b"1\n",
        "exit_code": 0,
    },
    {
        "name": "print-zero",
        "source": PRINT_ZERO_SOURCE,
        "args": [],
        "stdout": b"0\n",
        "exit_code": 0,
    },
    {
        "name": "write-file",
        "source": '(defn main [] (write-file "text.txt" "payload"))\n',
        "stdout": b"",
        "exit_code": 0,
        "file": ("text.txt", b"payload"),
    },
    {
        "name": "write-file-bytes",
        "source": """(defn main []
  (let [bytes (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 4) 0)
                    97)
                  115)
                109)]
    (write-file-bytes \"raw.bin\" bytes)))
""",
        "stdout": b"",
        "exit_code": 0,
        "file": ("raw.bin", b"\x00asm"),
    },
    {
        "name": "proc-exit",
        "source": "(defn main [] (proc-exit 7))\n",
        "stdout": b"",
        "exit_code": 7,
    },
)


def executable(path, label):
    resolved = path.expanduser().resolve()
    if not resolved.is_file() or not resolved.stat().st_mode & 0o111:
        raise SystemExit(f"{label} is not executable: {resolved}")
    return resolved


def run_case(program, wasmtime, case):
    with tempfile.TemporaryDirectory(prefix=f"lsharp-native-io-{case['name']}-") as directory:
        root = pathlib.Path(directory)
        source_path = root / "input.ls"
        wasm_path = root / "program.wasm"
        source_path.write_text(case["source"], encoding="utf-8")
        for file_name, contents in case.get("files", ()):
            file_path = root / file_name
            file_path.parent.mkdir(parents=True, exist_ok=True)
            file_path.write_bytes(contents)

        compile_result = subprocess.run(
            [str(program), "compile", str(source_path), "-o", str(wasm_path)],
            cwd=root,
            capture_output=True,
            check=False,
        )
        if compile_result.returncode != 0:
            raise AssertionError(
                f"{case['name']} compile failed: "
                f"stdout={compile_result.stdout!r} stderr={compile_result.stderr!r}"
            )
        if not wasm_path.is_file() or not wasm_path.stat().st_size:
            raise AssertionError(f"{case['name']} compile produced no Wasm artifact")

        case_args = list(case.get("args", []))
        argv0 = case_args.pop(0) if case_args else ""
        runtime_result = subprocess.run(
            [str(wasmtime), "--dir=.", "--argv0", argv0, str(wasm_path), *case_args],
            cwd=root,
            capture_output=True,
            input=case.get("stdin"),
            check=False,
        )
        if runtime_result.returncode != case["exit_code"]:
            raise AssertionError(
                f"{case['name']} exit mismatch: "
                f"actual={runtime_result.returncode} expected={case['exit_code']} "
                f"stdout={runtime_result.stdout!r} stderr={runtime_result.stderr!r}"
            )
        if runtime_result.stdout != case["stdout"]:
            raise AssertionError(
                f"{case['name']} stdout mismatch: "
                f"actual={runtime_result.stdout!r} expected={case['stdout']!r}"
            )
        if "file" in case:
            file_name, expected_bytes = case["file"]
            actual_bytes = (root / file_name).read_bytes()
            if actual_bytes != expected_bytes:
                raise AssertionError(
                    f"{case['name']} file mismatch: "
                    f"actual={actual_bytes!r} expected={expected_bytes!r}"
                )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--program", required=True, type=pathlib.Path)
    parser.add_argument("--wasmtime", required=True, type=pathlib.Path)
    args = parser.parse_args()
    program = executable(args.program, "native program")
    wasmtime = executable(args.wasmtime, "wasmtime")
    for case in CASES:
        run_case(program, wasmtime, case)
    print(f"native I/O runtime matrix passed: {len(CASES)} cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
