#!/usr/bin/env python3

"""Validate the explicit evidence identity carried by a native release.

The verifier is deliberately offline.  A provider adapter is responsible for
supplying explicit keyset/lifecycle snapshot bytes or their digests; this
command never fills them from the environment, network, or current checkout.
"""

import argparse
import hashlib
import json
import os
import pathlib
import re
import stat
import sys

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
REVIEW_LIFECYCLE_STATES = frozenset(
    ("proposed", "active", "superseded", "revoked")
)
INITIAL_REVIEW_LIFECYCLE_STATES = frozenset(("proposed", "active"))
TERMINAL_REVIEW_LIFECYCLE_STATES = frozenset(("superseded", "revoked"))


class IdentityError(ValueError):
    """An invalid or conflicting identity input."""


class UnverifiedIdentity(IdentityError):
    """An identity that cannot be treated as verified without provider input."""


def digest_file(path, field):
    try:
        file_stat = path.lstat()
        if stat.S_ISLNK(file_stat.st_mode) or not stat.S_ISREG(file_stat.st_mode):
            raise IdentityError(f"{field} must be a regular non-symlink file: {path}")
        payload = path.read_bytes()
    except OSError as error:
        raise IdentityError(f"{field} cannot be read: {path}: {error}") from error
    if not payload and field in ("trust store", "review lifecycle"):
        raise IdentityError(f"{field} must be non-empty: {path}")
    return "sha256:" + hashlib.sha256(payload).hexdigest()


