"""Schema fragments shared by the native MCP manifest input/output contract."""


IDENTIFIER_PATTERN = r"^[A-Za-z0-9_.-]+$"
U64_MAX = 18446744073709551615


def non_negative_integer_schema():
    return {"type": "integer", "minimum": 0, "maximum": U64_MAX}


def identifier_schema():
    return {"type": "string", "pattern": IDENTIFIER_PATTERN}


def id_schema():
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["namespace", "key"],
        "properties": {"namespace": identifier_schema(), "key": identifier_schema()},
    }


def subject_schema(kinds):
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["kind", "namespace", "key"],
        "properties": {
            "kind": {"type": "string", "enum": kinds},
            "namespace": identifier_schema(),
            "key": identifier_schema(),
        },
    }


def span_schema():
    position = non_negative_integer_schema()
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["start", "end"],
        "properties": {"start": position, "end": position.copy()},
    }


def sampling_schema():
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["cases", "seed", "generator"],
        "properties": {
            "cases": non_negative_integer_schema(),
            "seed": non_negative_integer_schema(),
            "generator": {"type": "string", "minLength": 1},
            "shrinks": {"type": "array", "items": non_negative_integer_schema()},
            "coverage": {
                "type": "object",
                "propertyNames": {"type": "string", "pattern": r"\S"},
                "additionalProperties": non_negative_integer_schema(),
            },
        },
    }


def execution_schema():
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["runner", "target", "source_commit", "artifact_digest", "sampling"],
        "properties": {
            "runner": {"type": "string", "minLength": 1},
            "target": {"type": "string", "minLength": 1},
            "source_commit": {"type": "string", "minLength": 1},
            "artifact_digest": {"type": "string", "minLength": 1},
            "sampling": sampling_schema(),
        },
    }


def node_schema():
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["kind", "namespace", "key", "text"],
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["intent", "claim", "assumption", "open-question"],
            },
            "namespace": identifier_schema(),
            "key": identifier_schema(),
            "text": {"type": "string", "minLength": 1},
            "span": span_schema(),
        },
    }


def evidence_schema():
    return {
        "type": "object",
        "additionalProperties": False,
        "required": [
            "namespace",
            "key",
            "method",
            "subject",
            "outcome",
            "execution",
            "provenance",
            "independence",
        ],
        "properties": {
            "namespace": identifier_schema(),
            "key": identifier_schema(),
            "method": {
                "type": "string",
                "enum": ["example", "case", "assert", "property", "production", "reference", "proof", "review"],
            },
            "subject": subject_schema(["intent", "claim", "contract"]),
            "outcome": {
                "type": "string",
                "enum": ["pass", "fail", "contradicted", "unknown", "stale"],
            },
            "execution": execution_schema(),
            "provenance": {
                "type": "object",
                "additionalProperties": False,
                "required": ["producer", "tool_version", "timestamp"],
                "properties": {
                    "producer": {"type": "string", "minLength": 1},
                    "tool_version": {"type": "string", "minLength": 1},
                    "timestamp": {"type": "string", "minLength": 1},
                },
            },
            "independence": {
                "type": "string",
                "enum": ["same-author", "independent-review", "external-observation"],
            },
        },
    }


def review_evidence_identity_schema():
    return {
        "type": "object",
        "additionalProperties": False,
        "required": [
            "subject_digest",
            "source_commit",
            "artifact_digest",
            "trust_store_digest",
            "lifecycle_digest",
            "now",
        ],
        "properties": {
            "subject_digest": {"type": "string", "minLength": 1},
            "source_commit": {"type": "string", "minLength": 1},
            "artifact_digest": {"type": "string", "minLength": 1},
            "trust_store_digest": {"type": ["string", "null"], "minLength": 1},
            "lifecycle_digest": {"type": ["string", "null"], "minLength": 1},
            "now": {"type": "string", "minLength": 1},
        },
    }


def edge_variant(relation, fields, properties):
    relation_schema = (
        {"enum": relation} if isinstance(relation, list) else {"const": relation}
    )
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["relation", *fields],
        "properties": {"relation": relation_schema, **properties},
    }


def edge_schema():
    return {
        "oneOf": [
            edge_variant("motivates", ["intent", "claim"], {"intent": id_schema(), "claim": id_schema()}),
            edge_variant("constrained-by", ["claim", "assumption"], {"claim": id_schema(), "assumption": id_schema()}),
            edge_variant("tested-by", ["claim", "contract"], {"claim": id_schema(), "contract": id_schema()}),
            edge_variant(
                ["supports", "contradicts"],
                ["observation", "claim"],
                {"observation": id_schema(), "claim": id_schema()},
            ),
            edge_variant(
                "evaluates",
                ["review", "subject"],
                {"review": id_schema(), "subject": subject_schema(["intent", "claim", "evidence"])},
            ),
            edge_variant(
                "invalidates",
                ["change", "subject"],
                {"change": id_schema(), "subject": subject_schema(["evidence", "review"])},
            ),
        ]
    }


def review_registry_schema():
    return {
        "type": "array",
        "items": {
            "type": "object",
            "additionalProperties": False,
            "required": ["namespace", "key", "provenance_digest", "visibility"],
            "properties": {
                "namespace": identifier_schema(),
                "key": identifier_schema(),
                "provenance_digest": {"type": "string", "minLength": 1},
                "visibility": {"type": "string", "enum": ["public", "redacted"]},
                "verification_state": {
                    "type": "string",
                    "enum": ["verified", "unverified", "stale", "revoked"],
                },
            },
        },
    }
