use lsharp_syntax::{ast::Decl, metadata::MetadataFormKind, parse};
use lsharp_types::metadata_contract::{
    ExecutableContract, ExpectedOutcome, inventory_contract_suites,
};

#[test]
fn canonical_case_forms_project_to_ordered_inventory_entries() {
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
        panic!(":case は一つの metadata form を保持するべき");
    };
    let MetadataFormKind::Case { expectations } = &form.kind else {
        panic!(":case は Case form として保持されるべき");
    };

    let suites = inventory_contract_suites(&program).expect(":case は inventory 化できるべき");
    let [suite] = suites.as_slice() else {
        panic!("contract suite は一つ必要");
    };
    assert_eq!(suite.owner().as_str(), "abs");
    let [
        ExecutableContract::Case(first),
        ExecutableContract::Case(second),
    ] = suite.executable()
    else {
        panic!(":case は source 順の Case へ変換されるべき");
    };
    assert_eq!(first.actual(), expectations[0].actual());
    assert_eq!(second.actual(), expectations[1].actual());
    assert_eq!(first.source_span(), expectations[0].source_span());
    assert!(matches!(
        first.expected(),
        ExpectedOutcome::Value(expected) if expected == expectations[0].expected()
    ));
}
