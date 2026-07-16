use lsharp_syntax::ast::Decl;
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::parse;

const LEGACY_CONTRACT_FORMS: &str = include_str!("fixtures/metadata/legacy_contract_forms.ls");

#[test]
fn legacy_contract_metadata_forms_preserve_source_order_and_spans() {
    let program =
        parse(LEGACY_CONTRACT_FORMS).expect("legacy metadata fixture は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("fixture の先頭宣言は metadata 付き defn であるべき");
    };

    assert_eq!(metadata.forms.len(), 3);
    assert!(matches!(
        &metadata.forms[0].kind,
        MetadataFormKind::LegacyExample { expressions } if expressions.len() == 1
    ));
    assert!(matches!(
        &metadata.forms[1].kind,
        MetadataFormKind::LegacyInvariant { .. }
    ));
    assert!(matches!(
        &metadata.forms[2].kind,
        MetadataFormKind::LegacyExample { expressions } if expressions.len() == 1
    ));

    let source_forms = metadata
        .forms
        .iter()
        .map(|form| &LEGACY_CONTRACT_FORMS[form.span().start..form.span().end])
        .collect::<Vec<_>>();
    assert_eq!(
        source_forms,
        vec![
            ":example [(succ 0)]",
            ":invariant (= result (+ x 1))",
            ":example [(succ 1)]",
        ]
    );

    assert_eq!(metadata.example.len(), 2);
    assert!(metadata.invariant.is_some());
    let MetadataFormKind::LegacyExample { expressions } = &metadata.forms[0].kind else {
        unreachable!();
    };
    assert_eq!(expressions[0], metadata.example[0]);
    let MetadataFormKind::LegacyInvariant { predicate } = &metadata.forms[1].kind else {
        unreachable!();
    };
    assert_eq!(predicate, metadata.invariant.as_ref().unwrap());
}
