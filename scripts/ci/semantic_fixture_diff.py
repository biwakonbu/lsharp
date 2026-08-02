#!/usr/bin/env python3

"""Compare one Rust-oracle report with one native-stage0 report.

This helper compares only observable contract fields.  Pending artifact or
runtime observations produce exit code 2, never a false success. Observed
runtime entries must identify the exact observed artifact digest they executed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import subprocess
import sys
from typing import Any, Dict, List, Mapping

from semantic_fixture_matrix import (
    DIAGNOSTIC_CODE,
    ManifestError,
    SUPPORTED_TARGETS,
    expect_keys,
    project_manifest,
    require_object,
    require_positive_int,
    require_string,
    validate_span,
)


SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^sha256:[0-9a-f]{64}$")
REPORT_PRODUCERS = {"rust-oracle", "native-stage0"}


class ObservationError(ValueError):
    """A report cannot be compared under the matrix contract."""


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return "sha256:" + digest.hexdigest()


def read_current_source_commit(root: pathlib.Path) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "rev-parse", "--verify", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise ObservationError("unable to resolve current source commit") from error
    source_commit = result.stdout.strip()
    if not SOURCE_COMMIT.fullmatch(source_commit):
        raise ObservationError("current source commit is not a 40-character lowercase commit")
    return source_commit


def load_json(path: pathlib.Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ObservationError(f"{label} JSON is invalid: {error}") from error


def validate_diagnostics(value: Any, label: str) -> List[Dict[str, Any]]:
    if not isinstance(value, list):
        raise ObservationError(f"{label} must be an array")
    diagnostics = []
    for index, value in enumerate(value):
        diagnostic = require_object(value, f"{label}[{index}]")
        expect_keys(diagnostic, ("code", "span"), f"{label}[{index}]")
        code = require_string(diagnostic["code"], f"{label}[{index}].code")
        if not DIAGNOSTIC_CODE.fullmatch(code):
            raise ObservationError(f"{label}[{index}].code is not an LS#### code")
        diagnostics.append(
            {"code": code, "span": validate_span(diagnostic["span"], f"{label}[{index}].span")}
        )
    return diagnostics


def validate_artifact(value: Any, kind: str, label: str) -> Dict[str, Any]:
    artifact = require_object(value, label)
    status = require_string(artifact.get("status"), f"{label}.status")
    if status == "not-applicable":
        expect_keys(artifact, ("status",), label)
        if kind != "invalid":
            raise ObservationError(f"{label} valid fixture cannot omit its artifact boundary")
        return {"status": status}
    if status == "pending":
        expect_keys(artifact, ("status",), label)
        if kind != "valid":
            raise ObservationError(f"{label} invalid fixture cannot have pending artifact")
        return {"status": status}
    if status != "observed":
        raise ObservationError(f"{label}.status must be observed, pending, or not-applicable")
    expect_keys(artifact, ("status", "sha256", "size"), label)
    digest = require_string(artifact["sha256"], f"{label}.sha256")
    if not SHA256.fullmatch(digest):
        raise ObservationError(f"{label}.sha256 must use sha256:<64 lowercase hex>")
    size = artifact["size"]
    if isinstance(size, bool) or not isinstance(size, int) or size <= 0:
        raise ObservationError(f"{label}.size must be a positive integer")
    if kind != "valid":
        raise ObservationError(f"{label} invalid fixture cannot produce an artifact")
    return {"status": status, "sha256": digest, "size": size}


def validate_runtime(value: Any, kind: str, label: str) -> Dict[str, Any]:
    runtime = require_object(value, label)
    status = require_string(runtime.get("status"), f"{label}.status")
    if status in {"pending", "not-run"}:
        expect_keys(runtime, ("status", "exit_code", "stdout", "stderr", "artifact_sha256"), label)
        if (
            runtime["exit_code"] is not None
            or runtime["stdout"] is not None
            or runtime["stderr"] is not None
            or runtime["artifact_sha256"] is not None
        ):
            raise ObservationError(f"{label} pending/not-run output must be null")
        if kind == "invalid" and status != "not-run":
            raise ObservationError(f"{label} invalid fixture runtime must be not-run")
        return dict(runtime)
    if status != "observed":
        raise ObservationError(f"{label}.status must be observed, pending, or not-run")
    expect_keys(runtime, ("status", "exit_code", "stdout", "stderr", "artifact_sha256"), label)
    exit_code = runtime["exit_code"]
    if isinstance(exit_code, bool) or not isinstance(exit_code, int):
        raise ObservationError(f"{label}.exit_code must be an integer")
    if not isinstance(runtime["stdout"], str) or not isinstance(runtime["stderr"], str):
        raise ObservationError(f"{label} observed output must be strings")
    artifact_sha256 = require_string(runtime["artifact_sha256"], f"{label}.artifact_sha256")
    if not SHA256.fullmatch(artifact_sha256):
        raise ObservationError(f"{label}.artifact_sha256 must use sha256:<64 lowercase hex>")
    if kind != "valid":
        raise ObservationError(f"{label} invalid fixture cannot run")
    return dict(runtime)


def validate_source_sha256(value: Any, label: str) -> str:
    source_sha256 = require_string(value, label)
    if not SHA256.fullmatch(source_sha256):
        raise ObservationError(f"{label} must use sha256:<64 lowercase hex>")
    return source_sha256


def validate_report(
    value: Any,
    label: str,
    manifest_fixtures: Mapping[str, Mapping[str, Any]],
    required_ids: List[str] = None,
    expected_producer: str = None,
) -> Dict[str, Any]:
    report = require_object(value, label)
    expect_keys(report, ("schema_version", "suite", "producer", "target", "source_commit", "fixtures"), label)
    if report["schema_version"] != 1 or report["suite"] != "v4-m1-01":
        raise ObservationError(f"{label} schema_version/suite does not match v4-m1-01")
    producer = require_string(report["producer"], f"{label}.producer")
    if producer not in REPORT_PRODUCERS:
        raise ObservationError(f"{label}.producer is unsupported: {producer}")
    if expected_producer is not None and producer != expected_producer:
        raise ObservationError(f"{label}.producer must be {expected_producer}")
    target = require_string(report["target"], f"{label}.target")
    if target not in SUPPORTED_TARGETS:
        raise ObservationError(f"{label}.target is unsupported: {target}")
    source_commit = require_string(report["source_commit"], f"{label}.source_commit")
    if not SOURCE_COMMIT.fullmatch(source_commit):
        raise ObservationError(f"{label}.source_commit must be a 40-character lowercase commit")
    fixtures = report["fixtures"]
    if not isinstance(fixtures, list) or not fixtures:
        raise ObservationError(f"{label}.fixtures must be a non-empty array")
    result = []
    ids = []
    for index, fixture_value in enumerate(fixtures):
        fixture_label = f"{label}.fixtures[{index}]"
        fixture = require_object(fixture_value, fixture_label)
        expect_keys(
            fixture,
            ("id", "source_sha256", "diagnostics", "exit_code", "artifact", "runtime"),
            fixture_label,
        )
        identifier = require_string(fixture["id"], f"{fixture_label}.id")
        if identifier not in manifest_fixtures:
            raise ObservationError(f"{fixture_label}.id is not in the fixture matrix: {identifier}")
        ids.append(identifier)
        source_sha256 = validate_source_sha256(
            fixture["source_sha256"], f"{fixture_label}.source_sha256"
        )
        exit_code = fixture["exit_code"]
        if isinstance(exit_code, bool) or not isinstance(exit_code, int):
            raise ObservationError(f"{fixture_label}.exit_code must be an integer")
        kind = manifest_fixtures[identifier]["kind"]
        artifact = validate_artifact(fixture["artifact"], kind, f"{fixture_label}.artifact")
        runtime = validate_runtime(fixture["runtime"], kind, f"{fixture_label}.runtime")
        if runtime["status"] == "observed":
            if artifact["status"] != "observed":
                raise ObservationError(
                    f"{fixture_label}.runtime cannot be observed without an observed artifact"
                )
            if runtime["artifact_sha256"] != artifact["sha256"]:
                raise ObservationError(
                    f"{fixture_label}.runtime.artifact_sha256 must match "
                    f"{fixture_label}.artifact.sha256"
                )
        result.append(
            {
                "id": identifier,
                "source_sha256": source_sha256,
                "diagnostics": validate_diagnostics(fixture["diagnostics"], f"{fixture_label}.diagnostics"),
                "exit_code": exit_code,
                "artifact": artifact,
                "runtime": runtime,
            }
        )
    if len(set(ids)) != len(ids) or ids != sorted(ids):
        raise ObservationError(f"{label}.fixtures ids must be unique and lexicographically sorted")
    expected_ids = sorted(required_ids if required_ids is not None else manifest_fixtures)
    if ids != expected_ids:
        raise ObservationError(f"{label}.fixtures must contain exactly the selected fixture matrix ids")
    return {
        "producer": producer,
        "target": target,
        "source_commit": source_commit,
        "fixtures": result,
    }


def mismatch(items: List[Dict[str, Any]], fixture: str, field: str, oracle: Any, native: Any) -> None:
    items.append({"fixture": fixture, "field": field, "oracle": oracle, "native": native})


def compare_reports(manifest: Mapping[str, Any], oracle: Dict[str, Any], native: Dict[str, Any]) -> Dict[str, Any]:
    if oracle["target"] != native["target"]:
        raise ObservationError(
            f"target mismatch: oracle={oracle['target']} native={native['target']}"
        )
    if oracle["source_commit"] != native["source_commit"]:
        raise ObservationError(
            "source_commit mismatch: "
            f"oracle={oracle['source_commit']} native={native['source_commit']}"
        )
    oracle_by_id = {fixture["id"]: fixture for fixture in oracle["fixtures"]}
    native_by_id = {fixture["id"]: fixture for fixture in native["fixtures"]}
    mismatches: List[Dict[str, Any]] = []
    pending: List[str] = []
    for expected in manifest["fixtures"]:
        identifier = expected["id"]
        oracle_fixture = oracle_by_id[identifier]
        native_fixture = native_by_id[identifier]
        expected_result = expected["expected"]
        if oracle_fixture["source_sha256"] != native_fixture["source_sha256"]:
            mismatch(
                mismatches,
                identifier,
                "source_sha256",
                oracle_fixture["source_sha256"],
                native_fixture["source_sha256"],
            )
        for field in ("diagnostics", "exit_code"):
            if oracle_fixture[field] != native_fixture[field]:
                mismatch(mismatches, identifier, field, oracle_fixture[field], native_fixture[field])
            if oracle_fixture[field] != expected_result[field]:
                mismatch(mismatches, identifier, f"oracle.{field}", expected_result[field], oracle_fixture[field])
            if native_fixture[field] != expected_result[field]:
                mismatch(mismatches, identifier, f"native.{field}", expected_result[field], native_fixture[field])

        if expected["kind"] == "valid":
            oracle_artifact = oracle_fixture["artifact"]
            native_artifact = native_fixture["artifact"]
            if oracle_artifact["status"] != native_artifact["status"]:
                mismatch(
                    mismatches,
                    identifier,
                    "artifact.status",
                    oracle_artifact["status"],
                    native_artifact["status"],
                )
            elif oracle_artifact["status"] == "observed":
                if oracle_artifact != native_artifact:
                    mismatch(mismatches, identifier, "artifact", oracle_artifact, native_artifact)
            else:
                pending.append(identifier + ".artifact")

            oracle_runtime = oracle_fixture["runtime"]
            native_runtime = native_fixture["runtime"]
            if oracle_runtime["status"] != native_runtime["status"]:
                mismatch(
                    mismatches,
                    identifier,
                    "runtime.status",
                    oracle_runtime["status"],
                    native_runtime["status"],
                )
            elif oracle_runtime["status"] == "observed":
                for field in ("exit_code", "stdout", "stderr", "artifact_sha256"):
                    if oracle_runtime[field] != native_runtime[field]:
                        mismatch(
                            mismatches,
                            identifier,
                            "runtime." + field,
                            oracle_runtime[field],
                            native_runtime[field],
                        )
                for producer, runtime in (("oracle", oracle_runtime), ("native", native_runtime)):
                    for field in ("exit_code", "stdout", "stderr"):
                        if runtime[field] != expected_result["runtime"][field]:
                            mismatch(
                                mismatches,
                                identifier,
                                f"{producer}.runtime.{field}",
                                expected_result["runtime"][field],
                                runtime[field],
                            )
            else:
                pending.append(identifier + ".runtime")

    status = "mismatch" if mismatches else "pending" if pending else "pass"
    return {
        "schema_version": 1,
        "suite": "v4-m1-01",
        "target": oracle["target"],
        "source_commit": oracle["source_commit"],
        "fixture_count": len(manifest["fixtures"]),
        "status": status,
        "pending_boundaries": sorted(set(pending)),
        "mismatches": mismatches,
    }


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--root", type=pathlib.Path, required=True)
    parser.add_argument("--oracle", type=pathlib.Path, required=True)
    parser.add_argument("--native", type=pathlib.Path, required=True)
    parser.add_argument(
        "--fixture-id",
        action="append",
        dest="fixture_ids",
        help="compare only this fixture (repeat for a selected subset)",
    )
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        raw_manifest = load_json(arguments.manifest, "fixture matrix")
        projected_manifest = project_manifest(require_object(raw_manifest, "manifest"), arguments.root.resolve())
        manifest_fixtures = {fixture["id"]: fixture for fixture in projected_manifest["fixtures"]}
        selected_ids = arguments.fixture_ids or sorted(manifest_fixtures)
        if len(set(selected_ids)) != len(selected_ids):
            raise ObservationError("--fixture-id values must be unique")
        unknown_ids = sorted(set(selected_ids) - set(manifest_fixtures))
        if unknown_ids:
            raise ObservationError("unknown --fixture-id: " + ", ".join(unknown_ids))
        oracle = validate_report(
            load_json(arguments.oracle, "oracle report"),
            "oracle report",
            manifest_fixtures,
            selected_ids,
            "rust-oracle",
        )
        native = validate_report(
            load_json(arguments.native, "native report"),
            "native report",
            manifest_fixtures,
            selected_ids,
            "native-stage0",
        )
        current_source_commit = read_current_source_commit(arguments.root.resolve())
        if (
            oracle["source_commit"] != current_source_commit
            or native["source_commit"] != current_source_commit
        ):
            raise ObservationError("report source_commit does not match current checkout HEAD")
        for report_label, report in (("oracle", oracle), ("native", native)):
            for fixture in report["fixtures"]:
                source = arguments.root / pathlib.PurePosixPath(
                    manifest_fixtures[fixture["id"]]["source"]
                )
                if fixture["source_sha256"] != sha256(source):
                    raise ObservationError(
                        f"{report_label} source_sha256 does not match current fixture: {fixture['id']}"
                    )
        selected_manifest = dict(projected_manifest)
        selected_manifest["fixtures"] = [
            fixture for fixture in projected_manifest["fixtures"] if fixture["id"] in set(selected_ids)
        ]
        selected_manifest["fixture_count"] = len(selected_manifest["fixtures"])
        result = compare_reports(selected_manifest, oracle, native)
    except (OSError, json.JSONDecodeError, ManifestError, ObservationError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2, sort_keys=True))
    return 0 if result["status"] == "pass" else 2 if result["status"] == "pending" else 1


if __name__ == "__main__":
    raise SystemExit(main())
