#!/usr/bin/env python3

"""Audit both supported V4 semantic-fixture evidence indexes as one gate.

Each target index is re-audited from its raw reports and comparison.  A single
pending or mismatching target therefore cannot be promoted by an aggregate
index that merely claims ``pass``.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import sys
from typing import Any, Dict, List, Mapping

from semantic_fixture_diff import ObservationError, load_json, read_current_source_commit
from semantic_fixture_evidence_audit import audit as audit_target
from semantic_fixture_evidence_audit import load_index as load_target_index
from semantic_fixture_evidence_audit import safe_relative_file
from semantic_fixture_matrix import (
    ManifestError,
    SUPPORTED_TARGETS,
    expect_keys,
    project_manifest,
    require_object,
    require_string,
)


SOURCE_COMMIT = re.compile(r"^[0-9a-f]{40}$")
ALLOWED_STATUS = {"pass", "pending", "mismatch"}
AGGREGATE_LAYOUT = ("ci-artifacts", "v4-m1-01")


def relative_path(path: pathlib.Path, root: pathlib.Path, label: str) -> str:
    try:
        return path.absolute().relative_to(root).as_posix()
    except ValueError as error:
        raise ObservationError(f"{label} must be under the project root") from error


def load_aggregate(
    path: pathlib.Path,
    root: pathlib.Path,
    manifest: Mapping[str, Any],
    current_source_commit: str,
) -> Dict[str, Any]:
    aggregate = require_object(load_json(path, "aggregate evidence index"), "aggregate evidence index")
    expect_keys(
        aggregate,
        ("schema_version", "suite", "task", "source_commit", "status", "indexes"),
        "aggregate evidence index",
    )
    if aggregate["schema_version"] != 1 or aggregate["suite"] != "v4-m1-06-aggregate":
        raise ObservationError("aggregate evidence index schema_version/suite does not match v4-m1-06-aggregate")
    task = require_string(aggregate["task"], "aggregate evidence index.task")
    if task != manifest["suite"].upper():
        raise ObservationError(
            f"aggregate evidence index.task must match fixture matrix suite: expected {manifest['suite'].upper()}"
        )
    source_commit = require_string(aggregate["source_commit"], "aggregate evidence index.source_commit")
    if not SOURCE_COMMIT.fullmatch(source_commit):
        raise ObservationError("aggregate evidence index.source_commit must be a 40-character lowercase commit")
    if source_commit != current_source_commit:
        raise ObservationError("aggregate evidence index source_commit does not match current checkout HEAD")
    status = require_string(aggregate["status"], "aggregate evidence index.status")
    if status not in ALLOWED_STATUS:
        raise ObservationError("aggregate evidence index.status must be pass, pending, or mismatch")

    aggregate_namespace = AGGREGATE_LAYOUT + (source_commit, "aggregate")
    aggregate_relative = relative_path(path, root, "aggregate evidence index")
    if pathlib.PurePosixPath(aggregate_relative).name != "index.json":
        raise ObservationError("aggregate evidence index must be named index.json")
    safe_relative_file(aggregate_relative, "aggregate evidence index", root, aggregate_namespace)

    entries = aggregate["indexes"]
    if not isinstance(entries, list) or len(entries) != len(SUPPORTED_TARGETS):
        raise ObservationError("aggregate evidence index must contain exactly both supported targets")
    normalized: List[Dict[str, Any]] = []
    targets: List[str] = []
    for index_number, value in enumerate(entries):
        label = f"aggregate evidence index.indexes[{index_number}]"
        entry = require_object(value, label)
        expect_keys(entry, ("target", "index"), label)
        target = require_string(entry["target"], f"{label}.target")
        if target not in SUPPORTED_TARGETS:
            raise ObservationError(f"{label}.target is unsupported: {target}")
        index_reference = require_string(entry["index"], f"{label}.index")
        target_namespace = AGGREGATE_LAYOUT + (source_commit, target)
        normalized_reference, index_path = safe_relative_file(
            index_reference, f"{label}.index", root, target_namespace
        )
        if pathlib.PurePosixPath(normalized_reference).name != "index.json":
            raise ObservationError(f"{label}.index must reference index.json")
        targets.append(target)
        normalized.append({"target": target, "index": normalized_reference, "path": index_path})
    if targets != SUPPORTED_TARGETS:
        raise ObservationError("aggregate evidence index targets must list both supported targets in order")
    return {
        "schema_version": 1,
        "suite": "v4-m1-06-aggregate",
        "task": task,
        "source_commit": source_commit,
        "status": status,
        "indexes": normalized,
    }


def aggregate(index: Mapping[str, Any], manifest: Mapping[str, Any], root: pathlib.Path) -> Dict[str, Any]:
    target_results = []
    for entry in index["indexes"]:
        target_index = load_target_index(entry["path"], root, manifest, index["source_commit"])
        if target_index["target"] != entry["target"]:
            raise ObservationError(
                f"target index target does not match aggregate entry: expected {entry['target']}"
            )
        result = audit_target(target_index, manifest)
        target_results.append(
            {
                "target": entry["target"],
                "index": entry["index"],
                "status": result["status"],
                "fixture_count": result["fixture_count"],
                "pending_boundaries": result["pending_boundaries"],
                "mismatches": result["mismatches"],
            }
        )

    if any(result["status"] == "mismatch" for result in target_results):
        status = "mismatch"
    elif any(result["status"] == "pending" for result in target_results):
        status = "pending"
    else:
        status = "pass"
    if index["status"] != status:
        raise ObservationError(
            f"aggregate evidence index.status does not match target results: index={index['status']} aggregate={status}"
        )
    return {
        "schema_version": 1,
        "suite": "v4-m1-06-aggregate",
        "task": index["task"],
        "source_commit": index["source_commit"],
        "status": status,
        "targets": target_results,
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
        index = load_aggregate(arguments.index, root, manifest, current_source_commit)
        result = aggregate(index, manifest, root)
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
