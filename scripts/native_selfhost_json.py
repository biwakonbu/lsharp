"""Strict JSON decoding shared by native MCP relay modules."""

import json


def strict_json_loads(content):
    """Decode JSON while rejecting duplicate object keys at every nesting level."""

    def reject_duplicate_keys(pairs):
        value = {}
        for key, item in pairs:
            if key in value:
                raise ValueError(f"duplicate JSON object key: {key}")
            value[key] = item
        return value

    return json.loads(content, object_pairs_hook=reject_duplicate_keys)
