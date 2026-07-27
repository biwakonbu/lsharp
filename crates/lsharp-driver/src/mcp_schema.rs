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
            "properties": {
                "error_code": { "type": "string" }
            },
            "required": ["error_code"]
        }),
        "lsharp_package_api" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "project_dir": { "type": "string" }
            },
            "required": ["name"]
        }),
        "lsharp_stdlib_api" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "module": { "type": "string" }
            }
        }),
        "lsharp_project_context" | "lsharp_search" => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "project_dir": { "type": "string" },
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
            "required": [
                "status",
                "trace_gaps",
                "open_questions",
                "independent_reviews",
                "contradicting_observations"
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
                        "required": ["code", "subject_id"],
                        "properties": {
                            "code": { "type": "string" },
                            "subject_id": { "type": "string" }
                        }
                    }
                },
                "open_questions": { "type": "integer", "minimum": 0 },
                "independent_reviews": { "type": "integer", "minimum": 0 },
                "contradicting_observations": { "type": "integer", "minimum": 0 },
                "manifest": {
                    "type": "object",
                    "required": ["schema_version", "nodes", "evidence", "edges"],
                    "properties": {
                        "schema_version": { "type": "integer", "const": 1 },
                        "nodes": { "type": "array" },
                        "reviews": review_registry_schema(),
                        "evidence": { "type": "array" },
                        "edges": { "type": "array" }
                    }
                }
            }
        }),
        _ => json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object"
        }),
    }
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
        "properties": {
            "source": { "type": "string" },
            "file": { "type": "string" },
            "manifest": {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "reviews": review_registry_schema()
                        }
                    },
                    { "type": "string" }
                ]
            },
            "manifest_file": { "type": "string" },
            "include_manifest": { "type": "boolean" }
        },
        "oneOf": [
            { "required": ["source"] },
            { "required": ["file"] },
            { "required": ["manifest"] },
            { "required": ["manifest_file"] }
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
                }
            }
        }
    })
}
