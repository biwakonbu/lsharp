#!/usr/bin/env python3

"""Produce one explicit native-stage0 observation for a semantic fixture.

The native runner, Wasmtime executable, stage0 manifest, source commit, target,
and work directories are required inputs.  This command never discovers a
Rust host, fallback compiler, embedded component, provider, or network
boundary implicitly.  Invalid fixtures are accepted only when the native
diagnostic exposes an LS#### code and a source byte span.
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
DIAGNOSTIC_CODE = re.compile(r"\[(LS[0-9]{4})\]")
BYTE_SPAN = re.compile(r"\((\d+)\.\.(\d+)\)")


class ReportError(ValueError):
    """The explicit native-stage0 report boundary cannot be produced."""


def require_absolute_directory(path: pathlib.Path, label: str) -> pathlib.Path:
    if not path.is_absolute() or path == pathlib.Path("/"):
        raise ReportError(f"{label} must be an absolute non-root path: {path}")
    if path.is_symlink() or not path.is_dir():
        raise ReportError(f"{label} must be a regular directory: {path}")
    return path.resolve()


def require_absolute_file(path: pathlib.Path, label: str) -> pathlib.Path:
    if not path.is_absolute() or path == pathlib.Path("/"):
        raise ReportError(f"{label} must be an absolute non-root path: {path}")
    if path.is_symlink() or not path.is_file():
        raise ReportError(f"{label} must be a regular file: {path}")
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


def source_point(source: str, offset: int, label: str) -> Dict[str, int]:
    encoded = source.encode("utf-8")
    if offset < 0 or offset > len(encoded):
        raise ReportError(f"native diagnostic span {label} is outside the source")
    try:
        prefix = encoded[:offset].decode("utf-8")
    except UnicodeDecodeError as error:
        raise ReportError(f"native diagnostic span {label} is not on a UTF-8 boundary") from error
    return {
        "line": prefix.count("\n") + 1,
        "column": len(prefix.rsplit("\n", 1)[-1]) + 1,
    }


def parse_invalid_diagnostic(output: str, source: str) -> Dict[str, Any]:
    code_match = DIAGNOSTIC_CODE.search(output)
    if code_match is None:
        raise ReportError("native diagnostic code is missing; refusing synthetic invalid report")
    span_match = BYTE_SPAN.search(output)
    if span_match is None:
        raise ReportError("native diagnostic span is missing; refusing synthetic invalid report")
    start = int(span_match.group(1))
    end = int(span_match.group(2))
    if end < start:
        raise ReportError("native diagnostic span has a reversed range")
    return {
        "code": code_match.group(1),
        "span": {
            "start": source_point(source, start, "start"),
            "end": source_point(source, end, "end"),
        },
    }


def load_stage0_manifest(
    path: pathlib.Path, target: str, source_commit: str
) -> Dict[str, Any]:
    try:
        manifest = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ReportError(f"stage0 manifest is invalid: {path}: {error}") from error
    if not isinstance(manifest, dict):
        raise ReportError("stage0 manifest must be a JSON object")
    if manifest.get("kind") != "lsharp-native-selfhost-stage0":
        raise ReportError("stage0 manifest kind is invalid")
    if manifest.get("target") != target:
        raise ReportError(
            f"stage0 manifest target mismatch: manifest={manifest.get('target')!r} target={target!r}"
        )
    if manifest.get("source_commit") != source_commit:
        raise ReportError(
            "stage0 manifest source_commit mismatch: "
            f"manifest={manifest.get('source_commit')!r} requested={source_commit!r}"
        )
    for field in ("compiler", "transport_driver", "materializer"):
        value = manifest.get(field)
        relative = pathlib.PurePosixPath(value) if isinstance(value, str) else None
        if (
            relative is None
            or not value
            or relative.is_absolute()
            or ".." in relative.parts
            or "\\" in value
        ):
            raise ReportError(f"stage0 manifest {field} must be a safe relative path")
    return manifest


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
    parser.add_argument("--runner", type=pathlib.Path, required=True)
    parser.add_argument("--wasmtime", type=pathlib.Path, required=True)
    parser.add_argument("--stage0-manifest", type=pathlib.Path, required=True)
    parser.add_argument("--work-dir", type=pathlib.Path, required=True)
    parser.add_argument("--runtime-dir", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    return parser.parse_args()


def report_header(arguments: argparse.Namespace, producer: str) -> Dict[str, Any]:
    return {
        "schema_version": 1,
        "suite": "v4-m1-01",
        "producer": producer,
        "target": arguments.target,
        "source_commit": arguments.source_commit,
    }


def main() -> int:
    arguments = parse_arguments()
    try:
        if not SOURCE_COMMIT.fullmatch(arguments.source_commit):
            raise ReportError("source_commit must be a 40-character lowercase commit")
        root = require_absolute_directory(arguments.root, "root")
        work_dir = require_absolute_directory(arguments.work_dir, "work-dir")
        runtime_dir = (
            require_absolute_directory(arguments.runtime_dir, "runtime-dir")
            if arguments.runtime_dir
            else work_dir
        )
        runner = require_executable(arguments.runner, "runner")
        wasmtime = require_executable(arguments.wasmtime, "wasmtime")
        stage0_manifest = require_absolute_file(arguments.stage0_manifest, "stage0-manifest")
        load_stage0_manifest(stage0_manifest, arguments.target, arguments.source_commit)
        raw_manifest = json.loads(arguments.manifest.read_text(encoding="utf-8"))
        manifest = project_manifest(require_object(raw_manifest, "manifest"), root)
        fixture = next(
            (candidate for candidate in manifest["fixtures"] if candidate["id"] == arguments.fixture_id),
            None,
        )
        if fixture is None:
            raise ReportError(f"unknown fixture id: {arguments.fixture_id}")
        if fixture["kind"] not in {"valid", "invalid"}:
            raise ReportError(f"unsupported fixture kind: {fixture['kind']}")
        if arguments.target not in fixture["targets"]:
            raise ReportError(f"fixture target is not declared: {arguments.target}")
        if fixture["kind"] == "valid" and fixture["expected"]["diagnostics"]:
            raise ReportError("native report producer requires a valid fixture with no expected diagnostics")
        source = root / pathlib.PurePosixPath(fixture["source"])
        source_text = source.read_text(encoding="utf-8")
        artifact = work_dir / "semantic-fixture.wasm"
        if artifact.exists() or artifact.is_symlink():
            raise ReportError(f"artifact path already exists: {artifact}")

        environment = os.environ.copy()
        environment.pop("LSHARP_PATH", None)
        environment.pop("LSHARP_DISABLE_EMBEDDED_COMPONENT", None)
        compile_result = subprocess.run(
            [str(runner), "compile", str(source), "-o", str(artifact), "--target", "wasi-preview1"],
            cwd=root,
            env=environment,
            capture_output=True,
            check=False,
        )
        if fixture["kind"] == "invalid":
            if compile_result.returncode == 0:
                raise ReportError("native invalid fixture unexpectedly compiled successfully")
            if artifact.exists() or artifact.is_symlink():
                raise ReportError("native invalid fixture produced an unexpected Wasm artifact")
            report = report_header(arguments, "native-stage0")
            report["fixtures"] = [
                {
                    "id": fixture["id"],
                    "diagnostics": [
                        parse_invalid_diagnostic(
                            decode_output(compile_result.stderr)
                            + "\n"
                            + decode_output(compile_result.stdout),
                            source_text,
                        )
                    ],
                    "exit_code": compile_result.returncode,
                    "artifact": {"status": "not-applicable"},
                    "runtime": {
                        "status": "not-run",
                        "exit_code": None,
                        "stdout": None,
                        "stderr": None,
                    },
                }
            ]
            write_report(arguments.output, report)
            return 0
        if compile_result.returncode != 0:
            detail = decode_output(compile_result.stderr).strip() or decode_output(compile_result.stdout).strip()
            raise ReportError(f"native compile failed with exit {compile_result.returncode}: {detail}")
        if not artifact.is_file() or artifact.is_symlink():
            raise ReportError("native compile succeeded without a regular Wasm artifact")

        runtime_result = subprocess.run(
            [str(wasmtime), "run", str(artifact)],
            cwd=runtime_dir,
            env=environment,
            capture_output=True,
            check=False,
        )
        report = report_header(arguments, "native-stage0")
        report["fixtures"] = [
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
        ]
        write_report(arguments.output, report)
    except (OSError, UnicodeError, json.JSONDecodeError, ManifestError, ReportError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
