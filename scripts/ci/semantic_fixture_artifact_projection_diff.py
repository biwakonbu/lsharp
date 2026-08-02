#!/usr/bin/env python3

"""Compare Rust/native static Wasm projections without instantiating Wasm."""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from typing import Any, Dict


SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")
REQUIRED_FIXTURE_KEYS = {
    "id",
    "source",
    "source_sha256",
    "artifact_sha256",
    "required_observables",
    "imports",
    "tables",
    "exports",
}


class ProjectionDiffError(ValueError):
    pass


def load(path: pathlib.Path, label: str) -> Dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProjectionDiffError(f"{label} is invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise ProjectionDiffError(f"{label} must be an object")
    if value.get("schema_version") != 1 or value.get("suite") != "v4-m1-07":
        raise ProjectionDiffError(f"{label} schema_version/suite is invalid")
    if value.get("producer") != "static-wasm-artifact":
        raise ProjectionDiffError(f"{label}.producer is invalid")
    if not isinstance(value.get("target"), str) or not isinstance(value.get("source_commit"), str):
        raise ProjectionDiffError(f"{label} target/source_commit is invalid")
    if not SOURCE_COMMIT.fullmatch(value["source_commit"]):
        raise ProjectionDiffError(f"{label}.source_commit is invalid")
    fixtures = value.get("fixtures")
    if not isinstance(fixtures, list) or not fixtures:
        raise ProjectionDiffError(f"{label}.fixtures must be non-empty")
    ids = []
    for index, fixture in enumerate(fixtures):
        if not isinstance(fixture, dict) or set(fixture) != REQUIRED_FIXTURE_KEYS:
            raise ProjectionDiffError(f"{label}.fixtures[{index}] has an invalid closed shape")
        if not isinstance(fixture["id"], str) or fixture["id"] in ids:
            raise ProjectionDiffError(f"{label}.fixtures[{index}].id is invalid or duplicated")
        if not SHA256.fullmatch(fixture["source_sha256"]) or not SHA256.fullmatch(fixture["artifact_sha256"]):
            raise ProjectionDiffError(f"{label}.fixtures[{index}] digest is invalid")
        for field in ("required_observables", "imports", "tables", "exports"):
            if not isinstance(fixture[field], list):
                raise ProjectionDiffError(f"{label}.fixtures[{index}].{field} must be an array")
        ids.append(fixture["id"])
    if ids != sorted(ids):
        raise ProjectionDiffError(f"{label}.fixtures must be sorted by id")
    return value


def bind_report(projection: Dict[str, Any], report_path: pathlib.Path, label: str) -> None:
    try:
        report = json.loads(report_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ProjectionDiffError(f"{label} is invalid JSON: {error}") from error
    if not isinstance(report, dict):
        raise ProjectionDiffError(f"{label} must be an object")
    if report.get("target") != projection["target"]:
        raise ProjectionDiffError(f"{label} target does not match projection")
    if report.get("source_commit") != projection["source_commit"]:
        raise ProjectionDiffError(f"{label} source_commit does not match projection")
    report_fixtures = {fixture.get("id"): fixture for fixture in report.get("fixtures", []) if isinstance(fixture, dict)}
    for projected in projection["fixtures"]:
        observed = report_fixtures.get(projected["id"])
        if observed is None:
            raise ProjectionDiffError(f"{label} is missing fixture {projected['id']}")
        artifact = observed.get("artifact")
        if not isinstance(artifact, dict) or artifact.get("status") != "observed":
            raise ProjectionDiffError(f"{label} artifact for {projected['id']} is not observed")
        if artifact.get("sha256") != projected["artifact_sha256"]:
            raise ProjectionDiffError(
                f"{label} artifact digest does not match projection for {projected['id']}"
            )
        runtime = observed.get("runtime")
        if not isinstance(runtime, dict):
            raise ProjectionDiffError(f"{label} runtime for {projected['id']} is invalid")
        if runtime.get("status") == "observed" and runtime.get("artifact_sha256") != projected["artifact_sha256"]:
            raise ProjectionDiffError(
                f"{label} runtime artifact digest does not match projection for {projected['id']}"
            )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--oracle", type=pathlib.Path, required=True)
    parser.add_argument("--native", type=pathlib.Path, required=True)
    parser.add_argument("--oracle-report", type=pathlib.Path)
    parser.add_argument("--native-report", type=pathlib.Path)
    arguments = parser.parse_args()
    try:
        oracle = load(arguments.oracle, "oracle projection")
        native = load(arguments.native, "native projection")
        if bool(arguments.oracle_report) != bool(arguments.native_report):
            raise ProjectionDiffError("oracle/native report bindings must be supplied together")
        if arguments.oracle_report and arguments.native_report:
            bind_report(oracle, arguments.oracle_report, "oracle report")
            bind_report(native, arguments.native_report, "native report")
        if oracle["target"] != native["target"]:
            raise ProjectionDiffError("target mismatch")
        if oracle["source_commit"] != native["source_commit"]:
            raise ProjectionDiffError("source_commit mismatch")
        oracle_by_id = {fixture["id"]: fixture for fixture in oracle["fixtures"]}
        native_by_id = {fixture["id"]: fixture for fixture in native["fixtures"]}
        if sorted(oracle_by_id) != sorted(native_by_id):
            raise ProjectionDiffError("fixture id set mismatch")
        mismatches = []
        for identifier in sorted(oracle_by_id):
            for field in ("source", "source_sha256", "artifact_sha256", "required_observables", "imports", "tables", "exports"):
                if oracle_by_id[identifier][field] != native_by_id[identifier][field]:
                    mismatches.append({"fixture": identifier, "field": field, "oracle": oracle_by_id[identifier][field], "native": native_by_id[identifier][field]})
        result = {
            "schema_version": 1,
            "suite": "v4-m1-07",
            "target": oracle["target"],
            "source_commit": oracle["source_commit"],
            "status": "mismatch" if mismatches else "pass",
            "mismatches": mismatches,
        }
        print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
        return 1 if mismatches else 0
    except ProjectionDiffError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
