#!/usr/bin/env python3

"""Produce one explicit Rust-oracle observation for a valid fixture.

The compiler and Wasmtime executable are required inputs.  This command never
invokes cargo, host lsharp, an embedded selfhost component, or a provider
implicitly; the caller owns those boundaries and supplies the paths.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile
from typing import Any, Dict

from semantic_fixture_matrix import (
    ManifestError,
    SUPPORTED_TARGETS,
    project_manifest,
    require_object,
)


SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")


class ReportError(ValueError):
    """The explicit Rust report boundary cannot be produced."""


def require_absolute_directory(path: pathlib.Path, label: str) -> pathlib.Path:
    if not path.is_absolute() or path == pathlib.Path("/"):
        raise ReportError(f"{label} must be an absolute non-root path: {path}")
    if path.is_symlink() or not path.is_dir():
        raise ReportError(f"{label} must be a regular directory: {path}")
    return path.resolve()


def require_executable(path: pathlib.Path, label: str) -> pathlib.Path:
    if not path.is_absolute() or path.is_symlink() or not path.is_file() or not os.access(path, os.X_OK):
        raise ReportError(f"{label} must be an executable regular file: {path}")
    return path.resolve()


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def decode_output(value: bytes) -> str:
    return value.decode("utf-8", errors="replace")


def write_report(path: pathlib.Path, report: Dict[str, Any]) -> None:
    if not path.is_absolute() or path == pathlib.Path("/"):
        raise ReportError(f"output must be an absolute non-root path: {path}")
    if path.exists() or path.is_symlink() or not path.parent.is_dir():
        raise ReportError(f"output must be a new file under an existing directory: {path}")
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=path.parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as stream:
            temporary = pathlib.Path(stream.name)
            json.dump(report, stream, ensure_ascii=False, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        temporary = None
    except OSError as error:
        raise ReportError(f"report output cannot be written: {path}: {error}") from error
    finally:
        if temporary is not None:
            try:
                temporary.unlink()
            except OSError:
                pass


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--fixture-id", required=True)
    parser.add_argument("--target", choices=SUPPORTED_TARGETS, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--compiler", type=pathlib.Path, required=True)
    parser.add_argument("--wasmtime", type=pathlib.Path, required=True)
    parser.add_argument("--work-dir", type=pathlib.Path, required=True)
    parser.add_argument("--runtime-dir", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        if not SOURCE_COMMIT.fullmatch(arguments.source_commit):
            raise ReportError("source_commit must be a 40-character lowercase commit")
        root = require_absolute_directory(arguments.root, "root")
        work_dir = require_absolute_directory(arguments.work_dir, "work-dir")
        runtime_dir = require_absolute_directory(arguments.runtime_dir, "runtime-dir") if arguments.runtime_dir else work_dir
        compiler = require_executable(arguments.compiler, "compiler")
        wasmtime = require_executable(arguments.wasmtime, "wasmtime")
        raw_manifest = json.loads(arguments.manifest.read_text(encoding="utf-8"))
        manifest = project_manifest(require_object(raw_manifest, "manifest"), root)
        fixture = next(
            (candidate for candidate in manifest["fixtures"] if candidate["id"] == arguments.fixture_id),
            None,
        )
        if fixture is None:
            raise ReportError(f"unknown fixture id: {arguments.fixture_id}")
        if fixture["kind"] != "valid":
            raise ReportError("Rust report producer currently accepts valid fixtures only")
        if arguments.target not in fixture["targets"]:
            raise ReportError(f"fixture target is not declared: {arguments.target}")
        if fixture["expected"]["diagnostics"]:
            raise ReportError("Rust report producer requires a valid fixture with no expected diagnostics")
        source = root / pathlib.PurePosixPath(fixture["source"])
        artifact = work_dir / "semantic-fixture.wasm"
        if artifact.exists() or artifact.is_symlink():
            raise ReportError(f"artifact path already exists: {artifact}")

        environment = os.environ.copy()
        environment["LSHARP_DISABLE_EMBEDDED_COMPONENT"] = "1"
        environment.pop("LSHARP_PATH", None)
        compile_result = subprocess.run(
            [str(compiler), "compile", str(source), "-o", str(artifact), "--target", "wasi-preview1"],
            cwd=root,
            env=environment,
            capture_output=True,
            check=False,
        )
        if compile_result.returncode != 0:
            detail = decode_output(compile_result.stderr).strip() or decode_output(compile_result.stdout).strip()
            raise ReportError(f"Rust oracle compile failed with exit {compile_result.returncode}: {detail}")
        if not artifact.is_file() or artifact.is_symlink():
            raise ReportError("Rust oracle compile succeeded without a regular Wasm artifact")

        runtime_result = subprocess.run(
            [str(wasmtime), "run", str(artifact)],
            cwd=runtime_dir,
            env=environment,
            capture_output=True,
            check=False,
        )
        report = {
            "schema_version": 1,
            "suite": "v4-m1-01",
            "producer": "rust-oracle",
            "target": arguments.target,
            "source_commit": arguments.source_commit,
            "fixtures": [
                {
                    "id": fixture["id"],
                    "diagnostics": [],
                    "exit_code": compile_result.returncode,
                    "artifact": {
                        "status": "observed",
                        "sha256": sha256(artifact),
                        "size": artifact.stat().st_size,
                    },
                    "runtime": {
                        "status": "observed",
                        "exit_code": runtime_result.returncode,
                        "stdout": decode_output(runtime_result.stdout),
                        "stderr": decode_output(runtime_result.stderr),
                    },
                }
            ],
        }
        write_report(arguments.output, report)
    except (OSError, json.JSONDecodeError, ManifestError, ReportError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
