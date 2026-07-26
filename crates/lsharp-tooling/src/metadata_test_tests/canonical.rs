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
