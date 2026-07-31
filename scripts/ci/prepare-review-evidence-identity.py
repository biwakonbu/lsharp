#!/usr/bin/env python3

"""Build a canonical native release evidence identity from explicit inputs.

This is an offline input boundary.  It hashes only the artifact and the
provider snapshots named on the command line; it never reads the environment,
network, current checkout, or an implicit trust root.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import sys
import tempfile

from review_identity_timestamp import is_valid_utc_timestamp


IDENTITY_KEYS = (
    "subject_digest",
    "source_commit",
    "artifact_digest",
    "trust_store_digest",
    "lifecycle_digest",
    "now",
)
SHA256_PATTERN = re.compile(r"^sha256:[0-9a-f]{64}$")
SOURCE_COMMIT_PATTERN = re.compile(r"^[0-9a-f]{40}$")


class IdentityInputError(ValueError):
    """An explicit identity input cannot be used for a release."""


def validate_digest(value: str, field: str) -> None:
    if not SHA256_PATTERN.fullmatch(value):
        raise IdentityInputError(f"{field} must use sha256:<64 lowercase hex>")


def digest_file(path: pathlib.Path, field: str) -> str:
    try:
        payload = path.read_bytes()
    except OSError as error:
        raise IdentityInputError(f"{field} cannot be read: {path}: {error}") from error
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def build_identity(arguments: argparse.Namespace) -> dict[str, str | None]:
    validate_digest(arguments.subject_digest, "subject_digest")
    if not SOURCE_COMMIT_PATTERN.fullmatch(arguments.source_commit):
        raise IdentityInputError(
            "source_commit must be a 40-character lowercase hexadecimal commit"
        )
    if not is_valid_utc_timestamp(arguments.now):
        raise IdentityInputError("now must be a strict UTC timestamp ending in Z")

    has_trust_store = arguments.trust_store is not None
    has_lifecycle = arguments.review_lifecycle is not None
    if has_trust_store != has_lifecycle:
        raise IdentityInputError(
            "--trust-store and --review-lifecycle must be supplied together"
        )

    artifact_digest = digest_file(arguments.artifact, "artifact")
    trust_store_digest = (
        digest_file(arguments.trust_store, "trust store") if has_trust_store else None
    )
    lifecycle_digest = (
        digest_file(arguments.review_lifecycle, "review lifecycle")
        if has_lifecycle
        else None
    )
    return {
        "subject_digest": arguments.subject_digest,
        "source_commit": arguments.source_commit,
        "artifact_digest": artifact_digest,
        "trust_store_digest": trust_store_digest,
        "lifecycle_digest": lifecycle_digest,
        "now": arguments.now,
    }


def canonical_json(identity: dict[str, str | None]) -> str:
    if tuple(identity) != IDENTITY_KEYS:
        raise IdentityInputError("identity field order is not canonical")
    return json.dumps(identity, ensure_ascii=False, separators=(",", ":")) + "\n"


def write_atomic(path: pathlib.Path, payload: str) -> None:
    parent = path.parent
    if not parent.is_dir():
        raise IdentityInputError(f"output parent directory does not exist: {parent}")
    temporary_path: pathlib.Path | None = None
    try:
        with tempfile.NamedTemporaryFile(
            mode="w",
            encoding="utf-8",
            dir=parent,
            prefix=f".{path.name}.",
            suffix=".tmp",
            delete=False,
        ) as temporary:
            temporary_path = pathlib.Path(temporary.name)
            temporary.write(payload)
            temporary.flush()
            os.fsync(temporary.fileno())
        os.replace(temporary_path, path)
        temporary_path = None
    except OSError as error:
        raise IdentityInputError(f"identity output cannot be written: {path}: {error}") from error
    finally:
        if temporary_path is not None:
            try:
                temporary_path.unlink()
            except OSError:
                pass


def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--subject-digest", required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--artifact", required=True, type=pathlib.Path)
    parser.add_argument("--trust-store", type=pathlib.Path)
    parser.add_argument("--review-lifecycle", type=pathlib.Path)
    parser.add_argument("--now", required=True)
    parser.add_argument("--output", type=pathlib.Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_arguments()
    try:
        identity = build_identity(arguments)
        payload = canonical_json(identity)
        if arguments.output is not None:
            write_atomic(arguments.output, payload)
    except IdentityInputError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    print(payload, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
