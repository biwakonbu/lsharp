use super::*;

fn gen_tests(source: &str) -> Vec<GeneratedTest> {
    let program = lsharp_syntax::parse(source).unwrap();
    generate_tests(&program)
}

#[test]
fn test_generate_invariant_test() {
    let tests = gen_tests(r#"(defn abs [x] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"#);
    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0].name, "abs_invariant");
    assert_eq!(tests[0].function_name, "abs");
    assert_eq!(tests[0].kind, TestKind::Invariant);
}

#[test]
fn test_generate_example_test() {
    let tests = gen_tests(r#"(defn add [x y] :example [(add 1 2)] (+ x y))"#);
    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0].name, "add_example_0");
    assert_eq!(tests[0].function_name, "add");
    assert_eq!(tests[0].kind, TestKind::Example);
}

#[test]
fn test_generate_multiple_examples() {
    let tests = gen_tests(r#"(defn add [x y] :example [(add 1 2) (add 0 0)] (+ x y))"#);
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "add_example_0");
    assert_eq!(tests[1].name, "add_example_1");
}

#[test]
fn test_generate_both_invariant_and_example() {
    let tests = gen_tests(
        r#"(defn abs [x] :invariant (>= result 0) :example [(abs 5)] (if (< x 0) (- 0 x) x))"#,
    );
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].kind, TestKind::Invariant);
    assert_eq!(tests[1].kind, TestKind::Example);
}

#[test]
fn test_generate_ordered_canonical_cases() {
    let tests =
        gen_tests(r#"(defn succ [x] :case [(expect (succ 1) 2) (expect (succ 2) 4)] (+ x 1))"#);
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "succ_case_0");
    assert_eq!(tests[0].function_name, "succ");
    assert_eq!(tests[0].kind, TestKind::Case);
    assert_eq!(
        tests[0].expected.as_ref().map(ToString::to_string),
        Some("2".to_string())
    );
    assert_eq!(tests[1].name, "succ_case_1");
    assert_eq!(tests[1].kind, TestKind::Case);
    assert_eq!(
        tests[1].expected.as_ref().map(ToString::to_string),
        Some("4".to_string())
    );
}

#[test]
fn test_generate_multiple_property_forms_have_unique_names() {
    let tests = gen_tests(
        r#"(defn identity [x]
                :property [(for-all [value Int] :cases 1 :postcondition (= result value))]
                :property [(for-all [value Int] :cases 1 :postcondition (= result (+ value 0)))]
                x)"#,
    );
    assert_eq!(tests.len(), 2);
    assert_eq!(tests[0].name, "identity_property_0");
    assert_eq!(tests[1].name, "identity_property_1");
    assert!(tests.iter().all(|test| test.kind == TestKind::Property));
}

#[test]
fn test_no_tests_without_metadata() {
    let tests = gen_tests("(defn add [x y] (+ x y))");
    assert!(tests.is_empty());
}

#[test]
fn test_no_tests_with_doc_only() {
    let tests = gen_tests(r#"(defn add [x y] :doc "adds" (+ x y))"#);
    assert!(tests.is_empty());
}

#[test]
fn test_private_function_test_generation() {
    let tests = gen_tests(r#"(private (defn helper [x] :invariant (>= result 0) (+ x 1)))"#);
    assert_eq!(tests.len(), 1);
    assert_eq!(tests[0].function_name, "helper");
}
