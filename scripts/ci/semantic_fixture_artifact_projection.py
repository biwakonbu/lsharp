#!/usr/bin/env python3

"""Project source-declared ABI observables from an explicit Wasm artifact.

This is a sidecar evidence boundary, not an extension of the semantic fixture
report schema. It binds one current source fixture to one regular Wasm artifact
and an explicit ``wasm-tools print`` invocation, then emits deterministic
imports, tables (the artifact ftable shape), and exports for Rust/native
comparison. It never instantiates the artifact or discovers a compiler.
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
from typing import Any, Dict, List

from semantic_fixture_matrix import ManifestError, SUPPORTED_TARGETS, project_manifest, require_object


SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")
IMPORT = re.compile(r'\(import\s+"([^"\\]*(?:\\.[^"\\]*)*)"\s+"([^"\\]*(?:\\.[^"\\]*)*)"\s+\((\w+)')
TABLE = re.compile(r"\(table(?:\s+\(;\d+;\))?\s+(\d+)(?:\s+(\d+))?\s+(\w+)")
EXPORT = re.compile(r'\(export\s+"([^"\\]*(?:\\.[^"\\]*)*)"\s+\((\w+)')


class ProjectionError(ValueError):
    """The source-to-artifact projection cannot be trusted."""


def require_absolute_directory(path: pathlib.Path, label: str) -> pathlib.Path:
    if not path.is_absolute() or path == pathlib.Path("/") or path.is_symlink() or not path.is_dir():
        raise ProjectionError(f"{label} must be an absolute regular directory: {path}")
    return path.resolve()


def require_executable(path: pathlib.Path, label: str) -> pathlib.Path:
    if not path.is_absolute() or path.is_symlink() or not path.is_file() or not os.access(path, os.X_OK):
        raise ProjectionError(f"{label} must be an executable regular file: {path}")
    return path.resolve()


def require_artifact(path: pathlib.Path) -> pathlib.Path:
    if not path.is_absolute() or path == pathlib.Path("/") or path.is_symlink() or not path.is_file():
        raise ProjectionError(f"artifact must be an absolute regular file: {path}")
    try:
        data = path.read_bytes()
    except OSError as error:
        raise ProjectionError(f"artifact cannot be read: {path}: {error}") from error
    if len(data) <= 8 or data[:4] != b"\x00asm":
        raise ProjectionError("artifact must be a non-empty Wasm file with the Wasm magic")
    return path.resolve()


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def decode_wat_string(value: str, label: str) -> str:
    try:
        decoded = json.loads('"' + value + '"')
    except json.JSONDecodeError as error:
        raise ProjectionError(f"{label} contains an invalid WAT string") from error
    if not isinstance(decoded, str):
        raise ProjectionError(f"{label} is not a string")
    return decoded


def run_print(artifact: pathlib.Path, wasm_tools: pathlib.Path, work_dir: pathlib.Path) -> str:
    result = subprocess.run(
        [str(wasm_tools), "print", str(artifact)],
        cwd=work_dir,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).decode("utf-8", errors="replace").strip()
        raise ProjectionError(f"static artifact projection failed with exit {result.returncode}: {detail}")
    try:
        return result.stdout.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ProjectionError("static artifact projection output is not UTF-8") from error


def parse_wat(wat: str) -> Dict[str, List[Dict[str, Any]]]:
    imports = [
        {
            "module": decode_wat_string(module, "import module"),
            "name": decode_wat_string(name, "import name"),
            "kind": kind,
        }
        for module, name, kind in IMPORT.findall(wat)
    ]
    tables = []
    for minimum, maximum, element_type in TABLE.findall(wat):
        if element_type != "funcref":
            raise ProjectionError(f"unsupported Wasm table element type: {element_type}")
        tables.append({"min": int(minimum), "max": int(maximum) if maximum else None})
    exports = [
        {"name": decode_wat_string(name, "export name"), "kind": kind}
        for name, kind in EXPORT.findall(wat)
    ]
    return {"imports": imports, "tables": tables, "exports": exports}


def select_fixture(manifest: Dict[str, Any], fixture_id: str, target: str) -> Dict[str, Any]:
    matches = [fixture for fixture in manifest["fixtures"] if fixture["id"] == fixture_id]
    if not matches:
        raise ProjectionError(f"unknown fixture id: {fixture_id}")
    fixture = matches[0]
    if target not in fixture["targets"]:
        raise ProjectionError(f"fixture target is not declared: {target}")
    if fixture["kind"] != "valid" or not fixture["expected"]["artifact"]["required"]:
        raise ProjectionError("artifact projection requires a valid artifact fixture")
    if "wasm" not in fixture["observables"]:
        raise ProjectionError("fixture does not declare a Wasm artifact observable")
    return fixture


def project_fixture(
    fixture: Dict[str, Any], root: pathlib.Path, artifact: pathlib.Path, wasm_tools: pathlib.Path
) -> Dict[str, Any]:
    source = root / pathlib.PurePosixPath(fixture["source"])
    if source.is_symlink() or not source.is_file():
        raise ProjectionError(f"source fixture must be a regular file: {fixture['source']}")
    source = source.resolve()
    try:
        source.relative_to(root)
    except ValueError as error:
        raise ProjectionError("source fixture escapes the project root") from error
    projection = parse_wat(run_print(artifact, wasm_tools, artifact.parent))
    required = set(fixture["observables"]) & {"ftable", "imports"}
    if "imports" in required and not projection["imports"]:
        raise ProjectionError(f"{fixture['id']} declares imports but artifact has none")
    if "ftable" in required and not projection["tables"]:
        raise ProjectionError(f"{fixture['id']} declares ftable but artifact has no table")
    relative_source = source.relative_to(root).as_posix()
    return {
        "id": fixture["id"],
        "source": relative_source,
        "source_sha256": sha256(source),
        "artifact_sha256": sha256(artifact),
        "required_observables": sorted(required),
        **projection,
    }


def write_output(path: pathlib.Path, value: Dict[str, Any]) -> None:
    if not path.is_absolute() or path == pathlib.Path("/") or path.exists() or path.is_symlink() or not path.parent.is_dir():
        raise ProjectionError(f"output must be a new file under an existing directory: {path}")
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", dir=path.parent, prefix=f".{path.name}.", suffix=".tmp", delete=False) as stream:
            temporary = pathlib.Path(stream.name)
            json.dump(value, stream, ensure_ascii=False, indent=2, sort_keys=True)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        temporary = None
    except OSError as error:
        raise ProjectionError(f"projection output cannot be written: {path}: {error}") from error
    finally:
        if temporary is not None:
            try:
                temporary.unlink()
            except OSError:
                pass


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--fixture-id", required=True)
    parser.add_argument("--target", choices=SUPPORTED_TARGETS, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--artifact", type=pathlib.Path, required=True)
    parser.add_argument("--wasm-tools", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    arguments = parser.parse_args()
    try:
        if not SOURCE_COMMIT.fullmatch(arguments.source_commit):
            raise ProjectionError("source_commit must be a 40-character lowercase commit")
        root = require_absolute_directory(arguments.root, "root")
        wasm_tools = require_executable(arguments.wasm_tools, "wasm-tools")
        artifact = require_artifact(arguments.artifact)
        raw_manifest = json.loads(arguments.manifest.read_text(encoding="utf-8"))
        manifest = project_manifest(require_object(raw_manifest, "manifest"), root)
        fixture = select_fixture(manifest, arguments.fixture_id, arguments.target)
        result = {
            "schema_version": 1,
            "suite": "v4-m1-07",
            "producer": "static-wasm-artifact",
            "target": arguments.target,
            "source_commit": arguments.source_commit,
            "fixtures": [project_fixture(fixture, root, artifact, wasm_tools)],
        }
        write_output(arguments.output, result)
    except (OSError, UnicodeError, json.JSONDecodeError, ManifestError, ProjectionError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
