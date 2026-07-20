use lsharp_syntax::{ast::Decl, metadata::MetadataFormKind, parse};
use lsharp_types::metadata_check::{Severity, check_metadata};
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

#[test]
fn canonical_case_requires_matching_actual_and_expected_types() {
    const SOURCE: &str = "(defn noop [] :case [(expect 1 true)] true)";
    let program = parse(SOURCE).expect("型不一致 case も diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(
        diagnostics.len(),
        1,
        "型不一致 case を成功扱いしてはならない"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains(":case actual / expected"));
    assert!(diagnostic.message.contains("Int"));
    assert!(diagnostic.message.contains("Bool"));
    assert_eq!(&SOURCE[diagnostic.span.start..diagnostic.span.end], "true");
    assert_eq!(diagnostic.function_name, "noop");
}

#[test]
fn canonical_case_requires_at_least_one_expectation() {
    const SOURCE: &str = "(defn noop [] :case [] 0)";
    let program = parse(SOURCE).expect("空 case も diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(
        diagnostics.len(),
        1,
        "空 case をテスト 0 件の成功扱いしてはならない"
    );
    assert!(diagnostics[0].message.contains(":case は少なくとも 1 件"));
    assert_eq!(diagnostics[0].function_name, "noop");
}

#[test]
fn canonical_case_accepts_int_and_bool_comparisons() {
    const SOURCE: &str = "(defn noop [] :case [(expect 1 1) (expect true false)] true)";
    let program = parse(SOURCE).expect("Int/Bool case は parse できるべき");

    let diagnostics = check_metadata(&program);

    assert!(
        diagnostics.is_empty(),
        "Int/Bool case は型検査を通るべき: {diagnostics:?}"
    );
}

#[test]
fn canonical_case_rejects_unsupported_string_comparison() {
    const SOURCE: &str = "(defn noop [] :case [(expect \"a\" \"a\")] true)";
    let program = parse(SOURCE).expect("String case は diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("Int / Bool"));
    assert_eq!(
        &SOURCE[diagnostics[0].span.start..diagnostics[0].span.end],
        "\"a\"",
        "unsupported String case は actual 式の span を正本にするべき"
    );
    assert_eq!(diagnostics[0].function_name, "noop");
}

#[test]
fn canonical_case_does_not_capture_defn_parameters() {
    const SOURCE: &str = "(defn identity [x] :case [(expect x 1)] x)";
    let program = parse(SOURCE).expect("parameter capture case は parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains(":case の型推論に失敗"));
    assert!(diagnostics[0].message.contains("未定義の変数"));
    assert_eq!(
        &SOURCE[diagnostics[0].span.start..diagnostics[0].span.end],
        "x"
    );
    assert_eq!(diagnostics[0].function_name, "identity");
}
