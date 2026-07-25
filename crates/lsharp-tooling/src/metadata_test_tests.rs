use super::*;
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

fn unique_temp_dir(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "lsharp_tooling_metadata_{name}_{}_{}",
        std::process::id(),
        nonce
    ));
    fs::create_dir_all(&dir).expect("temp dir creation failed");
    dir
}

#[test]
fn test_run_metadata_tests_returns_empty_when_source_has_no_metadata() {
    let dir = unique_temp_dir("empty");
    let file = dir.join("Main.ls");
    fs::write(&file, "(defn add [x y] (+ x y))\n").unwrap();

    let run = run_metadata_tests(&file).expect("metadata test helper should succeed");
    assert!(!run.has_tests(), "metadata がないので tests は空のはず");
    assert_eq!(run.total(), 0);
    assert_eq!(run.passed(), 0);
    assert_eq!(run.failed(), 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_preserves_parse_error_code() {
    let dir = unique_temp_dir("parse_diagnostic");
    let file = dir.join("Broken.ls");
    fs::write(&file, "@").expect("metadata parse diagnostic fixture write failed");

    let error =
        run_metadata_tests(&file).expect_err("不正な source は metadata test を失敗させるべき");
    assert!(
        error.to_string().contains("[LS0001]"),
        "metadata test parse diagnostics は stable code を含むべき: {error}"
    );

    fs::remove_dir_all(&dir).expect("metadata parse diagnostic directory cleanup failed");
}

#[test]
fn test_run_metadata_tests_missing_source_preserves_driver_io_error_code() {
    let dir = unique_temp_dir("missing-source");
    let file = dir.join("Missing.ls");

    let error =
        run_metadata_tests(&file).expect_err("存在しない source は metadata test を失敗させるべき");
    assert!(
        error.to_string().starts_with("[LS5001]"),
        "metadata test file I/O diagnostics は stable code を含むべき: {error}"
    );
    assert!(error.to_string().contains("Missing.ls"));

    fs::remove_dir_all(&dir).expect("metadata missing source directory cleanup failed");
}

#[test]
fn test_run_metadata_tests_executes_example_and_invariant() {
    let dir = unique_temp_dir("success");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        r#"(defn abs
  [x]
  :example [(= (abs 5) 5)]
  :invariant (>= result 0)
  (if (< x 0) (- 0 x) x))
"#,
    )
    .unwrap();

    let run = run_metadata_tests(&file).expect("metadata test helper should succeed");
    assert!(run.has_tests(), "metadata があるので tests が必要");
    assert_eq!(run.total(), 2);
    assert_eq!(run.passed(), 2);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "abs_invariant");
    assert_eq!(run.results[1].name, "abs_example_0");
    assert!(run.results.iter().all(|result| result.passed));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_canonical_assertions() {
    let dir = unique_temp_dir("canonical_assertion_execution");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        r#"(defn truth [] (= 1 1))
(defn falsehood [] (= 1 2))
(defn noop [] :assert [(truth) (falsehood)] 0)
"#,
    )
    .unwrap();

    let run =
        run_metadata_tests(&file).expect("canonical assertion は実行結果へ materialize されるべき");
    assert_eq!(run.total(), 2);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 1);
    assert_eq!(run.results[0].name, "noop_assertion_0");
    assert_eq!(run.results[1].name, "noop_assertion_1");
    assert!(run.results[0].passed);
    assert!(!run.results[1].passed);
    assert!(matches!(
        run.results[0].kind,
        lsharp_types::metadata_check::TestKind::Assertion
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_reports_unknown_invariant_variable() {
    let dir = unique_temp_dir("unknown_invariant_variable");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        r#"(defn succ
  [x]
  :invariant (= result (+ missing 1))
  (+ x 1))
"#,
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("未定義 invariant 変数は stable diagnostic で拒否されるべき");
    assert!(
        error.to_string().contains("[LS1001]"),
        "unknown invariant variable は LS1001 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_non_bool_invariant() {
    let dir = unique_temp_dir("non_bool_invariant");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        r#"(defn succ
  [x]
  :invariant (+ x 1)
  (+ x 1))
"#,
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("non-Bool invariant は stable diagnostic で拒否されるべき");
    assert!(
        error.to_string().contains("[LS1002]"),
        "non-Bool invariant は LS1002 を返すべき: {error}"
    );
    assert!(
        !error
            .to_string()
            .contains("テストプログラムの型チェックに失敗"),
        "non-Bool invariant は生成テストの後段エラーへ漏れてはならない: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_allows_local_let_binding_in_invariant() {
    let dir = unique_temp_dir("local_let_invariant");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        r#"(defn succ
  [x]
  :invariant (let [delta 1] (= result (+ x delta)))
  (+ x 1))
"#,
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("invariant の local let binding は metadata oracle で有効であるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "succ_invariant");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_empty_canonical_assertion() {
    let dir = unique_temp_dir("empty_canonical_assertion");
    let file = dir.join("Main.ls");
    fs::write(&file, "(defn noop [] :assert [] true)\n").unwrap();

    let error =
        run_metadata_tests(&file).expect_err("空の canonical assertion を成功扱いしてはならない");
    assert!(
        error.to_string().contains("[LS2004]"),
        "空の canonical assertion は LS2004 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_literal_true_canonical_assertion() {
    let dir = unique_temp_dir("literal_true_canonical_assertion");
    let file = dir.join("Main.ls");
    fs::write(&file, "(defn noop [] :assert [true] true)\n").unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("literal true canonical assertion を成功扱いしてはならない");
    assert!(
        error.to_string().contains("[LS2005]"),
        "literal true canonical assertion は LS2005 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_statically_true_integer_comparison() {
    let dir = unique_temp_dir("static_true_integer_comparison");
    let file = dir.join("Main.ls");
    fs::write(&file, "(defn noop [] :assert [(= 1 1)] true)\n").unwrap();

    let error =
        run_metadata_tests(&file).expect_err("静的に true な整数比較を成功扱いしてはならない");
    assert!(
        error.to_string().contains("[LS2005]"),
        "静的に true な整数比較は LS2005 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_mismatched_canonical_case_types() {
    let dir = unique_temp_dir("mismatched_canonical_case_types");
    let file = dir.join("Main.ls");
    fs::write(&file, "(defn noop [] :case [(expect 1 true)] true)\n").unwrap();

    let error =
        run_metadata_tests(&file).expect_err("型不一致 canonical case を成功扱いしてはならない");
    assert!(
        error.to_string().contains("[LS1002]"),
        "型不一致 canonical case は LS1002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_empty_canonical_case() {
    let dir = unique_temp_dir("empty_canonical_case");
    let file = dir.join("Main.ls");
    fs::write(&file, "(defn noop [] :case [] 0)\n").unwrap();

    let error =
        run_metadata_tests(&file).expect_err("空の canonical case を成功扱いしてはならない");
    assert!(
        error.to_string().contains("[LS2006]"),
        "空の canonical case は LS2006 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_canonical_case_parameter_capture() {
    let dir = unique_temp_dir("canonical_case_parameter_capture");
    let file = dir.join("Main.ls");
    fs::write(&file, "(defn identity [x] :case [(expect x 1)] x)\n").unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("case が owner parameter を暗黙 capture したように成功してはならない");
    assert!(
        error.to_string().contains("[LS1001]"),
        "case parameter capture は LS1001 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_deterministic_property_smoke() {
    let dir = unique_temp_dir("deterministic_property_smoke");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [x Int] :cases 5 :postcondition (= result x))] x)\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file).expect("deterministic property smoke は実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "identity_property_0");
    assert_eq!(
        run.results[0].kind,
        lsharp_types::metadata_check::TestKind::Property
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_property_precondition_and_skips_false_samples() {
    let dir = unique_temp_dir("deterministic_property_precondition");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [value Int] :cases 5 :precondition [(>= value 0)] :postcondition (= result value))] x)\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("single Int precondition property は false sample を skip して実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "identity_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_all_property_preconditions_as_conjunction() {
    let dir = unique_temp_dir("deterministic_property_preconditions");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [value Int] :cases 5 :precondition [(>= value 0) (< value 42)] :postcondition (= result value))] x)\n",
    )
    .unwrap();

    let run =
        run_metadata_tests(&file).expect("複数 precondition は conjunction として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "identity_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_two_int_property_binders() {
    let dir = unique_temp_dir("deterministic_property_two_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn sum [left right] :property [(for-all [a Int b Int] :cases 5 :precondition [(< b 5)] :postcondition (= result (+ a b)))] (+ left right))\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("二つの Int binder は deterministic pair prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "sum_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_bool_property_binder() {
    let dir = unique_temp_dir("deterministic_property_bool_binder");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [value Bool] :cases 2 :postcondition (or value (not value)))] x)\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("単一 Bool binder は false/true の deterministic prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "identity_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_mixed_int_bool_property_binders() {
    let dir = unique_temp_dir("deterministic_property_mixed_int_bool_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [input enabled] :property [(for-all [value Int flag Bool] :cases 2 :postcondition (and (>= value 0) (or flag (not flag))))] enabled)\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("Int/Bool mixed binder は deterministic typed prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "choose_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_two_bool_property_binders() {
    let dir = unique_temp_dir("deterministic_property_two_bool_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [left right] :property [(for-all [a Bool b Bool] :cases 2 :postcondition (= result (if (or a b) 1 0)))] (if (or left right) 1 0))\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("二つの Bool binder は deterministic typed prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "choose_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_three_bool_property_binders() {
    let dir = unique_temp_dir("deterministic_property_three_bool_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [left middle right] :property [(for-all [a Bool b Bool c Bool] :cases 2 :postcondition (= result (if (or a (or b c)) 1 0)))] (if (or left (or middle right)) 1 0))\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("三つの Bool binder は deterministic typed prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "choose_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_three_bool_property_above_two_cases() {
    let dir = unique_temp_dir("unsupported_three_bool_property_cases");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [left middle right] :property [(for-all [a Bool b Bool c Bool] :cases 3 :postcondition (= result (if (or a (or b c)) 1 0)))] (if (or left (or middle right)) 1 0))\n",
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("三つの Bool property の cases 3 は narrow profile 外であるべき");
    assert!(
        error.to_string().contains("[LS3002]"),
        "三つの Bool property の cases 3 は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_two_bool_property_above_two_cases() {
    let dir = unique_temp_dir("unsupported_two_bool_property_cases");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [left right] :property [(for-all [a Bool b Bool] :cases 3 :postcondition (= result (if (or a b) 1 0)))] (if (or left right) 1 0))\n",
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("二つの Bool property の cases 3 は narrow profile 外であるべき");
    assert!(
        error.to_string().contains("[LS3002]"),
        "二つの Bool property の cases 3 は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_mixed_int_bool_property_above_two_cases() {
    let dir = unique_temp_dir("unsupported_mixed_int_bool_property_cases");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [input enabled] :property [(for-all [value Int flag Bool] :cases 3 :postcondition (and (>= value 0) (or flag (not flag))))] enabled)\n",
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("Int/Bool mixed property の cases 3 は narrow profile 外であるべき");
    assert!(
        error.to_string().contains("[LS3002]"),
        "Int/Bool mixed property の cases 3 は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_three_int_property_binders() {
    let dir = unique_temp_dir("deterministic_property_three_int_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn sum3 [left middle right] :property [(for-all [a Int b Int c Int] :cases 1 :postcondition (= result (+ a (+ b c))))] (+ left (+ middle right)))\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("三つの Int binder は cases 1 の deterministic prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "sum3_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_three_int_property_binders_above_two_cases() {
    let dir = unique_temp_dir("unsupported_property_three_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn sum3 [left middle right] :property [(for-all [a Int b Int c Int] :cases 3 :postcondition (= result (+ a (+ b c))))] (+ left (+ middle right)))\n",
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("3 Int binder property の cases 3 は narrow profile 外であるべき");
    assert!(
        error.to_string().contains("[LS3002]"),
        "3 Int binder property の cases 3 は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_three_mixed_int_bool_property_binders() {
    let dir = unique_temp_dir("deterministic_property_three_mixed_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [input enabled offset] :property [(for-all [left Int flag Bool right Int] :cases 2 :postcondition (= result (if flag (+ left right) left)))] (if enabled (+ input offset) input))\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("3 binder mixed property は deterministic typed prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "choose_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_three_mixed_int_bool_property_above_two_cases() {
    let dir = unique_temp_dir("unsupported_property_three_mixed_cases");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [input enabled offset] :property [(for-all [left Int flag Bool right Int] :cases 3 :postcondition (= result (if flag (+ left right) left)))] (if enabled (+ input offset) input))\n",
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("3 binder mixed property の cases 3 は narrow profile 外であるべき");
    assert!(
        error.to_string().contains("[LS3002]"),
        "3 binder mixed property の cases 3 は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_four_mixed_int_bool_property_binders() {
    let dir = unique_temp_dir("deterministic_property_four_mixed_binders");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [left enabled right ready] :property [(for-all [first Int flag Bool second Int again Bool] :cases 2 :postcondition (= result (if (and flag again) (+ first second) first)))] (if (and enabled ready) (+ left right) left))\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file)
        .expect("4 binder mixed property は source-order typed prefix として実行できるべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 0);
    assert_eq!(run.results[0].name, "choose_property_0");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_four_mixed_int_bool_property_above_two_cases() {
    let dir = unique_temp_dir("unsupported_property_four_mixed_cases");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn choose [left enabled right ready] :property [(for-all [first Int flag Bool second Int again Bool] :cases 3 :postcondition (= result (if (and flag again) (+ first second) first)))] (if (and enabled ready) (+ left right) left))\n",
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("4 binder mixed property の cases 3 は narrow profile 外であるべき");
    assert!(
        error.to_string().contains("[LS3002]"),
        "4 binder mixed property の cases 3 は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_property_outside_deterministic_smoke_profile() {
    let dir = unique_temp_dir("unsupported_property_profile");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [x Int] :cases 6 :postcondition (= result x))] x)\n",
    )
    .unwrap();

    let error =
        run_metadata_tests(&file).expect_err("profile 外の property を成功扱いしてはならない");
    assert!(
        error.to_string().contains("[LS3002]"),
        "profile 外の property は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_rejects_bool_property_above_two_cases() {
    let dir = unique_temp_dir("unsupported_bool_property_cases");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [value Bool] :cases 3 :postcondition (or value (not value)))] x)\n",
    )
    .unwrap();

    let error = run_metadata_tests(&file)
        .expect_err("Bool property の cases 3 は narrow profile 外であるべき");
    assert!(
        error.to_string().contains("[LS3002]"),
        "Bool property の cases 3 は LS3002 を返すべき: {error}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_reports_failing_deterministic_property() {
    let dir = unique_temp_dir("failing_deterministic_property");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [x Int] :cases 1 :postcondition (= result (+ x 1)))] x)\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file).expect("failing property も結果を返すべき");
    assert_eq!(run.total(), 1);
    assert_eq!(run.passed(), 0);
    assert_eq!(run.failed(), 1);
    assert!(run.results[0].error.is_some());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_run_metadata_tests_executes_canonical_cases() {
    let dir = unique_temp_dir("canonical_case_execution");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))\n",
    )
    .unwrap();

    let run = run_metadata_tests(&file).expect("canonical case runner は実行できるべき");
    assert_eq!(run.total(), 2);
    assert_eq!(run.passed(), 1);
    assert_eq!(run.failed(), 1);
    assert_eq!(run.results[0].name, "succ_case_0");
    assert_eq!(run.results[1].name, "succ_case_1");
    assert!(run.results[0].passed);
    assert!(!run.results[1].passed);

    let _ = fs::remove_dir_all(&dir);
}
