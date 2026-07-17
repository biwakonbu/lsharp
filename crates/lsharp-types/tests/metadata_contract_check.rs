use lsharp_syntax::parse;
use lsharp_types::metadata_check::{Severity, check_metadata};

#[test]
fn canonical_bool_assertion_is_accepted() {
    const SOURCE: &str =
        "(defn positive [x] (> x 0)) (defn checked [] :assert [(positive 1) (positive 2)] true)";
    let program = parse(SOURCE).expect("Bool :assert は parse できるべき");

    let diagnostics = check_metadata(&program);

    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn canonical_assertion_requires_bool_at_predicate_span() {
    const SOURCE: &str = "(defn positive [] :assert [(+ 1 2)] true)";
    let program = parse(SOURCE).expect("non-Bool :assert も構文としては parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(
        diagnostic
            .message
            .contains(":assert predicate は Bool 必須")
    );
    assert!(diagnostic.message.contains("Int"));
    assert_eq!(
        &SOURCE[diagnostic.span.start..diagnostic.span.end],
        "(+ 1 2)"
    );
    assert_eq!(diagnostic.function_name, "positive");
}

#[test]
fn canonical_assertion_does_not_capture_defn_parameters() {
    const SOURCE: &str = "(defn identity [x] :assert [(= x x)] x)";
    let program = parse(SOURCE).expect("parameter reference を含む :assert は parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(
        diagnostic
            .message
            .contains(":assert predicate の型推論に失敗")
    );
    assert!(diagnostic.message.contains("未定義の変数 (undefined): x"));
    assert_eq!(&SOURCE[diagnostic.span.start..diagnostic.span.end], "x");
    assert_eq!(diagnostic.function_name, "identity");
}

#[test]
fn canonical_assertion_requires_at_least_one_predicate() {
    const SOURCE: &str = "(defn noop [] :assert [] true)";
    let program = parse(SOURCE).expect("空の :assert も diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(
        diagnostics.len(),
        1,
        "空の assertion を成功扱いしてはならない"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains(":assert"));
    assert!(diagnostic.message.contains("少なくとも 1 件"));
    assert_eq!(
        &SOURCE[diagnostic.span.start..diagnostic.span.end],
        ":assert []"
    );
    assert_eq!(diagnostic.function_name, "noop");
}

#[test]
fn canonical_assertion_non_vacuity_qualifies_module_owner() {
    const SOURCE: &str = "(module Demo (private (defn noop [] :assert [] true)))";
    let program = parse(SOURCE).expect("module 内の空 assertion も parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(
        diagnostics.len(),
        1,
        "module 内の空 assertion を成功扱いしてはならない"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.function_name, "Demo.noop");
    assert_eq!(
        &SOURCE[diagnostic.span.start..diagnostic.span.end],
        ":assert []"
    );
}

#[test]
fn canonical_assertion_rejects_literal_true_as_vacuous() {
    const SOURCE: &str = "(defn noop [] :assert [true] true)";
    let program = parse(SOURCE).expect("literal true assertion は parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(
        diagnostics.len(),
        1,
        "literal true を検査成功としてはならない"
    );
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains("vacuous"));
    assert_eq!(&SOURCE[diagnostic.span.start..diagnostic.span.end], "true");
    assert_eq!(diagnostic.function_name, "noop");
}

#[test]
fn canonical_assertion_rejects_statically_true_integer_comparisons_as_vacuous() {
    for predicate in [
        "(= 1 1)",
        "(< 1 2)",
        "(> 2 1)",
        "(<= 1 2)",
        "(>= 2 1)",
        "(!= 1 2)",
    ] {
        let source = format!("(defn noop [] :assert [{predicate}] true)");
        let program = parse(&source)
            .expect("静的に true な整数比較は diagnostic のため parse できるべき");

        let diagnostics = check_metadata(&program);

        assert_eq!(diagnostics.len(), 1, "{predicate}: {diagnostics:?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("vacuous"), "{diagnostics:?}");
        assert_eq!(
            &source[diagnostic.span.start..diagnostic.span.end],
            predicate
        );
        assert_eq!(diagnostic.function_name, "noop");
    }
}

#[test]
fn canonical_property_requires_at_least_one_for_all() {
    const SOURCE: &str = "(defn noop [] :property [] true)";
    let program = parse(SOURCE).expect("空の :property も diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1, "空の property を成功扱いしてはならない");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains(":property"));
    assert!(diagnostic.message.contains("少なくとも 1 件"));
    assert_eq!(diagnostic.function_name, "noop");
}

#[test]
fn canonical_property_rejects_zero_cases() {
    const SOURCE: &str =
        "(defn identity [x] :property [(for-all [x Int] :cases 0 :postcondition (= result x))] x)";
    let program = parse(SOURCE).expect("zero-case property は diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1, "zero case を成功扱いしてはならない");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains("case count"));
    assert_eq!(diagnostic.function_name, "identity");
}

#[test]
fn canonical_property_requires_a_typed_binder() {
    const SOURCE: &str =
        "(defn noop [] :property [(for-all [] :postcondition (>= result 0))] true)";
    let program = parse(SOURCE).expect("binder なし property は diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1, "binder なしを成功扱いしてはならない");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains("typed binder"));
    assert_eq!(diagnostic.function_name, "noop");
}

#[test]
fn canonical_property_rejects_literal_true_postcondition_as_vacuous() {
    const SOURCE: &str =
        "(defn identity [x] :property [(for-all [x Int] :postcondition true)] x)";
    let program = parse(SOURCE).expect("literal true property は diagnostic のため parse できるべき");

    let diagnostics = check_metadata(&program);

    assert_eq!(diagnostics.len(), 1, "literal true を成功扱いしてはならない");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.message.contains("vacuous"));
    assert_eq!(diagnostic.function_name, "identity");
}
