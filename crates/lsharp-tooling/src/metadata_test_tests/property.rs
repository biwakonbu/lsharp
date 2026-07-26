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
        "(defn identity [x] :property [(for-all [value Bool] :cases 2 :postcondition (= result (if value 1 0)))] (if x 1 0))\n",
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
fn test_run_metadata_tests_rejects_vacuous_bool_property() {
    let dir = unique_temp_dir("vacuous_bool_property");
    let file = dir.join("Main.ls");
    fs::write(
        &file,
        "(defn identity [x] :property [(for-all [value Bool] :cases 2 :postcondition (or value (not value)))] x)\n",
    )
    .unwrap();

    let error =
        run_metadata_tests(&file).expect_err("vacuous Bool property を成功扱いしてはならない");
    assert!(
        error.to_string().contains("[LS2005]"),
        "vacuous Bool property は LS2005 を返すべき: {error}"
    );

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
        "(defn identity [x] :property [(for-all [value Bool] :cases 3 :postcondition (= result (if value 1 0)))] (if x 1 0))\n",
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
