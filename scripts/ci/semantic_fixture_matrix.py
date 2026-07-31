#!/usr/bin/env python3

"""Validate and project the v0.4 semantic fixture matrix.

The manifest is an inventory contract, not runtime evidence.  Pending artifact
and target results stay explicit until the corresponding Rust/native gates run.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from typing import Any, Dict, Iterable, List, Mapping, Set


SUPPORTED_TARGETS = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]
ALLOWED_LAYERS = {"syntax", "types", "ir", "codegen", "runtime", "public"}
ALLOWED_OBSERVABLES = {"ast", "type", "ir", "wasm", "runtime", "report"}
ALLOWED_COMMANDS = {"check", "compile", "test", "build"}
DIAGNOSTIC_CODE = re.compile(r"^LS[0-9]{4}$")


class ManifestError(ValueError):
    """The fixture matrix does not satisfy its versioned contract."""


def expect_keys(value: Mapping[str, Any], required: Iterable[str], label: str) -> None:
    required_set = set(required)
    actual = set(value)
    missing = sorted(required_set - actual)
    unexpected = sorted(actual - required_set)
    if missing or unexpected:
        details = []
        if missing:
            details.append("missing=" + ",".join(missing))
        if unexpected:
            details.append("unexpected=" + ",".join(unexpected))
        raise ManifestError(f"{label} fields invalid ({'; '.join(details)})")


def require_object(value: Any, label: str) -> Dict[str, Any]:
    if not isinstance(value, dict):
        raise ManifestError(f"{label} must be an object")
    return value


def require_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise ManifestError(f"{label} must be a non-empty string")
    return value


def require_string_list(value: Any, label: str, allowed: Set[str] = None) -> List[str]:
    if not isinstance(value, list) or not value or any(not isinstance(item, str) for item in value):
        raise ManifestError(f"{label} must be a non-empty string array")
    if len(set(value)) != len(value):
        raise ManifestError(f"{label} must not contain duplicates")
    if allowed is not None:
        unknown = sorted(set(value) - allowed)
        if unknown:
            raise ManifestError(f"{label} contains unsupported values: {', '.join(unknown)}")
    return list(value)


def require_positive_int(value: Any, label: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 1:
        raise ManifestError(f"{label} must be a positive integer")
    return value


def validate_span(value: Any, label: str) -> Dict[str, Dict[str, int]]:
    span = require_object(value, label)
    expect_keys(span, ("start", "end"), label)
    result: Dict[str, Dict[str, int]] = {}
    for edge in ("start", "end"):
        point = require_object(span[edge], f"{label}.{edge}")
        expect_keys(point, ("line", "column"), f"{label}.{edge}")
        result[edge] = {
            "line": require_positive_int(point["line"], f"{label}.{edge}.line"),
            "column": require_positive_int(point["column"], f"{label}.{edge}.column"),
        }
    return result


def validate_source(root: pathlib.Path, source: Any, label: str) -> str:
    relative = pathlib.PurePosixPath(require_string(source, label))
    if relative.is_absolute() or ".." in relative.parts or "\\" in relative.parts:
        raise ManifestError(f"{label} is unsafe; use a project-relative path")
    if relative.suffix != ".ls":
        raise ManifestError(f"{label} must point to an .ls source fixture")
    path = root.joinpath(*relative.parts)
    try:
        path.relative_to(root)
    except ValueError as error:
        raise ManifestError(f"{label} escapes the project root") from error
    if not path.is_file():
        raise ManifestError(f"{label} does not exist: {relative.as_posix()}")
    return relative.as_posix()


def validate_execution(value: Any, label: str) -> Dict[str, str]:
    execution = require_object(value, label)
    expect_keys(execution, ("stage0", "fallback", "network"), label)
    result = {
        key: require_string(execution[key], f"{label}.{key}")
        for key in ("stage0", "fallback", "network")
    }
    expected = {
        "stage0": "current-source",
        "fallback": "forbidden",
        "network": "forbidden",
    }
    for key, expected_value in expected.items():
        if result[key] != expected_value:
            raise ManifestError(f"{label}.{key} must be {expected_value}")
    return result


def validate_expected(value: Any, kind: str, label: str) -> Dict[str, Any]:
    expected = require_object(value, label)
    expect_keys(expected, ("diagnostics", "exit_code", "artifact", "runtime"), label)
    diagnostics = expected["diagnostics"]
    if not isinstance(diagnostics, list):
        raise ManifestError(f"{label}.diagnostics must be an array")
    normalized_diagnostics = []
    for index, diagnostic_value in enumerate(diagnostics):
        diagnostic = require_object(diagnostic_value, f"{label}.diagnostics[{index}]")
        expect_keys(diagnostic, ("code", "span"), f"{label}.diagnostics[{index}]")
        code = require_string(diagnostic["code"], f"{label}.diagnostics[{index}].code")
        if not DIAGNOSTIC_CODE.fullmatch(code):
            raise ManifestError(f"{label}.diagnostics[{index}].code is not an LS#### code")
        normalized_diagnostics.append(
            {"code": code, "span": validate_span(diagnostic["span"], f"{label}.diagnostics[{index}].span")}
        )

    exit_code = expected["exit_code"]
    if isinstance(exit_code, bool) or not isinstance(exit_code, int):
        raise ManifestError(f"{label}.exit_code must be an integer")

    artifact = require_object(expected["artifact"], f"{label}.artifact")
    expect_keys(artifact, ("required", "status"), f"{label}.artifact")
    if not isinstance(artifact["required"], bool):
        raise ManifestError(f"{label}.artifact.required must be boolean")
    artifact_status = require_string(artifact["status"], f"{label}.artifact.status")

    runtime = require_object(expected["runtime"], f"{label}.runtime")
    expect_keys(runtime, ("status", "exit_code", "stdout", "stderr"), f"{label}.runtime")
    runtime_status = require_string(runtime["status"], f"{label}.runtime.status")

    if kind == "valid":
        if diagnostics or exit_code != 0:
            raise ManifestError(f"{label} valid fixture must have no diagnostics and exit_code 0")
        if artifact != {"required": True, "status": "pending"}:
            raise ManifestError(f"{label} valid fixture artifact evidence must remain pending")
        if runtime_status != "expected" or runtime["exit_code"] != 0:
            raise ManifestError(f"{label} valid fixture must declare expected runtime success")
        if not isinstance(runtime["stdout"], str) or not isinstance(runtime["stderr"], str):
            raise ManifestError(f"{label} valid fixture runtime output must be strings")
    else:
        if not diagnostics or exit_code == 0:
            raise ManifestError(f"{label} invalid fixture must declare a diagnostic and non-zero exit_code")
        if artifact != {"required": False, "status": "not-applicable"}:
            raise ManifestError(f"{label} invalid fixture must not claim an artifact")
        if runtime != {"status": "not-run", "exit_code": None, "stdout": None, "stderr": None}:
            raise ManifestError(f"{label} invalid fixture runtime must be not-run")

    return {
        "diagnostics": normalized_diagnostics,
        "exit_code": exit_code,
        "artifact": {"required": artifact["required"], "status": artifact_status},
        "runtime": {
            "status": runtime_status,
            "exit_code": runtime["exit_code"],
            "stdout": runtime["stdout"],
            "stderr": runtime["stderr"],
        },
    }


def project_manifest(manifest: Mapping[str, Any], root: pathlib.Path) -> Dict[str, Any]:
    expect_keys(manifest, ("schema_version", "suite", "targets", "execution", "fixtures"), "manifest")
    if manifest["schema_version"] != 1:
        raise ManifestError("manifest.schema_version must be 1")
    if manifest["suite"] != "v4-m1-01":
        raise ManifestError("manifest.suite must be v4-m1-01")
    targets = require_string_list(manifest["targets"], "manifest.targets")
    if targets != SUPPORTED_TARGETS:
        raise ManifestError("manifest.targets must explicitly list the two supported targets in order")
    execution = validate_execution(manifest["execution"], "manifest.execution")
    fixtures = manifest["fixtures"]
    if not isinstance(fixtures, list) or not fixtures:
        raise ManifestError("manifest.fixtures must be a non-empty array")

    projected_fixtures = []
    identifiers = []
    for index, fixture_value in enumerate(fixtures):
        label = f"manifest.fixtures[{index}]"
        fixture = require_object(fixture_value, label)
        expect_keys(
            fixture,
            ("id", "source", "kind", "layers", "observables", "targets", "commands", "execution", "expected"),
            label,
        )
        identifier = require_string(fixture["id"], f"{label}.id")
        identifiers.append(identifier)
        kind = require_string(fixture["kind"], f"{label}.kind")
        if kind not in {"valid", "invalid"}:
            raise ManifestError(f"{label}.kind must be valid or invalid")
        fixture_targets = require_string_list(fixture["targets"], f"{label}.targets")
        if fixture_targets != targets:
            raise ManifestError(f"{label}.targets must match manifest.targets explicitly")
        layers = require_string_list(fixture["layers"], f"{label}.layers", ALLOWED_LAYERS)
        observables = require_string_list(
            fixture["observables"], f"{label}.observables", ALLOWED_OBSERVABLES
        )
        if "report" not in observables:
            raise ManifestError(f"{label}.observables must include report")
        commands = require_string_list(fixture["commands"], f"{label}.commands", ALLOWED_COMMANDS)
        fixture_execution = validate_execution(fixture["execution"], f"{label}.execution")
        if fixture_execution != execution:
            raise ManifestError(f"{label}.execution must match manifest.execution")
        projected_fixtures.append(
            {
                "id": identifier,
                "source": validate_source(root, fixture["source"], f"{label}.source"),
                "kind": kind,
                "layers": layers,
                "observables": observables,
                "targets": fixture_targets,
                "commands": commands,
                "execution": fixture_execution,
                "expected": validate_expected(fixture["expected"], kind, f"{label}.expected"),
            }
        )

    if len(set(identifiers)) != len(identifiers):
        raise ManifestError("fixture ids must be unique")
    if identifiers != sorted(identifiers):
        raise ManifestError("fixture ids must be lexicographically sorted")
    return {
        "schema_version": 1,
        "suite": "v4-m1-01",
        "targets": targets,
        "execution": execution,
        "fixture_count": len(projected_fixtures),
        "fixtures": projected_fixtures,
    }


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument(
        "--root",
        type=pathlib.Path,
        default=pathlib.Path(__file__).resolve().parents[2],
        help="repository root used to resolve project-relative source fixtures",
    )
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        manifest = json.loads(arguments.manifest.read_text(encoding="utf-8"))
        projected = project_manifest(require_object(manifest, "manifest"), arguments.root.resolve())
    except (OSError, json.JSONDecodeError, ManifestError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(json.dumps(projected, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
