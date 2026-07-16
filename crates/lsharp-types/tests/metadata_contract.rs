use lsharp_syntax::ast::{Decl, Expr};
use lsharp_syntax::metadata::MetadataFormKind;
use lsharp_syntax::parse;
use lsharp_types::metadata_contract::{
    ContractInventoryError, LegacyContract, inventory_contract_suites,
};

const LEGACY_CONTRACT_FORMS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../lsharp-syntax/tests/fixtures/metadata/legacy_contract_forms.ls"
));

#[test]
fn legacy_forms_remain_pending_migration_in_contract_suite() {
    let program =
        parse(LEGACY_CONTRACT_FORMS).expect("legacy metadata fixture は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("fixture の先頭宣言は metadata 付き defn であるべき");
    };

    let suites =
        inventory_contract_suites(&program).expect("lossless metadata は inventory できるべき");
    assert_eq!(suites.len(), 1);
    let suite = &suites[0];

    assert_eq!(suite.owner().as_str(), "succ");
    assert!(suite.docs().is_empty());
    assert!(suite.executable().is_empty());
    assert!(suite.intent_links().is_empty());
    assert!(suite.claim_links().is_empty());
    assert_eq!(suite.pending_migration().len(), 3);
    assert_eq!(
        suite.source_span(),
        metadata.forms[0].span().merge(metadata.forms[2].span())
    );

    for (pending, syntax_form) in suite.pending_migration().iter().zip(&metadata.forms) {
        assert_eq!(pending.source_span(), syntax_form.span());
    }

    assert_legacy_example(&suite.pending_migration()[0], &metadata.forms[0].kind);
    assert_legacy_invariant(&suite.pending_migration()[1], &metadata.forms[1].kind);
    assert_legacy_example(&suite.pending_migration()[2], &metadata.forms[2].kind);
}

#[test]
fn aggregate_contract_without_ordered_forms_fails_closed() {
    let mut program =
        parse(LEGACY_CONTRACT_FORMS).expect("legacy metadata fixture は parse できるべき");
    metadata_mut(&mut program).forms.clear();

    assert_eq!(
        inventory_contract_suites(&program).unwrap_err(),
        ContractInventoryError::MissingOrderedForms {
            owner: "succ".to_string(),
        }
    );
}

#[test]
fn aggregate_and_ordered_form_mismatch_fails_closed() {
    let mut program =
        parse(LEGACY_CONTRACT_FORMS).expect("legacy metadata fixture は parse できるべき");
    metadata_mut(&mut program).example.pop();

    assert_eq!(
        inventory_contract_suites(&program).unwrap_err(),
        ContractInventoryError::ProjectionMismatch {
            owner: "succ".to_string(),
        }
    );
}

fn metadata_mut(program: &mut lsharp_syntax::ast::Program) -> &mut lsharp_syntax::ast::Metadata {
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &mut program.decls[0]
    else {
        panic!("fixture の先頭宣言は metadata 付き defn であるべき");
    };
    metadata
}

fn assert_legacy_example(contract: &LegacyContract, syntax_form: &MetadataFormKind) {
    let LegacyContract::Example { expressions, .. } = contract else {
        panic!("legacy :example は pending Example のまま保持するべき");
    };
    let MetadataFormKind::LegacyExample {
        expressions: syntax_expressions,
    } = syntax_form
    else {
        panic!("syntax form は LegacyExample であるべき");
    };
    assert_eq!(expressions, syntax_expressions);
}

fn assert_legacy_invariant(contract: &LegacyContract, syntax_form: &MetadataFormKind) {
    let LegacyContract::Invariant { predicate, .. } = contract else {
        panic!("legacy :invariant は pending Invariant のまま保持するべき");
    };
    let MetadataFormKind::LegacyInvariant {
        predicate: syntax_predicate,
    } = syntax_form
    else {
        panic!("syntax form は LegacyInvariant であるべき");
    };
    assert_eq!(predicate, syntax_predicate);
    assert!(matches!(predicate, Expr::App(..)));
}
