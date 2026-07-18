use lsharp_syntax::parse;
use lsharp_types::metadata_migration::{
    LegacyMigrationDisposition, LegacySelectedSemantics, classify_legacy_contracts,
};

#[test]
fn legacy_metadata_is_classified_without_silent_conversion() {
    let source = r#"
(defn succ [x]
  :example [(succ 0) (= (succ 1) 2)]
  :invariant (= result (+ x 1))
  (+ x 1))
"#;
    let program = parse(source).expect("legacy metadata fixture は parse できるべき");
    let diagnostics = classify_legacy_contracts(&program)
        .expect("legacy metadata は migration diagnostics を返すべき");

    assert_eq!(diagnostics.len(), 3);
    assert_eq!(diagnostics[0].code(), "LS2001");
    assert_eq!(
        diagnostics[0].selected_semantics(),
        LegacySelectedSemantics::ExampleTruthiness
    );
    assert_eq!(
        diagnostics[0].disposition(),
        LegacyMigrationDisposition::DocumentationExample
    );
    assert_eq!(diagnostics[1].code(), "LS2001");
    assert_eq!(
        diagnostics[1].disposition(),
        LegacyMigrationDisposition::Assertion
    );
    assert_eq!(diagnostics[2].code(), "LS2002");
    assert_eq!(
        diagnostics[2].selected_semantics(),
        LegacySelectedSemantics::InvariantDeterministicSmoke
    );
    assert_eq!(
        diagnostics[2].disposition(),
        LegacyMigrationDisposition::PropertyPostcondition
    );
}

#[test]
fn polymorphic_legacy_example_requires_manual_review() {
    let source = r#"
(defn identity [x]
  :example [(fn [value] value)]
  x)
"#;
    let program = parse(source).expect("polymorphic legacy example は parse できるべき");
    let diagnostics = classify_legacy_contracts(&program)
        .expect("polymorphic legacy example は migration diagnostic を返すべき");

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code(), "LS2003");
    assert_eq!(
        diagnostics[0].selected_semantics(),
        LegacySelectedSemantics::ExampleTruthiness
    );
    assert_eq!(
        diagnostics[0].disposition(),
        LegacyMigrationDisposition::ManualReview
    );
    assert!(diagnostics[0]
        .message()
        .starts_with("legacy :example は silent conversion できません。manual review が必要です:"));
}
