use serde_json::Value;

const INTENT_GRAPH_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/schemas/intent-graph.schema.json"
));

#[test]
fn intent_graph_schema_requires_non_empty_execution_and_provenance_strings() {
    let schema: Value =
        serde_json::from_str(INTENT_GRAPH_SCHEMA).expect("intent graph schema は JSON であるべき");
    let required_non_empty = [
        "/$defs/evidence/properties/execution/properties/runner",
        "/$defs/evidence/properties/execution/properties/target",
        "/$defs/evidence/properties/execution/properties/source_commit",
        "/$defs/evidence/properties/execution/properties/artifact_digest",
        "/$defs/evidence/properties/execution/properties/sampling/properties/generator",
        "/$defs/evidence/properties/provenance/properties/producer",
        "/$defs/evidence/properties/provenance/properties/tool_version",
        "/$defs/evidence/properties/provenance/properties/timestamp",
    ];

    for pointer in required_non_empty {
        assert_eq!(
            schema.pointer(pointer).and_then(Value::as_object).and_then(|property| {
                property.get("minLength").and_then(Value::as_u64)
            }),
            Some(1),
            "{pointer} は空文字を許可してはいけない"
        );
    }
}
