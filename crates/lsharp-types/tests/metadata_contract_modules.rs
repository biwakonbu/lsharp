use lsharp_syntax::ast::Expr;
use lsharp_syntax::parse;
use lsharp_types::metadata_contract::{LegacyContract, inventory_contract_suites};

const NESTED_CONTRACT_FORMS: &str = include_str!("fixtures/metadata/nested_contract_forms.ls");

#[test]
fn nested_module_contract_inventory_uses_qualified_owner() {
    let program =
        parse(NESTED_CONTRACT_FORMS).expect("nested metadata fixture は parse できるべき");
    let suites =
        inventory_contract_suites(&program).expect("nested contract は inventory できるべき");

    assert_eq!(suites.len(), 1);
    let suite = &suites[0];
    assert_eq!(suite.owner().as_str(), "App.Sub.succ");
    assert!(suite.docs().is_empty());
    assert!(suite.executable().is_empty());

    let [
        LegacyContract::Example {
            expressions,
            source_span: example_span,
        },
        LegacyContract::Invariant {
            predicate,
            source_span: invariant_span,
        },
    ] = suite.pending_migration()
    else {
        panic!("nested succ は legacy example / invariant を source 順に保持するべき");
    };
    assert_eq!(expressions.len(), 1);
    assert!(matches!(predicate, Expr::App(..)));
    assert_eq!(
        &NESTED_CONTRACT_FORMS[example_span.start..example_span.end],
        ":example [(succ 0)]"
    );
    assert_eq!(
        &NESTED_CONTRACT_FORMS[invariant_span.start..invariant_span.end],
        ":invariant (= result (+ x 1))"
    );
}
