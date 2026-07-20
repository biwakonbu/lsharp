use lsharp_types::metadata_check::{TestKind, generate_tests};
use lsharp_types::metadata_contract::{
    DEFAULT_PROPERTY_CASES, DEFAULT_PROPERTY_SEED, ExecutableContract, GeneratorPlan,
    TYPE_DIRECTED_GENERATOR_VERSION, inventory_contract_suites,
};
use lsharp_types::types::Type;

#[test]
fn property_form_projects_to_typed_canonical_ir_with_sampling_plan() {
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
    let suites = inventory_contract_suites(&program).expect("canonical inventory を構築できるべき");

    assert_eq!(suites.len(), 1);
    assert!(suites[0].pending_migration().is_empty());
    assert_eq!(suites[0].executable().len(), 1);
    let ExecutableContract::Property(property) = &suites[0].executable()[0] else {
        panic!(":property は canonical Property へ投影するべき");
    };
    assert_eq!(property.binders().len(), 1);
    assert_eq!(property.binders()[0].name(), "x");
    assert_eq!(property.binders()[0].ty(), &Type::int());
    let binder_span = property.binders()[0].source_span();
    assert_eq!(&source[binder_span.start..binder_span.end], "x Int");
    assert_eq!(
        property.binders()[0].generator(),
        &GeneratorPlan::TypeDirected
    );
    assert_eq!(property.preconditions().len(), 1);
    let precondition_span = property.preconditions()[0].source_span();
    assert_eq!(
        &source[precondition_span.start..precondition_span.end],
        "(>= x -100)"
    );
    assert_eq!(
        format!("{}", property.preconditions()[0].expression()),
        "(>= x -100)"
    );
    let postcondition_span = property.postcondition().source_span();
    assert_eq!(
        &source[postcondition_span.start..postcondition_span.end],
        "(>= result 0)"
    );
    assert_eq!(
        format!("{}", property.postcondition().expression()),
        "(>= result 0)"
    );
    assert_eq!(property.sampling().cases(), 12);
    assert_eq!(property.sampling().seed(), 81_042);
    assert_eq!(
        property.sampling().generator_version(),
        TYPE_DIRECTED_GENERATOR_VERSION
    );
    assert!(!property.sampling().shrink());
    assert!(property.sampling().coverage_buckets().is_empty());
}

#[test]
fn property_form_uses_portable_sampling_defaults_when_options_are_omitted() {
    let source = r#"(defn identity [x]
  :property [(for-all [x Int]
               :postcondition (= result x))]
  x)
"#;
    let program = lsharp_syntax::parse(source).expect(":property source は parse できるべき");
    let suites = inventory_contract_suites(&program).expect("canonical inventory を構築できるべき");
    let ExecutableContract::Property(property) = &suites[0].executable()[0] else {
        panic!(":property は canonical Property へ投影するべき");
    };

    assert_eq!(property.sampling().cases(), DEFAULT_PROPERTY_CASES);
    assert_eq!(property.sampling().seed(), DEFAULT_PROPERTY_SEED);
    assert!(property.sampling().shrink());
}

#[test]
fn property_smoke_spec_accepts_single_string_binder() {
    let source = r#"(defn identity [value]
  :property [(for-all [sample String]
               :cases 5
               :postcondition (string-eq result sample))]
  value)
"#;
    let program = lsharp_syntax::parse(source).expect("String property source は parse できるべき");
    let tests = generate_tests(&program);

    assert_eq!(
        tests.len(),
        1,
        "single String binder は property test へ投影されるべき"
    );
    assert_eq!(tests[0].kind, TestKind::Property);
    assert!(tests[0].property.is_some());
}
