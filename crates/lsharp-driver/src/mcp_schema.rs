fn tool(name: &str, description: &str) -> McpTool {
    McpTool {
        name: name.to_string(),
        description: description.to_string(),
    }
}

fn tool_input_schema(name: &str) -> Value {
    match name {
        "lsharp_check" => json_schema(&["source"], &["file"]),
        "lsharp_validate" => validate_input_schema(),
        "lsharp_hover" | "lsharp_completion" | "lsharp_definition" | "lsharp_references" => {
            json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "source": { "type": "string" },
                    "file": { "type": "string" },
                    "line": { "type": "integer", "minimum": 0 },
                    "character": { "type": "integer", "minimum": 0 },
                    "col": { "type": "integer", "minimum": 0 }
                },
                "required": ["line", "character"]
            })
        }
        "lsharp_format" | "lsharp_compile_run" => json_schema(&["source"], &["file"]),
        "lsharp_errors" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "error_code": { "type": "string", "minLength": 1 }
            },
            "required": ["error_code"]
        }),
        "lsharp_install" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "name": { "type": "string", "minLength": 1 },
                "project_dir": { "type": "string", "minLength": 1 }
            },
            "oneOf": [{ "required": ["name"] }]
        }),
        "lsharp_package_api" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "name": { "type": "string", "minLength": 1 },
                "project_dir": { "type": "string", "minLength": 1 }
            },
            "oneOf": [{ "required": ["name"] }]
        }),
        "lsharp_stdlib_api" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "module": { "type": "string", "minLength": 1 }
            },
            "oneOf": [{ "required": [] }]
        }),
        "lsharp_project_context" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "project_dir": { "type": "string", "minLength": 1 }
            },
            "oneOf": [{ "required": [] }]
        }),
        "lsharp_search" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "project_dir": { "type": "string", "minLength": 1 },
                "query": { "type": "string" }
            }
        }),
        _ => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        }),
    }
}

