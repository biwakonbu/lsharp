#!/usr/bin/env python3
"""Validate the native provider trust-store rotation boundary.

The native provider adapter does not verify Ed25519 signatures.  This focused
preflight only validates the explicit trust-store snapshot shape and makes
active-key selection deterministic before a future semantic verifier consumes
it.
"""

from __future__ import annotations

import base64
import binascii
import json
import pathlib
import re
import sys


BASE64URL = re.compile(r"^[A-Za-z0-9_-]+$")


def fail(message: str) -> int:
    print(message, file=sys.stderr)
    return 1


def decode_public_key(value: object, index: int) -> bytes:
    if not isinstance(value, str) or not value:
        raise ValueError(f"trust key {index} public_key must be a non-empty string")
    if "=" in value or len(value) % 4 == 1 or not BASE64URL.fullmatch(value):
        raise ValueError(f"trust key {index} public_key must be canonical base64url")
    try:
        decoded = base64.urlsafe_b64decode(value + "=" * (-len(value) % 4))
    except (binascii.Error, ValueError) as error:
        raise ValueError(f"trust key {index} public_key is not base64url: {error}") from error
    if base64.urlsafe_b64encode(decoded).decode().rstrip("=") != value:
        raise ValueError(f"trust key {index} public_key must be canonical base64url")
    if len(decoded) != 32:
        raise ValueError(f"trust key {index} public_key must decode to 32 bytes")
    return decoded


def validate(path: pathlib.Path) -> list[tuple[str, str, str]]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError(f"trust store JSON is invalid: {error}") from error
    if not isinstance(value, dict) or set(value) != {"keys"}:
        raise ValueError("trust store root must contain only keys")
    keys = value["keys"]
    if not isinstance(keys, list):
        raise ValueError("trust store keys must be an array")

    identities: set[tuple[str, str, str]] = set()
    active: dict[tuple[str, str], str] = {}
    selected: list[tuple[str, str, str]] = []
    for index, key in enumerate(keys):
        if not isinstance(key, dict):
            raise ValueError(f"trust key {index} must be an object")
        allowed = {"provider", "key_id", "algorithm", "public_key", "active"}
        unknown = sorted(set(key).difference(allowed))
        if unknown:
            raise ValueError(f"trust key {index} has unknown field: {unknown[0]}")
        missing = sorted({"provider", "key_id", "algorithm", "public_key"}.difference(key))
        if missing:
            raise ValueError(f"trust key {index} missing field: {missing[0]}")
        provider = key["provider"]
        key_id = key["key_id"]
        algorithm = key["algorithm"]
        if not isinstance(provider, str) or not provider.strip():
            raise ValueError(f"trust key {index} provider must be non-empty")
        if not isinstance(key_id, str) or not key_id.strip():
            raise ValueError(f"trust key {index} key_id must be non-empty")
        if algorithm != "ed25519":
            raise ValueError(f"trust key {index} algorithm must be ed25519")
        decode_public_key(key["public_key"], index)
        is_active = key.get("active", True)
        if not isinstance(is_active, bool):
            raise ValueError(f"trust key {index} active must be boolean")
        identity = (provider, key_id, algorithm)
        if identity in identities:
            raise ValueError(
                f"duplicate trust key: provider={provider}, key_id={key_id}, algorithm={algorithm}"
            )
        identities.add(identity)
        if is_active:
            selection = (provider, algorithm)
            previous = active.get(selection)
            if previous is not None:
                raise ValueError(
                    "multiple active keys: "
                    f"provider={provider}, algorithm={algorithm}, "
                    f"existing_key_id={previous}, key_id={key_id}"
                )
            active[selection] = key_id
            selected.append((provider, algorithm, key_id))
    return sorted(selected)


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        return fail(f"usage: {argv[0]} TRUST_STORE_JSON")
    try:
        selected = validate(pathlib.Path(argv[1]))
    except ValueError as error:
        return fail(str(error))
    for provider, algorithm, key_id in selected:
        print(f"active key: {provider}/{algorithm}={key_id}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
