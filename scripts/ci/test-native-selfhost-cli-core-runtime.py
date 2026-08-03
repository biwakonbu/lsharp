#!/usr/bin/env python3
"""Check the native App.Cli core command surface without a Rust fallback."""

import argparse
import pathlib
import subprocess
import tempfile


INPUT_SOURCE = "(defn main [] 42)\n"
METADATA_SOURCE = """(defn abs [x]
  :example [(= (abs 5) 5) (= (abs (- 0 7)) 7)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"""


def run(program, root, args, stdin=b""):
    result = subprocess.run(
        [str(program), *args],
        cwd=root,
        input=stdin,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            f"{' '.join(args)} failed: exit={result.returncode} "
            f"stdout={result.stdout!r} stderr={result.stderr!r}"
        )
    if result.stderr:
        raise AssertionError(f"{' '.join(args)} emitted stderr: {result.stderr!r}")
    return result.stdout


def require_exact(label, actual, expected):
    if actual != expected:
        raise AssertionError(f"{label} mismatch: actual={actual!r} expected={expected!r}")


def require_contains(label, actual, expected):
    if expected not in actual:
        raise AssertionError(f"{label} is missing {expected!r}: actual={actual!r}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--program", required=True, type=pathlib.Path)
    args = parser.parse_args()
    program = args.program.expanduser().resolve()
    if not program.is_file() or not program.stat().st_mode & 0o111:
        raise SystemExit(f"native program is not executable: {program}")

    with tempfile.TemporaryDirectory(prefix="lsharp-native-cli-core-") as directory:
        root = pathlib.Path(directory)
        (root / "input.ls").write_text(INPUT_SOURCE, encoding="utf-8")
        (root / "metadata.ls").write_text(METADATA_SOURCE, encoding="utf-8")

        require_contains("help", run(program, root, ["--help"]), b"Usage: lsharp")
        require_exact("version", run(program, root, ["--version"]), b"lsharp 0.1.0")
        require_exact(
            "parse",
            run(program, root, ["parse", "input.ls"]),
            b"decls:1\nfirst-decl:defn\nfirst-body:int\ndiagnostics:0\n",
        )
        require_exact("check", run(program, root, ["check", "input.ls"]), b"Int\ndiagnostics:0\n")
        require_exact("fmt", run(program, root, ["fmt", "input.ls"]), b"(defn main [] 42)\n")
        require_exact(
            "test",
            run(program, root, ["test", "input.ls"]),
            b"examples:0\ninvariants:0\nfailures:0\n",
        )
        require_exact(
            "metadata test",
            run(program, root, ["test", "metadata.ls"]),
            b"examples:2\ninvariants:1\nfailures:0\n",
        )

        for command in ("compile", "build"):
            output_path = root / f"{command}.wasm"
            stdout = run(program, root, [command, "input.ls", "-o", str(output_path)])
            if not output_path.is_file() or output_path.read_bytes()[:4] != b"\x00asm":
                raise AssertionError(f"{command} did not produce a core Wasm artifact")
            require_contains(f"{command} summary", stdout, b"wasm-size:")

    print("native CLI core runtime matrix passed: 9 cases")


if __name__ == "__main__":
    main()