fn tool_output_schema(name: &str) -> Value {
    match name {
        "lsharp_check" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "required": ["ok", "diagnostics", "migrationDiagnostics"],
            "properties": {
                "ok": { "type": "boolean" },
                "diagnostics": { "type": "array" },
                "migrationDiagnostics": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["code", "owner", "selectedSemantics", "disposition", "range"],
                        "properties": {
                            "code": {
                                "type": "string",
                                "enum": ["LS2001", "LS2002", "LS2003"]
                            },
                            "owner": { "type": "string" },
                            "selectedSemantics": {
                                "type": "string",
                                "enum": [
                                    "legacy-example-truthiness",
                                    "legacy-invariant-deterministic-smoke"
                                ]
                            },
                            "disposition": {
                                "type": "string",
                                "enum": [
                                    "docs-only-example",
                                    "assertion",
                                    "property-postcondition",
                                    "manual-review"
                                ]
                            },
                            "range": {
                                "type": "object",
                                "required": ["start", "end"],
                                "properties": {
                                    "start": { "$ref": "#/$defs/position" },
                                    "end": { "$ref": "#/$defs/position" }
                                }
                            },
                            "message": { "type": "string" }
                        }
                    }
                }
            },
            "$defs": {
                "position": {
                    "type": "object",
                    "required": ["line", "character"],
                    "properties": {
                        "line": { "type": "integer", "minimum": 0 },
                        "character": { "type": "integer", "minimum": 0 }
                    }
                }
            }
        }),
        "lsharp_validate" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "required": [
                "status",
                "trace_gaps",
                "open_questions",
                "independent_reviews",
                "contradicting_observations",
                "stale_reviews",
                "stale_evidence"
            ],
            "properties": {
                "status": {
                    "type": "string",
                    "enum": ["pass", "fail", "unknown"]
                },
                "trace_gaps": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["code", "subject_id"],
                        "properties": {
                            "code": {
                                "enum": [
                                    "trace-gap.intent-without-claim",
                                    "trace-gap.claim-without-test"
                                ]
                            },
                            "subject_id": { "type": "string", "minLength": 1 }
                        }
                    }
                },
                "open_questions": { "type": "integer", "minimum": 0, "maximum": u64::MAX },
                "independent_reviews": { "type": "integer", "minimum": 0, "maximum": u64::MAX },
                "contradicting_observations": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": u64::MAX
                },
                "stale_reviews": { "type": "integer", "minimum": 0, "maximum": u64::MAX },
                "stale_evidence": { "type": "integer", "minimum": 0, "maximum": u64::MAX },
                "review_evidence_identity": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": [
                        "subject_digest",
                        "source_commit",
                        "artifact_digest",
                        "trust_store_digest",
                        "lifecycle_digest",
                        "now"
                    ],
                    "properties": {
                        "subject_digest": { "type": "string", "minLength": 1 },
                        "source_commit": { "type": "string", "minLength": 1 },
                        "artifact_digest": { "type": "string", "minLength": 1 },
                        "trust_store_digest": {
                            "type": ["string", "null"],
                            "minLength": 1
                        },
                        "lifecycle_digest": {
                            "type": ["string", "null"],
                            "minLength": 1
                        },
                        "now": { "type": "string", "minLength": 1 }
                    }
                },
                "review_verifications": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["review_id", "state"],
                        "properties": {
                            "review_id": {
                                "type": "string",
                                "pattern": "^review:[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$"
                            },
                            "state": {
                                "type": "string",
                                "enum": ["verified", "unverified", "stale", "revoked"]
                            }
                        }
                    }
                },
                "manifest": intent_graph_manifest_schema()
            }
        }),
        "lsharp_project_context" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "required": ["project", "dependencies", "installedPackages"],
            "properties": {
                "project": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["name", "version", "description", "exports"],
                    "properties": {
                        "name": { "type": "string" },
                        "version": { "type": "string" },
                        "description": { "type": "string" },
                        "exports": { "type": "array", "items": { "type": "string" } }
                    }
                },
                "dependencies": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["name", "source"],
                        "properties": {
                            "name": { "type": "string", "minLength": 1 },
                            "source": { "type": "string", "enum": ["registry", "path", "git"] },
                            "version": { "type": "string" },
                            "path": { "type": "string", "minLength": 1 },
                            "git": { "type": "string", "minLength": 1 },
                            "branch": { "type": ["string", "null"] },
                            "tag": { "type": ["string", "null"] }
                        }
                    }
                },
                "installedPackages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["name", "version", "path"],
                        "properties": {
                            "name": { "type": "string", "minLength": 1 },
                            "version": { "type": "string" },
                            "path": { "type": "string", "minLength": 1 }
                        }
                    }
                }
            }
        }),
        "lsharp_errors" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "required": ["code", "name", "description", "fix", "doc"],
            "properties": {
                "code": { "type": "string", "minLength": 1 },
                "legacy_code": { "type": ["string", "null"], "minLength": 1 },
                "name": { "type": "string", "minLength": 1 },
                "description": { "type": "string", "minLength": 1 },
                "detail": { "type": "string", "minLength": 1 },
                "fix": { "type": "string", "minLength": 1 },
                "doc": {
                    "type": "string",
                    "const": error_codes::ERROR_REFERENCE_DOC
                }
            }
        }),
        "lsharp_search" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "additionalProperties": false,
            "required": ["packages"],
            "properties": {
                "packages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "required": ["name", "version", "path"],
                        "properties": {
                            "name": { "type": "string", "minLength": 1 },
                            "version": { "type": "string" },
                            "path": { "type": "string", "minLength": 1 }
                        }
                    }
                }
            }
        }),
        "lsharp_package_api" | "lsharp_stdlib_api" => package_api_output_schema(),
        _ => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        }),
    }
}

fn package_api_output_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object", "additionalProperties": false,
        "required": ["package", "version", "modules"],
        "properties": {
            "package": { "type": "string", "minLength": 1 },
            "version": { "type": "string", "minLength": 1 },
            "modules": { "type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["name", "doc", "functions", "types"],
                "properties": {
                    "name": { "type": "string", "minLength": 1 },
                    "doc": { "type": ["string", "null"] },
                    "functions": { "type": "array", "items": {
                        "type": "object", "additionalProperties": false,
                        "required": ["name", "signature", "params", "returns", "doc", "example"],
                        "properties": {
                            "name": { "type": "string", "minLength": 1 },
                            "signature": { "type": "string", "minLength": 1 },
                            "params": { "type": "array", "items": {
                                "type": "object", "additionalProperties": false,
                                "required": ["name", "type", "doc"],
                                "properties": {
                                    "name": { "type": "string", "minLength": 1 },
                                    "type": { "type": "string", "minLength": 1 },
                                    "doc": { "type": ["string", "null"] }
                                }
                            }},
                            "returns": { "type": "object", "additionalProperties": false,
                                "required": ["type", "doc"], "properties": {
                                    "type": { "type": "string", "minLength": 1 },
                                    "doc": { "type": ["string", "null"] }
                                }
                            },
                            "doc": { "type": ["string", "null"] },
                            "example": { "type": ["string", "null"] }
                        }
                    }},
                    "types": { "type": "array", "items": {
                        "type": "object", "additionalProperties": false,
                        "required": ["name", "kind"], "properties": {
                            "name": { "type": "string", "minLength": 1 },
                            "kind": { "type": "string", "minLength": 1 }
                        }
                    }}
                }
            }}
        }
    })
}

