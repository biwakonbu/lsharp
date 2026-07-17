use lsharp_syntax::{ast::Decl, metadata::MetadataFormKind, parse};
use lsharp_types::metadata_contract::{ExecutableContract, inventory_contract_suites};

const ASSERT_CONTRACTS: &str = "(defn positive [] :assert [(> 1 0) (= 1 1)] true)";

#[test]
fn canonical_assert_forms_preserve_order_and_predicate_spans() {
    let program = parse(ASSERT_CONTRACTS).expect(":assert metadata は parse できるべき");
    let Decl::Defn {
        metadata: Some(metadata),
        ..
    } = &program.decls[0]
    else {
        panic!("fixture は metadata 付き defn であるべき");
    };
    let [form] = metadata.forms.as_slice() else {
        panic!(":assert vector は一つの lossless metadata form を保つべき");
    };
    let MetadataFormKind::Assertion { predicates } = &form.kind else {
        panic!(":assert は canonical Assertion form であるべき");
    };
    assert_eq!(predicates.len(), 2);
    assert_eq!(
        &ASSERT_CONTRACTS[form.span().start..form.span().end],
        ":assert [(> 1 0) (= 1 1)]"
    );

    let suites = inventory_contract_suites(&program)
        .expect(":assert metadata は canonical inventory に変換できるべき");

    assert_eq!(suites.len(), 1);
    let suite = &suites[0];
    assert_eq!(suite.owner().as_str(), "positive");
    assert!(suite.docs().is_empty());
    assert!(suite.pending_migration().is_empty());
    assert_eq!(suite.source_span(), form.span());

    let [
        ExecutableContract::Assertion(first),
        ExecutableContract::Assertion(second),
    ] = suite.executable()
    else {
        panic!(":assert vector は source 順の canonical Assertion になるべき");
    };

    assert_eq!(
        &ASSERT_CONTRACTS[first.source_span().start..first.source_span().end],
        "(> 1 0)"
    );
    assert_eq!(
        &ASSERT_CONTRACTS[second.source_span().start..second.source_span().end],
        "(= 1 1)"
    );
    assert_eq!(first.predicate().span(), first.source_span());
    assert_eq!(second.predicate().span(), second.source_span());
}
