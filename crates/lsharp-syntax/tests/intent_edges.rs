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
