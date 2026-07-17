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

#[test]
fn canonical_case_metadata_preserves_expectations_and_spans() {
    const SOURCE: &str = "(defn abs [x] :case [(expect (abs -5) 5) (expect (abs 0) 0)] (+ x 1))";
    let program = parse(SOURCE).expect("canonical :case metadata は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("fixture は metadata 付き defn であるべき");
    };

    let [form] = metadata.forms.as_slice() else {
        panic!(":case は一つの lossless metadata form を保持するべき");
    };
    let MetadataFormKind::Case { expectations } = &form.kind else {
        panic!(":case は Case form として保持されるべき");
    };
    assert_eq!(expectations.len(), 2);
    assert_eq!(
        &SOURCE[form.span().start..form.span().end],
        ":case [(expect (abs -5) 5) (expect (abs 0) 0)]"
    );
    assert_eq!(
        &SOURCE[expectations[0].source_span().start..expectations[0].source_span().end],
        "(expect (abs -5) 5)"
    );
    assert_eq!(
        &SOURCE[expectations[0].actual().span().start..expectations[0].actual().span().end],
        "(abs -5)"
    );
    assert_eq!(
        &SOURCE[expectations[0].expected().span().start..expectations[0].expected().span().end],
        "5"
    );
}