def load_json(path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise IdentityError(f"identity JSON is invalid: {path}: {error}") from error


def load_identity(path, manifest):
    value = load_json(path)
    if manifest:
        if not isinstance(value, dict):
            raise IdentityError(
                "manifest must be a JSON object containing review_evidence_identity"
            )
        value = value.get("review_evidence_identity")
        if value is None:
            raise IdentityError("manifest review_evidence_identity is required")
    if not isinstance(value, dict):
        raise IdentityError("review_evidence_identity must be an object")
    return value


def validate_review_lifecycle_snapshot(path):
    try:
        payload = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as error:
        raise IdentityError(f"review lifecycle snapshot is not valid UTF-8: {path}: {error}") from error

    try:
        parsed = json.loads(payload)
        records = parsed if isinstance(parsed, list) else [parsed]
    except json.JSONDecodeError:
        records = []
        for line_number, line in enumerate(payload.splitlines(), start=1):
            if not line.strip():
                continue
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError as error:
                raise IdentityError(
                    f"review lifecycle snapshot record is invalid JSON: {path}:{line_number}: {error}"
                ) from error

    if not records:
        raise IdentityError(f"review lifecycle snapshot must contain at least one record: {path}")
    seen_sequences = set()
    last_sequences = {}
    last_states = {}
    for record in records:
        if not isinstance(record, dict):
            raise IdentityError(f"review lifecycle snapshot records must be JSON objects: {path}")
        state = record.get("state")
        if state not in REVIEW_LIFECYCLE_STATES:
            allowed = ", ".join(sorted(REVIEW_LIFECYCLE_STATES))
            raise IdentityError(
                f"review lifecycle state must be one of {allowed}: {path}"
            )
        if "effective_at" in record and not is_valid_utc_timestamp(record["effective_at"]):
            raise IdentityError(
                f"review lifecycle effective_at must be a strict UTC timestamp: {path}"
            )
        sequence = record.get("sequence")
        review_id = record.get("review_id")
        if isinstance(sequence, int) and not isinstance(sequence, bool) and isinstance(review_id, str):
            sequence_key = (review_id, sequence)
            if sequence_key in seen_sequences:
                raise IdentityError(
                    f"duplicate review lifecycle sequence: {path}: "
                    f"review_id={review_id!r} sequence={sequence}"
                )
            if review_id not in last_states and state not in INITIAL_REVIEW_LIFECYCLE_STATES:
                allowed = ", ".join(sorted(INITIAL_REVIEW_LIFECYCLE_STATES))
                raise IdentityError(
                    f"review lifecycle initial state must be one of {allowed}: {path}: "
                    f"review_id={review_id!r} state={state}"
                )
            previous_state = last_states.get(review_id)
            if previous_state == "proposed" and state in TERMINAL_REVIEW_LIFECYCLE_STATES:
                raise IdentityError(
                    f"review lifecycle terminal transition requires active: {path}: "
                    f"review_id={review_id!r} previous={previous_state} current={state}"
                )
            if previous_state in TERMINAL_REVIEW_LIFECYCLE_STATES:
                raise IdentityError(
                    f"review lifecycle terminal state reactivation: {path}: "
                    f"review_id={review_id!r} previous={previous_state} current={state}"
                )
            previous_sequence = last_sequences.get(review_id)
            if previous_sequence is not None and sequence < previous_sequence:
                raise IdentityError(
                    f"review lifecycle sequence rollback: {path}: "
                    f"review_id={review_id!r} previous={previous_sequence} current={sequence}"
                )
            seen_sequences.add(sequence_key)
            last_sequences[review_id] = sequence
            last_states[review_id] = state

    return records


def validate_digest(value, field, nullable):
    if value is None and nullable:
        return
    if not isinstance(value, str) or not value:
        raise IdentityError(f"{field} must be a non-empty sha256 digest")
    if not SHA256_PATTERN.fullmatch(value):
        raise IdentityError(f"{field} must use sha256:<64 lowercase hex>")


def validate_identity(
    identity,
    expected_source_commit=None,
    artifact=None,
    require_provider=False,
    trust_store=None,
    review_lifecycle=None,
):
    actual_keys = tuple(identity)
    if actual_keys != IDENTITY_KEYS:
        if set(actual_keys) != set(IDENTITY_KEYS):
            raise IdentityError(
                "review_evidence_identity fields must be exactly: "
                + ", ".join(IDENTITY_KEYS)
            )
        raise IdentityError("review_evidence_identity field order is not canonical")

    for field in ("subject_digest", "source_commit", "artifact_digest", "now"):
        value = identity[field]
        if not isinstance(value, str) or not value:
            raise IdentityError(f"{field} must be a non-empty string")

    source_commit = identity["source_commit"]
    if not SOURCE_COMMIT_PATTERN.fullmatch(source_commit):
        raise IdentityError("source_commit must be a 40-character lowercase hexadecimal commit")
    if expected_source_commit is not None and source_commit != expected_source_commit:
        raise IdentityError(
            "source_commit mismatch: "
            f"expected={expected_source_commit} actual={source_commit}"
        )

    if not SHA256_PATTERN.fullmatch(identity["subject_digest"]):
        raise IdentityError("subject_digest must use sha256:<64 lowercase hex>")
    validate_digest(identity["artifact_digest"], "artifact_digest", nullable=False)
    validate_digest(identity["trust_store_digest"], "trust_store_digest", nullable=True)
    validate_digest(identity["lifecycle_digest"], "lifecycle_digest", nullable=True)
    if not is_valid_utc_timestamp(identity["now"]):
        raise IdentityError("now must be a strict UTC timestamp ending in Z")

    has_trust_store = trust_store is not None
    has_lifecycle = review_lifecycle is not None
    if has_trust_store != has_lifecycle:
        raise IdentityError(
            "--trust-store and --review-lifecycle must be supplied together"
        )
    if has_trust_store and os.path.abspath(trust_store) == os.path.abspath(review_lifecycle):
        raise IdentityError(
            "--trust-store and --review-lifecycle must be different files"
        )
    if has_trust_store:
        expected_trust_store_digest = digest_file(trust_store, "trust store")
        if identity["trust_store_digest"] != expected_trust_store_digest:
            raise IdentityError(
                "trust_store_digest mismatch: "
                f"expected={expected_trust_store_digest} actual={identity['trust_store_digest']}"
            )
        expected_lifecycle_digest = digest_file(review_lifecycle, "review lifecycle")
        validate_review_lifecycle_snapshot(review_lifecycle)
        if identity["lifecycle_digest"] != expected_lifecycle_digest:
            raise IdentityError(
                "lifecycle_digest mismatch: "
                f"expected={expected_lifecycle_digest} actual={identity['lifecycle_digest']}"
            )

    if require_provider and (
        identity["trust_store_digest"] is None or identity["lifecycle_digest"] is None
    ):
        raise UnverifiedIdentity(
            "provider keyset/lifecycle digest is missing; release identity is unverified"
        )

    if artifact is not None:
        try:
            artifact_bytes = artifact.read_bytes()
        except OSError as error:
            raise IdentityError(f"release artifact cannot be read: {artifact}: {error}") from error
        actual_artifact_digest = "sha256:" + hashlib.sha256(artifact_bytes).hexdigest()
        if identity["artifact_digest"] != actual_artifact_digest:
            raise IdentityError(
                "artifact_digest mismatch: "
                f"expected={actual_artifact_digest} actual={identity['artifact_digest']}"
            )

    return identity


def parse_arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument("--identity", type=pathlib.Path, help="JSON object containing the identity")
    source.add_argument(
        "--manifest",
        type=pathlib.Path,
        help="release manifest containing review_evidence_identity",
    )
    parser.add_argument("--expected-identity", type=pathlib.Path)
    parser.add_argument("--artifact", type=pathlib.Path)
    parser.add_argument("--source-commit")
    parser.add_argument(
        "--trust-store",
        type=pathlib.Path,
        help="optional raw trust-store snapshot to digest and compare",
    )
    parser.add_argument(
        "--review-lifecycle",
        type=pathlib.Path,
        help="optional raw review-lifecycle snapshot to digest and compare",
    )
    parser.add_argument("--require-provider-input", action="store_true")
    return parser.parse_args()


def main():
    arguments = parse_arguments()
    path = arguments.identity or arguments.manifest
    try:
        identity = load_identity(path, arguments.manifest is not None)
        validate_identity(
            identity,
            expected_source_commit=arguments.source_commit,
            artifact=arguments.artifact,
            require_provider=arguments.require_provider_input,
            trust_store=arguments.trust_store,
            review_lifecycle=arguments.review_lifecycle,
        )
        if arguments.expected_identity is not None:
            expected = load_identity(arguments.expected_identity, False)
            validate_identity(expected)
            if identity != expected:
                raise IdentityError("review_evidence_identity conflicts with the expected identity")
    except UnverifiedIdentity as error:
        print(f"unverified: {error}", file=sys.stderr)
        return 2
    except IdentityError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1

    print(json.dumps(identity, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
