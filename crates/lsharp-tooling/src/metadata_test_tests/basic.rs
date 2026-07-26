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
