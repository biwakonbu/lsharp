use lsharp_syntax::{ast::Decl, metadata::MetadataFormKind, parse};

#[test]
fn intent_edge_metadata_preserves_typed_wire_ids_and_source_order() {
    let program = parse(
        r#"
        (defn cancel []
          :motivates "intent:checkout/safe-cancel" "claim:checkout/cancel-rejects-shipped"
          :constrained-by "claim:checkout/cancel-rejects-shipped" "assumption:checkout/state-authoritative"
          true)
        "#,
    )
    .expect("intent edge metadata は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn を期待しました");
    };

    assert_eq!(metadata.forms.len(), 2);
    assert!(matches!(
        &metadata.forms[0].kind,
        MetadataFormKind::Motivates { intent, claim }
            if intent == "intent:checkout/safe-cancel"
                && claim == "claim:checkout/cancel-rejects-shipped"
    ));
    assert!(matches!(
        &metadata.forms[1].kind,
        MetadataFormKind::ConstrainedBy { claim, assumption }
            if claim == "claim:checkout/cancel-rejects-shipped"
                && assumption == "assumption:checkout/state-authoritative"
    ));
    assert!(metadata.forms[0].span().start < metadata.forms[1].span().start);
}

#[test]
fn intent_edge_metadata_requires_both_wire_ids() {
    let missing_target = parse(r#"(defn cancel [] :motivates "intent:checkout/safe-cancel" true)"#)
        .expect_err("edge endpoint がない入力は拒否するべき");
    assert_eq!(missing_target.code(), "LS0101");
}

#[test]
fn tested_by_metadata_preserves_claim_and_contract_wire_ids() {
    let program = parse(
        r#"
        (defn cancel []
          :tested-by "claim:checkout/cancel-rejects-shipped" "contract:checkout/cancel-case"
          true)
        "#,
    )
    .expect("tested-by metadata は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn を期待しました");
    };

    assert!(matches!(
        &metadata.forms[0].kind,
        MetadataFormKind::TestedBy { claim, contract }
            if claim == "claim:checkout/cancel-rejects-shipped"
                && contract == "contract:checkout/cancel-case"
    ));
}

#[test]
fn evidence_edges_preserve_observation_and_claim_wire_ids() {
    let program = parse(
        r#"
        (defn cancel []
          :supports "evidence:checkout/cancel-observation" "claim:checkout/cancel-rejects-shipped"
          :contradicts "evidence:checkout/cancel-counterexample" "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect("evidence edge metadata は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn を期待しました");
    };

    assert!(matches!(
        &metadata.forms[0].kind,
        MetadataFormKind::Supports { observation, claim }
            if observation == "evidence:checkout/cancel-observation"
                && claim == "claim:checkout/cancel-rejects-shipped"
    ));
    assert!(matches!(
        &metadata.forms[1].kind,
        MetadataFormKind::Contradicts { observation, claim }
            if observation == "evidence:checkout/cancel-counterexample"
                && claim == "claim:checkout/cancel-rejects-shipped"
    ));
    assert!(metadata.forms[0].span().start < metadata.forms[1].span().start);
}

#[test]
fn evidence_record_metadata_preserves_required_fields_and_source_span() {
    let program = parse(
        r#"
        (defn cancel []
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "case"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 1
            :seed 42
            :generator "checkout-cancel-fixture"
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("evidence record metadata は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn を期待しました");
    };

    let MetadataFormKind::Evidence { record } = &metadata.forms[0].kind else {
        panic!("evidence record form を期待しました");
    };
    assert_eq!(record.id(), "evidence:checkout/cancel-observation");
    assert_eq!(record.subject(), "claim:checkout/cancel-rejects-shipped");
    assert_eq!(record.method(), "case");
    assert_eq!(record.outcome(), "pass");
    assert_eq!(record.cases(), 1);
    assert_eq!(record.seed(), 42);
    assert_eq!(record.independence(), "same-author");
    assert!(metadata.forms[0].span().start < metadata.forms[0].span().end);
}

#[test]
fn evidence_record_metadata_requires_all_named_fields() {
    let error = parse(
        r#"
        (defn cancel []
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
          true)
        "#,
    )
    .expect_err("evidence record の required field 欠落は拒否するべき");
    assert_eq!(error.code(), "LS0101");
}

#[test]
fn evidence_record_metadata_preserves_optional_sampling_fields() {
    let program = parse(
        r#"
        (defn cancel []
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped"
            :method "property"
            :outcome "pass"
            :runner "cargo-test"
            :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef"
            :artifact-digest "sha256:abc123"
            :cases 3
            :seed 42
            :generator "checkout-cancel-fixture"
            :shrinks [8 3 1]
            :coverage [("negative" 2) ("positive" 1)]
            :producer "lsharp-test"
            :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z"
            :independence "same-author"
          true)
        "#,
    )
    .expect("optional sampling fields 付き evidence record は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn を期待しました");
    };

    let MetadataFormKind::Evidence { record } = &metadata.forms[0].kind else {
        panic!("evidence record form を期待しました");
    };
    assert_eq!(record.shrinks(), &[8, 3, 1]);
    assert_eq!(
        record.coverage(),
        &[("negative".to_string(), 2), ("positive".to_string(), 1)]
    );
}

#[test]
fn evidence_record_metadata_rejects_invalid_optional_sampling_fields() {
    let negative_shrink = parse(
        r#"
        (defn cancel []
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped" :method "property"
            :outcome "pass" :runner "cargo-test" :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef" :artifact-digest "sha256:abc123"
            :cases 1 :seed 42 :generator "fixture" :shrinks [-1]
            :producer "lsharp-test" :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z" :independence "same-author"
          true)
        "#,
    )
    .expect_err("負の shrink は拒否するべき");
    assert_eq!(negative_shrink.code(), "LS0101");

    let duplicate_bucket = parse(
        r#"
        (defn cancel []
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped" :method "property"
            :outcome "pass" :runner "cargo-test" :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef" :artifact-digest "sha256:abc123"
            :cases 1 :seed 42 :generator "fixture"
            :coverage [("same" 1) ("same" 2)]
            :producer "lsharp-test" :tool-version "0.2.0"
            :timestamp "2026-07-25T00:00:00Z" :independence "same-author"
          true)
        "#,
    )
    .expect_err("重複 coverage bucket は拒否するべき");
    assert_eq!(duplicate_bucket.code(), "LS0101");

    let unclosed_shrinks = parse(
        r#"
        (defn cancel []
          :evidence "evidence:checkout/cancel-observation"
            :subject "claim:checkout/cancel-rejects-shipped" :method "property"
            :outcome "pass" :runner "cargo-test" :target "aarch64-apple-darwin"
            :source-commit "0123456789abcdef" :artifact-digest "sha256:abc123"
            :cases 1 :seed 42 :generator "fixture" :shrinks [1
        "#,
    )
    .expect_err("閉じていない shrink list は診断するべき");
    assert_eq!(unclosed_shrinks.code(), "LS0102");
}
