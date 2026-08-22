use super::{
    check_assertion_non_vacuity, check_assertion_types, check_case_non_vacuity, check_case_types,
    check_property_non_vacuity, check_property_types,
};
use crate::metadata_check::{MetadataDiagnostic, Severity, check_metadata};
use lsharp_syntax::ast::Program;

#[test]
fn canonical_contract_check_modules_preserve_empty_program_contract() {
    let program = Program { decls: Vec::new() };

    assert!(check_assertion_non_vacuity(&program).is_empty());
    assert!(check_case_non_vacuity(&program).is_empty());
    assert!(check_property_non_vacuity(&program).is_empty());
    assert!(check_assertion_types(&program, &[]).is_empty());
    assert!(check_case_types(&program, &[]).is_empty());
    assert!(check_property_types(&program, &[]).is_empty());
}

/// `STATIC-CONTRACT-01`: probe は CLI ではなく `check_metadata` を直接叩く。
/// `lsharp test` は selfhost runner 経由なので vacuous と正当を判別できない。
fn check(source: &str) -> Vec<MetadataDiagnostic> {
    let program = lsharp_syntax::parse(source).unwrap_or_else(|error| {
        panic!("fixture が parse できない: {error:?}\n--- source ---\n{source}")
    });
    check_metadata(&program)
}

fn errors_containing(source: &str, needle: &str) -> Vec<String> {
    check(source)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(|diagnostic| diagnostic.message)
        .filter(|message| message.contains(needle))
        .collect()
}

const ASSERT_VACUOUS: &str = ":assert predicate は静的に true で検査を識別できず vacuous です";
const PRECONDITION_VACUOUS: &str = ":property の precondition は到達不能で vacuous です";

fn assert_source(predicate: &str) -> String {
    format!("(defn checked [] :assert [{predicate}] true)")
}

fn precondition_source(precondition: &str) -> String {
    format!(
        "(defn checked [value] :property [(for-all [value Int] :cases 1 \
         :precondition [{precondition}] :postcondition (= result value))] value)"
    )
}

// --- control: 直書きの literal は既に検出されている ---

#[test]
fn static_contract_assert_literal_true_is_vacuous_control() {
    assert_eq!(
        errors_containing(&assert_source("true"), ASSERT_VACUOUS).len(),
        1,
        "control が壊れていると以下の 4 形の RED が意味を持たない"
    );
}

#[test]
fn static_contract_precondition_literal_false_is_vacuous_control() {
    assert_eq!(
        errors_containing(&precondition_source("false"), PRECONDITION_VACUOUS).len(),
        1,
        "control が壊れていると以下の 4 形の RED が意味を持たない"
    );
}

// --- `:assert` 側: if / let / do / match に包まれた静的 true ---

#[test]
fn static_contract_assert_if_wrapped_true_is_vacuous() {
    assert_eq!(
        errors_containing(&assert_source("(if true true false)"), ASSERT_VACUOUS).len(),
        1,
        "静的に true な if は control (:assert [true]) と同じ診断を出すべき"
    );
}

#[test]
fn static_contract_assert_let_wrapped_true_is_vacuous() {
    assert_eq!(
        errors_containing(&assert_source("(let [ignored 1] true)"), ASSERT_VACUOUS).len(),
        1,
        "静的に true な let body は control と同じ診断を出すべき"
    );
}

#[test]
fn static_contract_assert_do_wrapped_true_is_vacuous() {
    assert_eq!(
        errors_containing(&assert_source("(do 1 true)"), ASSERT_VACUOUS).len(),
        1,
        "do の最終式が静的に true なら control と同じ診断を出すべき"
    );
}

#[test]
fn static_contract_assert_match_wrapped_true_is_vacuous() {
    assert_eq!(
        errors_containing(&assert_source("(match 1 [_ true])"), ASSERT_VACUOUS).len(),
        1,
        "全 arm が静的に true な match は control と同じ診断を出すべき"
    );
}

// --- `:property` の precondition 側: if / let / do / match に包まれた静的 false ---

#[test]
fn static_contract_precondition_if_wrapped_false_is_vacuous() {
    assert_eq!(
        errors_containing(
            &precondition_source("(if true false true)"),
            PRECONDITION_VACUOUS
        )
        .len(),
        1,
        "静的に false な if precondition は到達不能として診断されるべき"
    );
}

#[test]
fn static_contract_precondition_let_wrapped_false_is_vacuous() {
    assert_eq!(
        errors_containing(
            &precondition_source("(let [ignored 1] false)"),
            PRECONDITION_VACUOUS
        )
        .len(),
        1,
        "静的に false な let body precondition は到達不能として診断されるべき"
    );
}

#[test]
fn static_contract_precondition_do_wrapped_false_is_vacuous() {
    assert_eq!(
        errors_containing(&precondition_source("(do 1 false)"), PRECONDITION_VACUOUS).len(),
        1,
        "do の最終式が静的に false な precondition は到達不能として診断されるべき"
    );
}

