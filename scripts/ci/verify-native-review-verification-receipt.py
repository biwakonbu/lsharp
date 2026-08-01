#!/usr/bin/env python3
"""Validate the native handoff for an externally verified review signature."""

from __future__ import annotations

import json
import pathlib
import re
import sys
from datetime import datetime


DIGEST = re.compile(r"^sha256:[0-9a-f]{64}$")
REVIEW_ID = re.compile(r"^review:[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
TIMESTAMP = re.compile(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$")
DOMAIN = b"lsharp.review-verification-receipt.v1\0"


def fail(message: str) -> int:
    print(message, file=sys.stderr)
    return 1


def append_field(output: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    output.extend(len(encoded).to_bytes(8, "big"))
    output.extend(encoded)


def canonical_bytes(value: dict[str, object]) -> bytes:
    output = bytearray(DOMAIN)
    for name in (
        "review_id",
        "state",
        "provider",
        "key_id",
        "algorithm",
        "attestation_digest",
        "trust_store_digest",
        "verification_now",
    ):
        append_field(output, value[name])
    return bytes(output)


def validate(path: pathlib.Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"verification receipt JSON is invalid: {error}") from error
    required = {
        "review_id",
        "state",
        "provider",
        "key_id",
        "algorithm",
        "attestation_digest",
        "trust_store_digest",
        "verification_now",
    }
    if not isinstance(value, dict):
        raise ValueError("verification receipt root must be an object")
    unknown = sorted(set(value).difference(required))
    if unknown:
        raise ValueError(f"verification receipt has unknown field: {unknown[0]}")
    missing = sorted(required.difference(value))
    if missing:
        raise ValueError(f"verification receipt is missing field: {missing[0]}")
    if not isinstance(value["review_id"], str) or not REVIEW_ID.fullmatch(value["review_id"]):
        raise ValueError("verification receipt review_id has invalid format")
    if value["state"] != "verified":
        raise ValueError("verification receipt state must be verified")
    if value["algorithm"] != "ed25519":
        raise ValueError("verification receipt algorithm must be ed25519")
    for name in ("provider", "key_id"):
        if not isinstance(value[name], str) or not value[name].strip():
            raise ValueError(f"verification receipt {name} must be non-empty")
    for name in ("attestation_digest", "trust_store_digest"):
        if not isinstance(value[name], str) or not DIGEST.fullmatch(value[name]):
            raise ValueError(f"verification receipt {name} must be a sha256 digest")
    verification_now = value["verification_now"]
    if not isinstance(verification_now, str) or not TIMESTAMP.fullmatch(verification_now):
        raise ValueError("verification receipt verification_now must be canonical UTC")
    try:
        datetime.strptime(verification_now, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError as error:
        raise ValueError("verification receipt verification_now must be a valid UTC date") from error
    return value


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        return fail(f"usage: {argv[0]} RECEIPT_JSON")
    try:
        value = validate(pathlib.Path(argv[1]))
    except ValueError as error:
        return fail(str(error))
    print(f"verification receipt: {value['review_id']}={value['state']}")
    print(f"canonical receipt bytes: {canonical_bytes(value).hex()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