fn json_schema(required_primary: &[&str], required_secondary: &[&str]) -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "source": { "type": "string" },
            "file": { "type": "string" }
        },
        "anyOf": [
            { "required": required_primary },
            { "required": required_secondary }
        ]
    })
}

fn validate_input_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "source": { "type": "string" },
            "file": { "type": "string", "minLength": 1 },
            "manifest": {
                "oneOf": [
                    intent_graph_manifest_schema(),
                    { "type": "string", "minLength": 1 }
                ]
            },
            "manifest_file": { "type": "string", "minLength": 1 },
            "include_manifest": { "type": "boolean" },
            "trust_store": { "type": "string", "minLength": 1 },
            "review_lifecycle": { "type": "string", "minLength": 1 },
            "review_subject_digest": { "type": "string", "minLength": 1 },
            "review_source_commit": { "type": "string", "minLength": 1 },
            "review_artifact_digest": { "type": "string", "minLength": 1 },
            "review_now": {
                "type": "string",
                "minLength": 1,
                "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$"
            }
        },
        "dependentRequired": {
            "review_subject_digest": ["review_source_commit", "review_now"],
            "review_source_commit": ["review_subject_digest", "review_now"],
            "review_artifact_digest": [
                "review_subject_digest",
                "review_source_commit",
                "review_now"
            ],
            "review_now": ["review_subject_digest", "review_source_commit"]
        },
        "oneOf": [
            { "required": ["source"] },
            { "required": ["file"] },
            { "required": ["manifest"] },
            { "required": ["manifest_file"] }
        ]
    })
}

fn intent_graph_manifest_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["schema_version", "nodes", "evidence", "edges"],
        "properties": {
            "schema_version": { "type": "integer", "const": 1 },
            "nodes": {
                "type": "array",
                "items": node_schema()
            },
            "reviews": review_registry_schema(),
            "review_evidence_identity": review_evidence_identity_schema(),
            "evidence": {
                "type": "array",
                "items": evidence_schema()
            },
            "edges": {
                "type": "array",
                "items": edge_schema()
            }
        }
    })
}

fn review_evidence_identity_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "subject_digest",
            "source_commit",
            "artifact_digest",
            "trust_store_digest",
            "lifecycle_digest",
            "now"
        ],
        "properties": {
            "subject_digest": { "type": "string", "minLength": 1 },
            "source_commit": { "type": "string", "minLength": 1 },
            "artifact_digest": { "type": "string", "minLength": 1 },
            "trust_store_digest": { "type": ["string", "null"], "minLength": 1 },
            "lifecycle_digest": { "type": ["string", "null"], "minLength": 1 },
            "now": { "type": "string", "minLength": 1 }
        }
    })
}

fn non_negative_integer_schema() -> Value {
    json!({
        "type": "integer",
        "minimum": 0,
        "maximum": u64::MAX
    })
}

fn node_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["kind", "namespace", "key", "text"],
        "properties": {
            "kind": {
                "type": "string",
                "enum": ["intent", "claim", "assumption", "open-question"]
            },
            "namespace": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            },
            "key": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            },
            "text": { "type": "string", "minLength": 1 },
            "span": span_schema()
        }
    })
}

fn span_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["start", "end"],
        "properties": {
            "start": non_negative_integer_schema(),
            "end": non_negative_integer_schema()
        }
    })
}

