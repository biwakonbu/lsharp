"""Strict JSON decoding shared by native MCP relay modules."""

import json


def strict_json_loads(content):
    """Decode JSON while rejecting duplicate keys and non-standard constants."""

    def reject_duplicate_keys(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"duplicate JSON object key: {key}")
            value[key] = item
        return value

    def reject_nonstandard_constant(value):
        raise ValueError(f"non-standard JSON constant: {value}")

    return json.loads(
        content,
        object_pairs_hook=reject_duplicate_keys,
        parse_constant=reject_nonstandard_constant,
    )
