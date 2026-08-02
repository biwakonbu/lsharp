#!/usr/bin/env python3
"""Validate the native handoff for an externally verified review signature."""

from __future__ import annotations

import importlib.util
import json
import pathlib
import re
import stat
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


def validate_value(value: object) -> dict[str, object]:
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


def validate(path: pathlib.Path) -> dict[str, object]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"verification receipt JSON is invalid: {error}") from error
    return validate_value(value)


def validate_trust_store_identity(
    value: dict[str, object], trust_store_path: pathlib.Path
) -> tuple[str, str, str]:
    try:
        file_stat = trust_store_path.lstat()
    except OSError as error:
        raise ValueError(f"trust store cannot be read: {trust_store_path}: {error}") from error
    if stat.S_ISLNK(file_stat.st_mode) or not stat.S_ISREG(file_stat.st_mode):
        raise ValueError(
            f"trust store must be a regular non-symlink file: {trust_store_path}"
        )
    validator_path = pathlib.Path(__file__).with_name("verify-native-review-trust-store.py")
    spec = importlib.util.spec_from_file_location(
        "lsharp_native_review_trust_store", validator_path
    )
    if spec is None or spec.loader is None:
        raise ValueError("trust store validator is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    try:
        active = module.validate(trust_store_path)
    except (OSError, ValueError) as error:
        raise ValueError(f"trust store validation failed: {error}") from error
    identity = (value["provider"], value["algorithm"], value["key_id"])
    matches = [candidate for candidate in active if candidate == identity]
    if len(matches) != 1:
        raise ValueError(
            "active trust identity mismatch: "
            f"receipt={value['provider']}/{value['algorithm']}={value['key_id']}"
        )
    return matches[0]


def main(argv: list[str]) -> int:
    trust_store_path = None
    if len(argv) == 2:
        receipt_path = pathlib.Path(argv[1])
    elif len(argv) == 4 and argv[2] == "--trust-store":
        receipt_path = pathlib.Path(argv[1])
        trust_store_path = pathlib.Path(argv[3])
    else:
        return fail(
            f"usage: {argv[0]} RECEIPT_JSON [--trust-store TRUST_STORE_JSON]"
        )
    try:
        value = validate(receipt_path)
        active_identity = None
        if trust_store_path is not None:
            active_identity = validate_trust_store_identity(value, trust_store_path)
    except ValueError as error:
        return fail(str(error))
    print(f"verification receipt: {value['review_id']}={value['state']}")
    print(f"canonical receipt bytes: {canonical_bytes(value).hex()}")
    if active_identity is not None:
        provider, algorithm, key_id = active_identity
        print(f"active trust identity: {provider}/{algorithm}={key_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
