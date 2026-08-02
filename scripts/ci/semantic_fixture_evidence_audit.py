#!/usr/bin/env python3

"""Audit a V4 semantic-fixture evidence index without promoting weak evidence.

The index declares the fixture command, ADR, and required negative gates.  The
referenced Rust/native reports and diff result remain the authoritative
observations; this command recomputes the comparison before emitting a
deterministic evidence projection.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from typing import Any, Dict, List, Mapping, Tuple

from semantic_fixture_diff import (
    ObservationError,
    compare_reports,
    load_json,
    read_current_source_commit,
    validate_report,
)
from semantic_fixture_matrix import (
    ManifestError,
    SUPPORTED_TARGETS,
    expect_keys,
    project_manifest,
    require_object,
    require_string,
)


SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")
REQUIRED_GATES = {
    "fallback-forbidden",
    "network-forbidden",
    "source-commit-bound",
    "target-declared",
}
ALLOWED_STATUS = {"pass", "pending", "mismatch"}
ARTIFACT_LAYOUT = ("ci-artifacts", "v4-m1-01")


def safe_relative_file(
    value: Any,
    label: str,
    root: pathlib.Path,
    namespace: Tuple[str, ...] = (),
) -> Tuple[str, pathlib.Path]:
    relative_value = require_string(value, label)
    relative = pathlib.PurePosixPath(relative_value)
    if (
        relative.is_absolute()
        or not relative.parts
        or "." in relative.parts
        or ".." in relative.parts
        or "\\" in relative_value
    ):
        raise ObservationError(f"{label} must be a safe project-relative path")
    if namespace and relative.parts[: len(namespace)] != namespace:
        prefix = "/".join(namespace)
        raise ObservationError(
            f"{label} must be under {prefix}/ for the index source_commit and target"
        )
    path = root.joinpath(*relative.parts)
    candidate = root
    for part in relative.parts:
        candidate = candidate / part
        if candidate.is_symlink():
            raise ObservationError(f"{label} must not traverse symlinks")
    try:
        resolved = path.resolve(strict=True)
        resolved.relative_to(root)
    except ValueError as error:
        raise ObservationError(f"{label} escapes the project root") from error
    except OSError as error:
        raise ObservationError(f"{label} must reference a regular file: {relative.as_posix()}") from error
    if resolved != path:
        raise ObservationError(f"{label} must not traverse symlinks")
    if not path.is_file():
        raise ObservationError(f"{label} must reference a regular file: {relative.as_posix()}")
    return relative.as_posix(), path


def validate_gates(value: Any, label: str) -> Dict[str, str]:
    gates = require_object(value, label)
    expect_keys(gates, REQUIRED_GATES, label)
    normalized = {}
    for gate in sorted(REQUIRED_GATES):
        if gates[gate] != "pass":
            raise ObservationError(f"{label}.{gate} must be pass")
        normalized[gate] = "pass"
    return normalized


def load_index(
    path: pathlib.Path,
    root: pathlib.Path,
    manifest: Mapping[str, Any],
    current_source_commit: str = None,
) -> Dict[str, Any]:
    index = require_object(load_json(path, "evidence index"), "evidence index")
    expect_keys(
        index,
        (
            "schema_version",
            "suite",
            "task",
            "target",
            "source_commit",
            "status",
            "adr",
            "oracle_report",
            "native_report",
            "comparison",
            "fixtures",
        ),
        "evidence index",
    )
    if index["schema_version"] != 1 or index["suite"] != "v4-m1-06":
        raise ObservationError("evidence index schema_version/suite does not match v4-m1-06")
    task = require_string(index["task"], "evidence index.task")
    if not re.fullmatch(r"V4-M1-[0-9]{2}", task):
        raise ObservationError("evidence index.task must use V4-M1-##")
    expected_task = manifest["suite"].upper()
    if task != expected_task:
        raise ObservationError(
            f"evidence index.task must match fixture matrix suite: expected {expected_task}"
        )
    target = require_string(index["target"], "evidence index.target")
    if target not in SUPPORTED_TARGETS:
        raise ObservationError(f"evidence index.target is unsupported: {target}")
    source_commit = require_string(index["source_commit"], "evidence index.source_commit")
    if not SOURCE_COMMIT.fullmatch(source_commit):
        raise ObservationError("evidence index.source_commit must be a 40-character lowercase commit")
    if current_source_commit is not None and source_commit != current_source_commit:
        raise ObservationError("evidence index source_commit does not match current checkout HEAD")
    status = require_string(index["status"], "evidence index.status")
    if status not in ALLOWED_STATUS:
        raise ObservationError("evidence index.status must be pass, pending, or mismatch")
    adr, _ = safe_relative_file(index["adr"], "evidence index.adr", root)
    adr_path = pathlib.PurePosixPath(adr)
    if adr_path.parts[:2] != ("docs", "adr") or adr_path.suffix != ".md":
        raise ObservationError("evidence index.adr must reference a Markdown file under docs/adr")
    artifact_namespace = ARTIFACT_LAYOUT + (source_commit, target)
    try:
        index_relative = path.absolute().relative_to(root).as_posix()
    except ValueError as error:
        raise ObservationError("evidence index must be under its target-scoped artifact namespace") from error
    if pathlib.PurePosixPath(index_relative).name != "index.json":
        raise ObservationError("evidence index must be named index.json")
    safe_relative_file(index_relative, "evidence index", root, artifact_namespace)
    oracle_report, oracle_path = safe_relative_file(
        index["oracle_report"], "evidence index.oracle_report", root, artifact_namespace
    )
    native_report, native_path = safe_relative_file(
        index["native_report"], "evidence index.native_report", root, artifact_namespace
    )
    comparison, comparison_path = safe_relative_file(
        index["comparison"], "evidence index.comparison", root, artifact_namespace
    )

    entries = index["fixtures"]
    if not isinstance(entries, list) or not entries:
        raise ObservationError("evidence index.fixtures must be a non-empty array")
    fixture_by_id = {fixture["id"]: fixture for fixture in manifest["fixtures"]}
    identifiers: List[str] = []
    normalized_entries = []
    for index_number, value in enumerate(entries):
        label = f"evidence index.fixtures[{index_number}]"
        entry = require_object(value, label)
        expect_keys(entry, ("id", "command", "negative_gates"), label)
        identifier = require_string(entry["id"], f"{label}.id")
        if identifier not in fixture_by_id:
            raise ObservationError(f"{label}.id is not in the fixture matrix: {identifier}")
        command = require_string(entry["command"], f"{label}.command")
        fixture = fixture_by_id[identifier]
        if command not in fixture["commands"]:
            raise ObservationError(f"{label}.command is not declared for fixture: {command}")
        if fixture["expected"]["artifact"]["required"] and command not in {"compile", "build"}:
            raise ObservationError(
                f"{label}.command must be an artifact command (compile or build)"
            )
        identifiers.append(identifier)
        normalized_entries.append(
            {
                "id": identifier,
                "command": command,
                "negative_gates": validate_gates(entry["negative_gates"], f"{label}.negative_gates"),
            }
        )
    if len(set(identifiers)) != len(identifiers) or identifiers != sorted(identifiers):
        raise ObservationError("evidence index fixture ids must be unique and lexicographically sorted")
    selected_ids = identifiers
    return {
        "schema_version": 1,
        "suite": "v4-m1-06",
        "task": task,
        "target": target,
        "source_commit": source_commit,
        "status": status,
        "adr": adr,
        "reports": {
            "oracle": oracle_report,
            "native": native_report,
            "comparison": comparison,
        },
        "paths": {
            "oracle": oracle_path,
            "native": native_path,
            "comparison": comparison_path,
        },
        "fixtures": normalized_entries,
        "selected_ids": selected_ids,
    }


def selected_manifest(manifest: Mapping[str, Any], selected_ids: List[str]) -> Dict[str, Any]:
    selected = dict(manifest)
    selected["fixtures"] = [fixture for fixture in manifest["fixtures"] if fixture["id"] in set(selected_ids)]
    selected["fixture_count"] = len(selected["fixtures"])
    return selected


def fixture_projection(
    entry: Mapping[str, Any], oracle: Mapping[str, Any], native: Mapping[str, Any]
) -> Dict[str, Any]:
    identifier = entry["id"]
    oracle_fixture = next(fixture for fixture in oracle["fixtures"] if fixture["id"] == identifier)
    native_fixture = next(fixture for fixture in native["fixtures"] if fixture["id"] == identifier)
    return {
        "id": identifier,
        "command": entry["command"],
        "negative_gates": entry["negative_gates"],
        "source_sha256": {
            "oracle": oracle_fixture["source_sha256"],
            "native": native_fixture["source_sha256"],
        },
        "diagnostics": {
            "oracle": oracle_fixture["diagnostics"],
            "native": native_fixture["diagnostics"],
        },
        "exit_code": {
            "oracle": oracle_fixture["exit_code"],
            "native": native_fixture["exit_code"],
        },
        "artifact": {
            "oracle": oracle_fixture["artifact"],
            "native": native_fixture["artifact"],
        },
        "runtime": {
            "oracle": oracle_fixture["runtime"],
            "native": native_fixture["runtime"],
        },
    }


def audit(index: Dict[str, Any], manifest: Mapping[str, Any]) -> Dict[str, Any]:
    paths = index["paths"]
    selected_ids = index["selected_ids"]
    manifest_fixtures = {fixture["id"]: fixture for fixture in manifest["fixtures"]}
    oracle = validate_report(
        load_json(paths["oracle"], "oracle report"),
        "oracle report",
        manifest_fixtures,
        selected_ids,
        "rust-oracle",
    )
    native = validate_report(
        load_json(paths["native"], "native report"),
        "native report",
        manifest_fixtures,
        selected_ids,
        "native-stage0",
    )
    if oracle["target"] != index["target"] or native["target"] != index["target"]:
        raise ObservationError("evidence index target does not match both reports")
    if oracle["source_commit"] != index["source_commit"] or native["source_commit"] != index["source_commit"]:
        raise ObservationError("evidence index source_commit does not match both reports")
    comparison = load_json(paths["comparison"], "comparison report")
    selected = selected_manifest(manifest, selected_ids)
    recomputed = compare_reports(selected, oracle, native)
    if comparison != recomputed:
        raise ObservationError("comparison report does not match the referenced reports")
    if index["status"] != recomputed["status"]:
        raise ObservationError(
            f"evidence index.status does not match comparison status: index={index['status']} comparison={recomputed['status']}"
        )
    entries = index["fixtures"]
    return {
        "schema_version": 1,
        "suite": "v4-m1-06",
        "task": index["task"],
        "target": index["target"],
        "source_commit": index["source_commit"],
        "status": recomputed["status"],
        "adr": index["adr"],
        "reports": index["reports"],
        "fixture_count": len(entries),
        "pending_boundaries": recomputed["pending_boundaries"],
        "mismatches": recomputed["mismatches"],
        "fixtures": [fixture_projection(entry, oracle, native) for entry in entries],
    }


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--index", type=pathlib.Path, required=True)
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        root = arguments.root.resolve()
        raw_manifest = load_json(arguments.manifest, "fixture matrix")
        manifest = project_manifest(require_object(raw_manifest, "manifest"), root)
        current_source_commit = read_current_source_commit(root)
        index = load_index(arguments.index, root, manifest, current_source_commit)
        result = audit(index, manifest)
    except (OSError, json.JSONDecodeError, ManifestError, ObservationError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
    if result["status"] == "pass":
        return 0
    if result["status"] == "pending":
        return 2
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
