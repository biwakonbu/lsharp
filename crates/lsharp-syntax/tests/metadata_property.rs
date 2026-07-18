use lsharp_syntax::ast::{Decl, TypeExpr};
use lsharp_syntax::metadata::MetadataFormKind;

#[test]
fn property_form_preserves_binders_sampling_options_predicates_and_spans() {
    let source = r#"(defn abs [x]
  :property [(for-all [x Int]
               :cases 12
               :seed 81042
               :shrink false
               :precondition [(>= x -100)]
               :postcondition (>= result 0))]
  (if (< x 0) (- 0 x) x))
"#;
    let program = lsharp_syntax::parse(source).expect(":property source は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("metadata 付き defn が必要");
    };

    assert_eq!(metadata.forms.len(), 1);
    let MetadataFormKind::Property { properties } = &metadata.forms[0].kind else {
        panic!(":property は lossless Property form として保持するべき");
    };
    assert_eq!(properties.len(), 1);
    let property = &properties[0];
    assert_eq!(property.binders().len(), 1);
    assert_eq!(property.binders()[0].name(), "x");
    assert!(matches!(
        property.binders()[0].ty(),
        TypeExpr::Named(_, name) if name == "Int"
    ));
    assert_eq!(property.cases(), Some(12));
    assert_eq!(property.seed(), Some(81_042));
    assert_eq!(property.shrink(), Some(false));
    assert_eq!(property.preconditions().len(), 1);
    assert_eq!(format!("{}", property.preconditions()[0]), "(>= x -100)");
    assert_eq!(format!("{}", property.postcondition()), "(>= result 0)");
    assert!(property.source_span().start < property.source_span().end);
    assert!(metadata.forms[0].span().start <= property.source_span().start);
    assert!(metadata.forms[0].span().end >= property.source_span().end);
}

#[test]
fn property_form_requires_a_postcondition() {
    let source = "(defn noop [] :property [(for-all [x Int] :cases 1)] true)\n";
    let error =
        lsharp_syntax::parse(source).expect_err(":postcondition がない property は拒否するべき");

    assert!(error.to_string().contains(":postcondition"));
}

#[test]
fn property_form_rejects_negative_cases() {
    let source =
        "(defn identity [x] :property [(for-all [x Int] :cases -1 :postcondition (= result x))] x)";
    let error =
        lsharp_syntax::parse(source).expect_err("negative :cases は parse できてはならない");

    assert!(error.to_string().contains("non-negative case count"));
}

#[test]
fn property_form_rejects_non_numeric_cases() {
    let source =
        "(defn identity [x] :property [(for-all [x Int] :cases false :postcondition (= result x))] x)";
    let error =
        lsharp_syntax::parse(source).expect_err("非数値 :cases は parse できてはならない");

    assert!(error.to_string().contains("non-negative case count"));
}

#[test]
fn property_form_rejects_unknown_option() {
    let source = "(defn identity [x] :property [(for-all [x Int] :unknown true :cases 1 :postcondition (= result x))] x)";
    let error = lsharp_syntax::parse(source).expect_err("未知の property option は拒否するべき");

    assert!(error.to_string().contains("property option"));
}

#[test]
fn property_form_rejects_prefixed_option_name() {
    let source = "(defn identity [x] :property [(for-all [x Int] :cases-extra true :postcondition (= result x))] x)";
    let error = lsharp_syntax::parse(source).expect_err("既知 option の prefix を持つ未知 option は拒否するべき");

    assert!(error.to_string().contains("property option"));
}

#[test]
fn property_form_rejects_missing_scalar_option_value() {
    let source = "(defn identity [x] :property [(for-all [x Int] :cases 1 :seed :postcondition (= result x))] x)";
    let error = lsharp_syntax::parse(source).expect_err("値が欠落した scalar option は拒否するべき");

    assert!(error.to_string().contains("non-negative seed"));
}