#[test]
fn static_contract_precondition_match_wrapped_false_is_vacuous() {
    assert_eq!(
        errors_containing(
            &precondition_source("(match 1 [_ false])"),
            PRECONDITION_VACUOUS
        )
        .len(),
        1,
        "全 arm が静的に false な match precondition は到達不能として診断されるべき"
    );
}

// --- 負の対照: 静的に決まらない形を vacuous と誤判定しないこと ---

#[test]
fn static_contract_assert_dynamic_if_is_not_vacuous() {
    let source = "(defn checked [flag] :assert [(if flag true false)] flag)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "条件も分岐も静的に決まらない if を vacuous 扱いしてはならない"
    );
}

#[test]
fn static_contract_assert_dynamic_let_body_is_not_vacuous() {
    let source = "(defn checked [flag] :assert [(let [ignored 1] flag)] flag)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "body が動的な let を vacuous 扱いしてはならない"
    );
}

#[test]
fn static_contract_assert_mixed_match_arms_are_not_vacuous() {
    let source = "(defn checked [n] :assert [(match n [1 true] [_ false])] true)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "arm ごとに結果が違う match を vacuous 扱いしてはならない"
    );
}

#[test]
fn static_contract_precondition_dynamic_do_is_not_vacuous() {
    let source = "(defn checked [value] :property [(for-all [value Int] :cases 1 \
                  :precondition [(do 1 (> value 0))] :postcondition (= result value))] value)";
    assert!(
        errors_containing(source, PRECONDITION_VACUOUS).is_empty(),
        "最終式が動的な do precondition を到達不能扱いしてはならない"
    );
}

#[test]
fn static_contract_assert_shadowed_operator_in_let_is_not_vacuous() {
    // `not` を let で再束縛しているので、builtin の意味で静的評価してはならない。
    // guard が無いと `(not false)` を true と読んで vacuous と誤診断する。
    let source = "(defn checked [f] :assert [(let [not f] (not false))] true)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "影に隠れた束縛がある let を builtin の意味で静的評価してはならない"
    );
}

#[test]
fn static_contract_assert_shadowed_operator_in_match_arm_is_not_vacuous() {
    // arm の pattern が `not` を束縛しているので、その body の `not` は builtin ではない。
    let source = "(defn checked [n] :assert [(match n [not (not false)])] true)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "影に隠れた束縛がある match arm を builtin の意味で静的評価してはならない"
    );
}

#[test]
fn static_contract_assert_let_bound_static_var_is_vacuous() {
    // `I-42` が挙げた形。body が束縛変数そのものなので、束縛値まで辿らないと判定できない。
    assert_eq!(
        errors_containing(&assert_source("(let [a true] a)"), ASSERT_VACUOUS).len(),
        1,
        "静的に true な値で束縛された変数を body に置いた let も control と同じ診断を出すべき"
    );
}

#[test]
fn static_contract_precondition_let_bound_static_var_is_vacuous() {
    assert_eq!(
        errors_containing(
            &precondition_source("(let [a false] a)"),
            PRECONDITION_VACUOUS
        )
        .len(),
        1,
        "静的に false な値で束縛された変数を body に置いた let precondition も到達不能"
    );
}

#[test]
fn static_contract_assert_let_bound_dynamic_var_is_not_vacuous() {
    let source = "(defn checked [flag] :assert [(let [a flag] a)] flag)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "動的な値で束縛された変数を静的 true と読んではならない"
    );
}

#[test]
fn static_contract_assert_let_rebinding_shadows_outer_static_value() {
    // 内側の束縛が外側を覆う。最新の束縛 (動的) が勝たないと誤診断になる。
    let source = "(defn checked [flag] :assert [(let [a true] (let [a flag] a))] flag)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "内側の let が同名を再束縛したら外側の静的値を使ってはならない"
    );
}

#[test]
fn static_contract_assert_match_arm_binding_shadows_outer_static_value() {
    // arm pattern が同名を束縛するので、arm body の `a` は let の静的 true ではない。
    let source = "(defn checked [n] :assert [(let [a true] (match n [a a]))] true)";
    assert!(
        errors_containing(source, ASSERT_VACUOUS).is_empty(),
        "match arm の pattern が同名を束縛したら外側の静的値を使ってはならない"
    );
}

/// `I-42` の証拠表が挙げた `(match true (true true))` は arm が bracket でないため
/// **parse できない**。「診断 0 件」は穴ではなく fixture の不備だったことを固定する。
#[test]
fn static_contract_issue_table_paren_match_arm_fixture_does_not_parse() {
    let source = assert_source("(match true (true true))");
    let error = lsharp_syntax::parse(&source).expect_err("bracket でない arm は parse できない");
    let rendered = format!("{error:?}");
    assert!(
        rendered.contains('['),
        "arm の bracket 期待が出るはず: {rendered}"
    );
}
