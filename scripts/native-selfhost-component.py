#!/usr/bin/env python3
"""native selfhost のPreview1出力をWASI componentとして包装する。"""

import argparse
import hashlib
import json
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
    parser.add_argument(
        "--runtime-evidence",
        metavar="PATH",
        help="write explicit component runtime evidence; requires --wasmtime",
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


def validate_runtime_evidence(value):
    evidence = pathlib.Path(value).expanduser()
    if not evidence.is_absolute():
        evidence = pathlib.Path.cwd() / evidence
    if evidence.exists() or evidence.is_symlink():
        raise ComponentPackagingError(
            f"runtime evidence path already exists: {evidence}"
        )
    try:
        evidence.parent.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to create runtime evidence directory {evidence.parent}: {error}"
        ) from error
    if not evidence.parent.is_dir():
        raise ComponentPackagingError(
            f"runtime evidence parent is not a directory: {evidence.parent}"
        )
    return evidence


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


def sha256(path):
    digest = hashlib.sha256()
    try:
        with path.open("rb") as artifact:
            for chunk in iter(lambda: artifact.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to read component runtime artifact {path}: {error}"
        ) from error
    return digest.hexdigest()


def decode_runtime_output(value):
    return value.decode("utf-8", errors="replace")


def write_runtime_evidence(path, value):
    temporary = None
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
        )
        os.close(descriptor)
        temporary = pathlib.Path(temporary_name)
        with temporary.open("w", encoding="utf-8") as stream:
            json.dump(value, stream, ensure_ascii=False, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        temporary = None
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to write runtime evidence {path}: {error}"
        ) from error
    finally:
        if temporary is not None:
            try:
                temporary.unlink()
            except OSError:
                pass


def create_temporary_path(path, label):
    try:
        descriptor, temporary_name = tempfile.mkstemp(
            prefix=f".{path.name}.", suffix=".tmp", dir=path.parent
        )
        os.close(descriptor)
        temporary_path = pathlib.Path(temporary_name)
        temporary_path.unlink()
        return temporary_path
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to create temporary {label} in {path.parent}: {error}"
        ) from error


def test_failpoint(name):
    if os.environ.get("LSHARP_TEST_COMPONENT_FAILPOINT") == name:
        raise ComponentPackagingError(f"test failpoint: {name}")


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


def run_runtime(wasmtime, temporary_component):
    try:
        completed = subprocess.run(
            [str(wasmtime), "run", str(temporary_component)],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to execute wasmtime component runtime: {error}"
        ) from error
    if completed.returncode != 0:
        raise ComponentPackagingError(
            f"wasmtime component runtime exited with status {completed.returncode}",
            completed.stderr,
        )
    forward_stderr(completed.stderr)
    return {
        "status": "observed",
        "exit_code": completed.returncode,
        "stdout": decode_runtime_output(completed.stdout),
        "stderr": decode_runtime_output(completed.stderr),
    }


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


def package_component(
    program, wasm_tools, wasmtime, runtime_evidence, command, source, output
):
    temporary_component = None
    temporary_evidence = None
    previous_output = None
    output_promoted = False
    evidence_promoted = False
    committed = False
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
            runtime_artifact_sha256 = sha256(temporary_component)
            runtime_observation = None
            if wasmtime is not None:
                runtime_observation = run_runtime(wasmtime, temporary_component)
                if sha256(temporary_component) != runtime_artifact_sha256:
                    raise ComponentPackagingError(
                        "wasmtime runtime changed component bytes; refusing promotion"
                    )
                if runtime_evidence is not None:
                    temporary_evidence = create_temporary_path(
                        runtime_evidence, "runtime evidence"
                    )
                    write_runtime_evidence(
                        temporary_evidence,
                        {
                            "schema_version": 1,
                            "kind": "lsharp-native-component-runtime-evidence",
                            "command": command,
                            "source": str(source),
                            "source_sha256": "sha256:" + sha256(source),
                            "component_sha256": "sha256:" + runtime_artifact_sha256,
                            "runtime": runtime_observation,
                        },
                    )

            if (output.exists() or output.is_symlink()) and not (
                output.is_dir() and not output.is_symlink()
            ):
                previous_output = create_temporary_path(output, "output backup")
                os.replace(output, previous_output)
            test_failpoint("output-promote")
            try:
                os.replace(temporary_component, output)
            except OSError as error:
                raise ComponentPackagingError(
                    f"failed to atomically replace output {output}: {error}"
                ) from error
            temporary_component = None
            output_promoted = True
            if temporary_evidence is not None:
                test_failpoint("evidence-promote")
                try:
                    os.replace(temporary_evidence, runtime_evidence)
                except OSError as error:
                    raise ComponentPackagingError(
                        f"failed to atomically replace runtime evidence {runtime_evidence}: {error}"
                    ) from error
                temporary_evidence = None
                evidence_promoted = True
            if previous_output is not None:
                cleanup_temporary_component(previous_output)
                previous_output = None
            committed = True
    except ComponentPackagingError as error:
        primary_error = error
        raise
    except OSError as error:
        raise ComponentPackagingError(
            f"failed to manage temporary core output: {error}"
        ) from error
    finally:
        if not committed:
            if evidence_promoted:
                try:
                    runtime_evidence.unlink()
                except FileNotFoundError:
                    pass
                except OSError:
                    pass
            if output_promoted:
                try:
                    output.unlink()
                except FileNotFoundError:
                    pass
                except OSError:
                    pass
            if previous_output is not None:
                try:
                    os.replace(previous_output, output)
                    previous_output = None
                except OSError:
                    pass
            if temporary_evidence is not None:
                try:
                    temporary_evidence.unlink()
                except FileNotFoundError:
                    pass
                except OSError:
                    pass
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
        if args.runtime_evidence and wasmtime is None:
            raise ComponentPackagingError(
                "runtime evidence requires an explicit --wasmtime runtime"
            )
        runtime_evidence = (
            validate_runtime_evidence(args.runtime_evidence)
            if args.runtime_evidence
            else None
        )
        package_component(
            program,
            wasm_tools,
            wasmtime,
            runtime_evidence,
            args.command,
            source,
            output,
        )
    except ComponentPackagingError as error:
        write_error(error)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
