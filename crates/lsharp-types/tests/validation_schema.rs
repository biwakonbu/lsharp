use serde_json::{Value, json};

const INTENT_GRAPH_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/schemas/intent-graph.schema.json"
));
const INTENT_VALIDATION_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/schemas/intent-validation.schema.json"
));
const REVIEW_PROVENANCE_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/schemas/review-provenance-v1.schema.json"
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
            schema
                .pointer(pointer)
                .and_then(Value::as_object)
                .and_then(|property| { property.get("minLength").and_then(Value::as_u64) }),
            Some(1),
            "{pointer} は空文字を許可してはいけない"
        );
    }
}

#[test]
fn review_provenance_schema_requires_canonical_timestamp_for_lifecycle_effective_at() {
    let schema: Value = serde_json::from_str(REVIEW_PROVENANCE_SCHEMA)
        .expect("review provenance schema は JSON であるべき");
    let effective_at = schema
        .pointer("/$defs/lifecycle/properties/effective_at")
        .expect("lifecycle.effective_at schema が必要");

    assert_eq!(
        effective_at["$ref"],
        "#/$defs/canonical_utc_timestamp",
        "lifecycle.effective_at は non-empty string ではなく canonical UTC timestamp を要求するべき"
    );
}

#[test]
fn review_provenance_schema_bounds_sequences_to_unsigned_64_bit_values() {
    let schema: Value = serde_json::from_str(REVIEW_PROVENANCE_SCHEMA)
        .expect("review provenance schema は JSON であるべき");

    for pointer in [
        "/$defs/attestation/properties/sequence",
        "/$defs/lifecycle/properties/sequence",
    ] {
        assert_eq!(
            schema
                .pointer(pointer)
                .and_then(|value| value.get("maximum")),
            Some(&json!(u64::MAX)),
            "{pointer} は Rust wire parser の u64 境界を schema へ公開するべき"
        );
    }
}

#[test]
fn intent_graph_schema_declares_optional_review_verification_state() {
    let schema: Value =
        serde_json::from_str(INTENT_GRAPH_SCHEMA).expect("intent graph schema は JSON であるべき");
    let review = schema
        .pointer("/$defs/review")
        .expect("review registry schema が必要");

    assert_eq!(
        review["properties"]["verification_state"]["enum"],
        serde_json::json!(["verified", "unverified", "stale", "revoked"])
    );
    assert!(
        !review["required"]
            .as_array()
            .expect("review required は array であるべき")
            .iter()
            .any(|field| field == "verification_state")
    );
}

#[test]
fn intent_graph_schema_declares_optional_review_evidence_identity() {
    let schema: Value =
        serde_json::from_str(INTENT_GRAPH_SCHEMA).expect("intent graph schema は JSON であるべき");
    let identity = schema
        .pointer("/properties/review_evidence_identity")
        .expect("manifest は optional review evidence identity を宣言するべき");

    assert_eq!(
        identity["required"],
        serde_json::json!([
            "subject_digest",
            "source_commit",
            "artifact_digest",
            "trust_store_digest",
            "lifecycle_digest",
            "now"
        ])
    );
    assert_eq!(identity["additionalProperties"], false);
    assert_eq!(
        identity["properties"]["trust_store_digest"]["minLength"],
        1
    );
    assert_eq!(
        identity["properties"]["lifecycle_digest"]["minLength"],
        1
    );
    assert!(
        !schema["required"]
            .as_array()
            .expect("manifest required は array であるべき")
            .iter()
            .any(|field| field == "review_evidence_identity")
    );
}

#[test]
fn intent_validation_schema_declares_optional_canonical_manifest() {
    let schema: Value = serde_json::from_str(INTENT_VALIDATION_SCHEMA)
        .expect("intent validation schema は JSON であるべき");
    let manifest = schema
        .pointer("/properties/manifest")
        .expect("report schema は optional manifest を宣言するべき");

    assert_eq!(manifest["$ref"], "intent-graph.schema.json");
    assert_eq!(schema["additionalProperties"], false);
    assert!(
        !schema["required"]
            .as_array()
            .expect("report required は array であるべき")
            .iter()
            .any(|field| field == "manifest")
    );
}

