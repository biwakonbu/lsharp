#!/usr/bin/env python3

"""Produce explicit Rust-oracle observations for selected semantic fixtures.

The compiler and Wasmtime executable are required inputs.  This command never
invokes cargo, host lsharp, an embedded selfhost component, or a provider
implicitly; the caller owns those boundaries and supplies the paths. Repeat
``--fixture-id`` to produce a deterministic batch. Invalid fixtures are
accepted only when their compiler diagnostic exposes an LS#### code and a
source byte span.
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
BYTE_SPAN = re.compile(
    r"(?:"
    r"\((?P<range_start>\d+)\.\.(?P<range_end>\d+)\)"
    r"|"
    r"Span\s*\{\s*start:\s*(?:[│|]\s*)?"
    r"(?P<struct_start>\d+)\s*,\s*end:\s*(?:[│|]\s*)?"
    r"(?P<struct_end>\d+)\s*\}"
    r")",
    re.DOTALL,
)


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


def source_point(source: str, offset: int, label: str) -> Dict[str, int]:
    encoded = source.encode("utf-8")
    if offset < 0 or offset > len(encoded):
        raise ReportError(f"Rust oracle diagnostic span {label} is outside the source")
    try:
        prefix = encoded[:offset].decode("utf-8")
    except UnicodeDecodeError as error:
        raise ReportError(f"Rust oracle diagnostic span {label} is not on a UTF-8 boundary") from error
    line = prefix.count("\n") + 1
    column = len(prefix.rsplit("\n", 1)[-1]) + 1
    return {"line": line, "column": column}


def parse_invalid_diagnostic(stderr: str, source: str) -> Dict[str, Any]:
    code_match = DIAGNOSTIC_CODE.search(stderr)
    if code_match is None:
        raise ReportError("Rust oracle diagnostic code is missing; refusing synthetic invalid report")
    span_match = BYTE_SPAN.search(stderr)
    if span_match is None:
        raise ReportError("Rust oracle diagnostic span is missing; refusing synthetic invalid report")
    start = int(span_match.group("range_start") or span_match.group("struct_start"))
    end = int(span_match.group("range_end") or span_match.group("struct_end"))
    if end < start:
        raise ReportError("Rust oracle diagnostic span has a reversed range")
    return {
        "code": code_match.group(1),
        "span": {
            "start": source_point(source, start, "start"),
            "end": source_point(source, end, "end"),
        },
    }


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


def select_fixtures(manifest: Dict[str, Any], fixture_ids: list[str], target: str) -> list[Dict[str, Any]]:
    if not fixture_ids:
        raise ReportError("at least one --fixture-id is required")
    if len(set(fixture_ids)) != len(fixture_ids):
        raise ReportError("duplicate fixture ids are not allowed")
    by_id = {fixture["id"]: fixture for fixture in manifest["fixtures"]}
    selected = []
    for fixture_id in sorted(fixture_ids):
        fixture = by_id.get(fixture_id)
        if fixture is None:
            raise ReportError(f"unknown fixture id: {fixture_id}")
        if fixture["kind"] not in {"valid", "invalid"}:
            raise ReportError(f"unsupported fixture kind: {fixture['kind']}")
        if target not in fixture["targets"]:
            raise ReportError(f"fixture target is not declared: {target}")
        if fixture["kind"] == "valid" and fixture["expected"]["diagnostics"]:
            raise ReportError("Rust report producer requires a valid fixture with no expected diagnostics")
        selected.append(fixture)
    return selected


def fixture_work_dir(work_dir: pathlib.Path, index: int, batch_size: int) -> pathlib.Path:
    if batch_size == 1:
        return work_dir
    path = work_dir / f"{index:04d}"
    if path.exists() or path.is_symlink():
        raise ReportError(f"fixture work directory already exists: {path}")
    try:
        path.mkdir()
    except OSError as error:
        raise ReportError(f"fixture work directory cannot be created: {path}: {error}") from error
    return path


def observe_fixture(
    fixture: Dict[str, Any],
    index: int,
    batch_size: int,
    root: pathlib.Path,
    work_dir: pathlib.Path,
    runtime_dir: pathlib.Path,
    compiler: pathlib.Path,
    wasmtime: pathlib.Path,
    environment: Dict[str, str],
) -> Dict[str, Any]:
    source = root / pathlib.PurePosixPath(fixture["source"])
    source_text = source.read_text(encoding="utf-8")
    artifact = fixture_work_dir(work_dir, index, batch_size) / "semantic-fixture.wasm"
    if artifact.exists() or artifact.is_symlink():
        raise ReportError(f"artifact path already exists: {artifact}")
    compile_result = subprocess.run(
        [str(compiler), "compile", str(source), "-o", str(artifact), "--target", "wasi-preview1"],
        cwd=root,
        env=environment,
        capture_output=True,
        check=False,
    )
    if fixture["kind"] == "invalid":
        if compile_result.returncode == 0:
            raise ReportError("Rust oracle invalid fixture unexpectedly compiled successfully")
        if artifact.exists() or artifact.is_symlink():
            raise ReportError("Rust oracle invalid fixture produced an unexpected Wasm artifact")
        return {
            "id": fixture["id"],
            "diagnostics": [
                parse_invalid_diagnostic(
                    decode_output(compile_result.stderr) + "\n" + decode_output(compile_result.stdout),
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
    return {
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


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--fixture-id", dest="fixture_ids", action="append", required=True)
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
        fixtures = select_fixtures(manifest, arguments.fixture_ids, arguments.target)

        environment = os.environ.copy()
        environment["LSHARP_DISABLE_EMBEDDED_COMPONENT"] = "1"
        environment.pop("LSHARP_PATH", None)
        report = {
            "schema_version": 1,
            "suite": "v4-m1-01",
            "producer": "rust-oracle",
            "target": arguments.target,
            "source_commit": arguments.source_commit,
            "fixtures": [],
        }
        for index, fixture in enumerate(fixtures):
            report["fixtures"].append(
                observe_fixture(
                    fixture,
                    index,
                    len(fixtures),
                    root,
                    work_dir,
                    runtime_dir,
                    compiler,
                    wasmtime,
                    environment,
                )
            )
        write_report(arguments.output, report)
    except (OSError, json.JSONDecodeError, ManifestError, ReportError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
