#!/usr/bin/env python3
"""native selfhost のPreview1出力をWASI componentとして包装する。"""

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile


class ComponentPackagingError(Exception):
    def __init__(self, message, child_stderr=b""):
        super().__init__(message)
        self.message = message
        self.child_stderr = child_stderr


def parse_arguments(argv):
    parser = argparse.ArgumentParser(
        description="Package native selfhost Preview1 output as a WASI component."
    )
    parser.add_argument("--program", required=True, metavar="PATH")
    parser.add_argument("--wasm-tools", metavar="PATH")
    parser.add_argument(
        "--wasmtime",
        metavar="PATH",
        help="optionally run the temporary component with an external wasmtime",
    )
    parser.add_argument("--command", required=True, choices=("compile", "build"))
    parser.add_argument("--source", required=True, metavar="FILE")
    parser.add_argument("--output", required=True, metavar="FILE")
    return parser.parse_args(argv)


def validate_executable(label, value):
    path = pathlib.Path(value).expanduser()
    if not path.is_file():
        raise ComponentPackagingError(f"{label} is not a regular file: {path}")
    if not os.access(path, os.X_OK):
        raise ComponentPackagingError(f"{label} is not executable: {path}")
    return path.resolve()


def validate_source(value):
    source = pathlib.Path(value).expanduser()
    if not source.is_file():
        raise ComponentPackagingError(f"source file is not a regular file: {source}")
    return source.resolve()


def validate_output(value):
    output = pathlib.Path(value).expanduser()
    if not output.is_absolute():
        output = pathlib.Path.cwd() / output
    if output.exists():
        if output.is_dir():
            raise ComponentPackagingError(f"output path is a directory: {output}")
        if not output.is_file():
            raise ComponentPackagingError(
                f"output path is not a regular file: {output}"
            )

    try:
        output.parent.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to create output directory {output.parent}: {error}"
        ) from error
    if not output.parent.is_dir():
        raise ComponentPackagingError(
            f"output parent is not a directory: {output.parent}"
        )
    return output


def validate_wasm_artifact(label, path):
    if path.is_symlink() or not path.is_file():
        raise ComponentPackagingError(f"{label} produced invalid Wasm artifact: {path}")
    try:
        with path.open("rb") as artifact:
            magic = artifact.read(4)
    except OSError as error:
        raise ComponentPackagingError(
            f"{label} produced invalid Wasm artifact: {path}: {error}"
        ) from error
    if magic != b"\x00asm":
        raise ComponentPackagingError(f"{label} produced invalid Wasm artifact: {path}")


def find_wasm_tools(value):
    if value is not None:
        return validate_executable("wasm-tools", value)

    candidate = shutil.which("wasm-tools")
    if candidate is None:
        raise ComponentPackagingError(
            "wasm-tools was not found on PATH; pass --wasm-tools PATH"
        )
    return validate_executable("wasm-tools", candidate)


def find_wasmtime(value):
    if value is not None:
        return validate_executable("wasmtime", value)

    candidate = shutil.which("wasmtime")
    if candidate is None:
        raise ComponentPackagingError(
            "wasmtime was not found on PATH; pass --wasmtime PATH"
        )
    return validate_executable("wasmtime", candidate)


def forward_stderr(stderr):
    if not stderr:
        return
    sys.stderr.buffer.write(stderr)
    sys.stderr.buffer.flush()


def run_command(arguments, label):
    try:
        completed = subprocess.run(
            arguments,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise ComponentPackagingError(f"failed to execute {label}: {error}") from error

    if completed.returncode != 0:
        raise ComponentPackagingError(
            f"{label} exited with status {completed.returncode}", completed.stderr
        )
    forward_stderr(completed.stderr)


def create_temporary_component_path(output):
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{output.name}.", suffix=".tmp", dir=output.parent
        )
        os.close(descriptor)
        temporary_path = pathlib.Path(temporary_name)
        temporary_path.unlink()
        return temporary_path
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to create temporary component output in {output.parent}: {error}"
        ) from error


def cleanup_temporary_component(path):
    try:
        if path.is_dir() and not path.is_symlink():
            shutil.rmtree(path)
        else:
            path.unlink()
    except FileNotFoundError:
        return
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to remove temporary component output {path}: {error}"
        ) from error


def package_component(program, wasm_tools, wasmtime, command, source, output):
    temporary_component = None
    primary_error = None
    try:
        with tempfile.TemporaryDirectory(prefix="native-selfhost-component-") as directory:
            core_output = pathlib.Path(directory) / "core.wasm"
            run_command(
                [str(program), command, str(source), "-o", str(core_output)],
                "native program",
            )
            if not core_output.is_file():
                raise ComponentPackagingError(
                    f"native program did not create core Wasm output: {core_output}"
                )
            validate_wasm_artifact("native program", core_output)

            temporary_component = create_temporary_component_path(output)
            run_command(
                [
                    str(wasm_tools),
                    "component",
                    "new",
                    str(core_output),
                    "-o",
                    str(temporary_component),
                ],
                "wasm-tools",
            )
            if not temporary_component.is_file():
                raise ComponentPackagingError(
                    "wasm-tools did not create component output: "
                    f"{temporary_component}"
                )
            validate_wasm_artifact("wasm-tools", temporary_component)
            run_command(
                [str(wasm_tools), "validate", str(temporary_component)],
                "wasm-tools semantic validation",
            )
            if wasmtime is not None:
                run_command(
                    [str(wasmtime), "run", str(temporary_component)],
                    "wasmtime component runtime",
                )

            try:
                os.replace(temporary_component, output)
            except OSError as error:
                raise ComponentPackagingError(
                    f"failed to atomically replace output {output}: {error}"
                ) from error
            temporary_component = None
    except ComponentPackagingError as error:
        primary_error = error
        raise
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to manage temporary core output: {error}"
        ) from error
    finally:
        if temporary_component is not None:
            try:
                cleanup_temporary_component(temporary_component)
            except ComponentPackagingError:
                if primary_error is None:
                    raise


def write_error(error):
    stderr = sys.stderr.buffer
    stderr.write(b"native-selfhost-component: ")
    stderr.write(error.message.encode("utf-8", "replace"))
    stderr.write(b"\n")
    if error.child_stderr:
        stderr.write(error.child_stderr)
        if not error.child_stderr.endswith(b"\n"):
            stderr.write(b"\n")
    stderr.flush()


def main(argv=None):
    args = parse_arguments(argv)
    try:
        program = validate_executable("program", args.program)
        source = validate_source(args.source)
        output = validate_output(args.output)
        wasm_tools = find_wasm_tools(args.wasm_tools)
        wasmtime = find_wasmtime(args.wasmtime) if args.wasmtime else None
        package_component(program, wasm_tools, wasmtime, args.command, source, output)
    except ComponentPackagingError as error:
        write_error(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