#[test]
fn intent_validation_schema_declares_optional_review_verification_facts() {
    let schema: Value = serde_json::from_str(INTENT_VALIDATION_SCHEMA)
        .expect("intent validation schema は JSON であるべき");
    let verification = schema
        .pointer("/properties/review_verifications")
        .expect("review verification facts は optional array を宣言するべき");

    assert_eq!(verification["type"], "array");
    assert_eq!(
        verification["items"]["properties"]["state"]["enum"],
        serde_json::json!(["verified", "unverified", "stale", "revoked"])
    );
    assert!(
        !schema["required"]
            .as_array()
            .expect("report required は array であるべき")
            .iter()
            .any(|field| field == "review_verifications")
    );
}

#[test]
fn intent_validation_schema_declares_optional_review_evidence_identity() {
    let schema: Value = serde_json::from_str(INTENT_VALIDATION_SCHEMA)
        .expect("intent validation schema は JSON であるべき");
    let identity = schema
        .pointer("/properties/review_evidence_identity")
        .expect("review evidence identity は optional object を宣言するべき");

    assert_eq!(
        identity["required"],
        serde_json::json!([
            "subject_digest",
            "source_commit",
            "artifact_digest",
            "trust_store_digest",
            "lifecycle_digest",
            "now"
        ])
    );
    assert_eq!(
        identity["properties"]["trust_store_digest"]["type"],
        serde_json::json!(["string", "null"])
    );
    assert_eq!(
        identity["properties"]["lifecycle_digest"]["type"],
        serde_json::json!(["string", "null"])
    );
    assert!(
        !schema["required"]
            .as_array()
            .expect("report required は array であるべき")
            .iter()
            .any(|field| field == "review_evidence_identity")
    );
}

#[test]
fn intent_graph_schema_declares_typed_subjects_for_each_consumer() {
    let schema: Value =
        serde_json::from_str(INTENT_GRAPH_SCHEMA).expect("intent graph schema は JSON であるべき");

    let evidence_subject = schema
        .pointer("/$defs/evidence/properties/subject/$ref")
        .expect("evidence subject の ref が必要");
    assert_eq!(evidence_subject, "#/$defs/evidence-subject");

    let variants = schema
        .pointer("/properties/edges/items/oneOf")
        .and_then(Value::as_array)
        .expect("edge relation variants は array であるべき");

    let evaluates = variants
        .iter()
        .find(|variant| {
            variant["required"]
                .as_array()
                .is_some_and(|required| required.iter().any(|field| field == "review"))
        })
        .expect("evaluates relation variant が必要");
    assert_eq!(
        evaluates["properties"]["subject"]["$ref"],
        "#/$defs/review-subject"
    );

    let invalidates = variants
        .iter()
        .find(|variant| {
            variant["required"]
                .as_array()
                .is_some_and(|required| required.iter().any(|field| field == "change"))
        })
        .expect("invalidates relation variant が必要");
    assert_eq!(
        invalidates["properties"]["subject"]["$ref"],
        "#/$defs/invalidation-subject"
    );

    let review_subject_kinds = schema
        .pointer("/$defs/review-subject/properties/kind/enum")
        .and_then(Value::as_array)
        .expect("review subject の kind enum が必要");
    let expected_review_subject_kinds = serde_json::json!(["intent", "claim", "evidence"]);
    assert_eq!(
        review_subject_kinds,
        expected_review_subject_kinds.as_array().unwrap()
    );

    let invalidation_subject_kinds = schema
        .pointer("/$defs/invalidation-subject/properties/kind/enum")
        .and_then(Value::as_array)
        .expect("invalidation subject の kind enum が必要");
    let expected_invalidation_subject_kinds = serde_json::json!(["evidence", "review"]);
    assert_eq!(
        invalidation_subject_kinds,
        expected_invalidation_subject_kinds.as_array().unwrap()
    );

    let evidence_subject_kinds = schema
        .pointer("/$defs/evidence-subject/properties/kind/enum")
        .and_then(Value::as_array)
        .expect("evidence subject の kind enum が必要");
    let expected_evidence_subject_kinds = serde_json::json!(["intent", "claim", "contract"]);
    assert_eq!(
        evidence_subject_kinds,
        expected_evidence_subject_kinds.as_array().unwrap()
    );
}
