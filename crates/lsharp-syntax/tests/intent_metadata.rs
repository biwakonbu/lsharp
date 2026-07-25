use lsharp_syntax::{ast::Decl, metadata::MetadataFormKind, parse};

const SOURCE: &str = r#"
(defn cancel []
  :intent "intent:checkout/safe-cancel" "Users can cancel an order"
  :claim "claim:checkout/cancel-rejects-shipped" "The API rejects shipped orders"
  :assumption "assumption:checkout/state-authoritative" "Shipment state is authoritative"
  :open-question "open-question:checkout/cancel-after-label" "Can cancellation happen after a label?"
  true)
"#;

#[test]
fn intent_metadata_preserves_source_order_identity_text_and_span() {
    let program = parse(SOURCE).expect("intent metadata は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn を期待しました");
    };

    assert_eq!(metadata.forms.len(), 4);
    let ids = metadata
        .forms
        .iter()
        .map(|form| match &form.kind {
            MetadataFormKind::Intent { id, text }
            | MetadataFormKind::Claim { id, text }
            | MetadataFormKind::Assumption { id, text }
            | MetadataFormKind::OpenQuestion { id, text } => (id.as_str(), text.as_str()),
            other => panic!("unexpected metadata form: {other:?}"),
        })
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            ("intent:checkout/safe-cancel", "Users can cancel an order"),
            (
                "claim:checkout/cancel-rejects-shipped",
                "The API rejects shipped orders"
            ),
            (
                "assumption:checkout/state-authoritative",
                "Shipment state is authoritative"
            ),
            (
                "open-question:checkout/cancel-after-label",
                "Can cancellation happen after a label?"
            ),
        ]
    );
    assert!(metadata.forms.windows(2).all(|forms| {
        forms[0].span().start < forms[1].span().start && forms[0].span().end <= forms[1].span().end
    }));
}

#[test]
fn intent_metadata_requires_wire_id_and_non_empty_text() {
    let missing_id = parse(r#"(defn cancel [] :intent "Users can cancel" true)"#)
        .expect_err("intent id がない入力は拒否するべき");
    assert_eq!(missing_id.code(), "LS0101");

    let missing_text = parse(r#"(defn cancel [] :claim "claim:checkout/cancel" true)"#)
        .expect_err("claim text がない入力は拒否するべき");
    assert_eq!(missing_text.code(), "LS0101");
}

#[test]
fn type_definition_metadata_preserves_source_forms_and_span() {
    let source = r#"
    (type (Result e)
      (Ok Int)
      (Err e)
      :intent "intent:checkout/result" "The result models checkout completion"
      :claim "claim:checkout/result-total" "Every checkout returns a result")
    "#;
    let program = parse(source).expect("type definition metadata は parse できるべき");
    let Decl::TypeDef {
        name,
        type_params,
        variants,
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き type definition を期待しました");
    };

    assert_eq!(name, "Result");
    assert_eq!(type_params, &["e"]);
    assert_eq!(variants.len(), 2);
    assert_eq!(metadata.forms.len(), 2);
    assert!(matches!(
        &metadata.forms[0].kind,
        MetadataFormKind::Intent { id, text }
            if id == "intent:checkout/result"
                && text == "The result models checkout completion"
    ));
    assert!(matches!(
        &metadata.forms[1].kind,
        MetadataFormKind::Claim { id, text }
            if id == "claim:checkout/result-total"
                && text == "Every checkout returns a result"
    ));
    assert!(metadata.forms[0].span().start < metadata.forms[1].span().start);
    assert!(metadata.forms[1].span().end <= source.len());
}