fn evidence_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": [
            "namespace",
            "key",
            "method",
            "subject",
            "outcome",
            "execution",
            "provenance",
            "independence"
        ],
        "properties": {
            "namespace": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            },
            "key": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            },
            "method": {
                "type": "string",
                "enum": ["example", "case", "assert", "property", "production", "reference", "proof", "review"]
            },
            "subject": subject_schema(&["intent", "claim", "contract"]),
            "outcome": {
                "type": "string",
                "enum": ["pass", "fail", "contradicted", "unknown", "stale"]
            },
            "execution": execution_schema(),
            "provenance": {
                "type": "object",
                "additionalProperties": false,
                "required": ["producer", "tool_version", "timestamp"],
                "properties": {
                    "producer": { "type": "string", "minLength": 1 },
                    "tool_version": { "type": "string", "minLength": 1 },
                    "timestamp": { "type": "string", "minLength": 1 }
                }
            },
            "independence": {
                "type": "string",
                "enum": ["same-author", "independent-review", "external-observation"]
            }
        }
    })
}

fn execution_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["runner", "target", "source_commit", "artifact_digest", "sampling"],
        "properties": {
            "runner": { "type": "string", "minLength": 1 },
            "target": { "type": "string", "minLength": 1 },
            "source_commit": { "type": "string", "minLength": 1 },
            "artifact_digest": { "type": "string", "minLength": 1 },
            "sampling": sampling_schema()
        }
    })
}

fn sampling_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["cases", "seed", "generator"],
        "properties": {
            "cases": non_negative_integer_schema(),
            "seed": non_negative_integer_schema(),
            "generator": { "type": "string", "minLength": 1 },
            "shrinks": {
                "type": "array",
                "items": non_negative_integer_schema()
            },
            "coverage": {
                "type": "object",
                "propertyNames": { "type": "string", "pattern": "\\S" },
                "additionalProperties": non_negative_integer_schema()
            }
        }
    })
}

fn id_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["namespace", "key"],
        "properties": {
            "namespace": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            },
            "key": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            }
        }
    })
}

fn subject_schema(kinds: &[&str]) -> Value {
    let kind_values = kinds
        .iter()
        .map(|kind| Value::String((*kind).to_string()))
        .collect::<Vec<_>>();
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["kind", "namespace", "key"],
        "properties": {
            "kind": { "type": "string", "enum": kind_values },
            "namespace": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            },
            "key": {
                "type": "string",
                "pattern": "^[A-Za-z0-9_.-]+$"
            }
        }
    })
}

fn edge_schema() -> Value {
    json!({
        "oneOf": [
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["relation", "intent", "claim"],
                "properties": {
                    "relation": { "const": "motivates" },
                    "intent": id_schema(),
                    "claim": id_schema()
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["relation", "claim", "assumption"],
                "properties": {
                    "relation": { "const": "constrained-by" },
                    "claim": id_schema(),
                    "assumption": id_schema()
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["relation", "claim", "contract"],
                "properties": {
                    "relation": { "const": "tested-by" },
                    "claim": id_schema(),
                    "contract": id_schema()
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["relation", "observation", "claim"],
                "properties": {
                    "relation": { "enum": ["supports", "contradicts"] },
                    "observation": id_schema(),
                    "claim": id_schema()
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["relation", "review", "subject"],
                "properties": {
                    "relation": { "const": "evaluates" },
                    "review": id_schema(),
                    "subject": subject_schema(&["intent", "claim", "evidence"])
                }
            },
            {
                "type": "object",
                "additionalProperties": false,
                "required": ["relation", "change", "subject"],
                "properties": {
                    "relation": { "const": "invalidates" },
                    "change": id_schema(),
                    "subject": subject_schema(&["evidence", "review"])
                }
            }
        ]
    })
}

fn review_registry_schema() -> Value {
    json!({
        "type": "array",
        "items": {
            "type": "object",
            "additionalProperties": false,
            "required": [
                "namespace",
                "key",
                "provenance_digest",
                "visibility"
            ],
            "properties": {
                "namespace": {
                    "type": "string",
                    "pattern": "^[A-Za-z0-9_.-]+$"
                },
                "key": {
                    "type": "string",
                    "pattern": "^[A-Za-z0-9_.-]+$"
                },
                "provenance_digest": {
                    "type": "string",
                    "minLength": 1
                },
                "visibility": {
                    "type": "string",
                    "enum": ["public", "redacted"]
                },
                "verification_state": {
                    "type": "string",
                    "enum": ["verified", "unverified", "stale", "revoked"]
                }
            }
        }
    })
}
