use super::support::*;

#[test]
fn test_e2e_selfhost_typeinfer_assertion_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeInferAssertions.ls");
    for name in [
        "property-skip-space-step-64-loop-bounded",
        "property-find-substring-step-64-loop-bounded",
        "property-balanced-expression-end-step-64-loop-bounded",
        "property-atom-expression-end-step-64-loop-bounded",
        "property-skip-space-rooted-v3",
        "property-find-substring-rooted-v3",
        "property-balanced-expression-end-rooted-v3",
        "property-atom-expression-end-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "TypeInferAssertions の scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_typeinfer_assertion_scanners_preserve_cross_chunk_indexes() {
    let width = 65;
    let spaces = " ".repeat(width);
    let haystack = format!("{}needle", "x".repeat(width));
    let balanced = format!("{}x{}", "(".repeat(width), ")".repeat(width));
    let atom = format!("{} ", "a".repeat(width));
    let harness = format!(
        r#"
(defn main []
  (let [skip-src "{spaces}x"
        find-src "{haystack}"
        balanced-src "{balanced}"
        atom-src "{atom}"]
    (do
      (print (property-skip-space skip-src 0 {skip_len}))
      (print (property-find-substring find-src "needle"))
      (print (property-balanced-expression-end balanced-src 0 {balanced_len} 0))
      (print (property-atom-expression-end atom-src 0 {atom_len}))
      0)))
"#,
        spaces = spaces,
        haystack = haystack,
        balanced = balanced,
        atom = atom,
        skip_len = width + 1,
        balanced_len = width * 2 + 1,
        atom_len = width + 1,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, ["65", "65", "131", "65"]);
}

#[test]
fn test_e2e_selfhost_typeinfer_property_checks_use_bounded_chunks() {
    let source = selfhost_module("TypeInferAssertions.ls");
    for name in [
        "property-binder-source-step-64-loop-bounded",
        "property-binder-source-rooted-v3",
        "property-binder-name-conflict-step-64-loop-bounded",
        "property-binder-name-conflict-rooted-v3",
        "check-property-preconditions-step-64-loop-bounded",
        "check-property-preconditions-rooted-v3",
        "property-balanced-bracket-end-step-64-loop-bounded",
        "property-balanced-bracket-end-rooted-v3",
        "property-unknown-option-step-64-loop-bounded",
        "property-unknown-option-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "TypeInferAssertions の property check は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_typeinfer_assertion_forms_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeInferAssertions.ls");
    for name in [
        "assertion-contains-param-step-64-loop-bounded",
        "assertion-contains-param-rooted-v3",
        "check-assertion-predicates-step-64-loop-bounded",
        "check-assertion-predicates-rooted-v3",
        "check-case-expectations-step-64-loop-bounded",
        "check-case-expectations-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "TypeInferAssertions の assertion/case form check は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_typeinfer_assertion_forms_preserve_cross_chunk_results() {
    let parameters = (0..65)
        .map(|index| format!("p{index}"))
        .collect::<Vec<_>>()
        .join(" ");
    let predicates = (0..65).map(|_| "true").collect::<Vec<_>>().join(" ");
    let expectations = (0..65)
        .map(|_| "(expect 1 1)")
        .collect::<Vec<_>>()
        .join(" ");
    let assertion_source = format!(
        "(defn checks [{parameters}] :assert [p64] 0)",
        parameters = parameters
    );
    let predicate_source = format!(
        "(defn predicates [] :assert [{predicates}] true)",
        predicates = predicates
    );
    let case_source = format!(
        "(defn cases [] :case [{expectations}] 0)",
        expectations = expectations
    );
    let assertion_literal = assertion_source.replace('"', "\\\"");
    let predicate_literal = predicate_source.replace('"', "\\\"");
    let case_literal = case_source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [assertion-result (check-canonical-assertions (parse-program "{assertion_literal}"))
        predicate-result (check-canonical-assertions (parse-program "{predicate_literal}"))
        case-result (check-canonical-cases (parse-program "{case_literal}"))]
    (do
      (print (vector-get assertion-result 0))
      (print (vector-get assertion-result 1))
      (print (vector-get predicate-result 0))
      (print (vector-get predicate-result 1))
      (print (vector-get case-result 0))
      (print (vector-get case-result 1))
      0)))
"#,
        assertion_literal = assertion_literal,
        predicate_literal = predicate_literal,
        case_literal = case_literal,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["1", "1001", "65", "2005", "0", "0"],
        "assertion/case form checker は64要素境界を跨いでも診断件数と先頭コードを保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_form_collections_use_bounded_rooted_chunks() {
    let source = selfhost_module("TypeInferAssertions.ls");
    for name in [
        "check-assertion-forms-step-64-loop-bounded",
        "check-assertion-forms-rooted-v3",
        "check-case-forms-step-64-loop-bounded",
        "check-case-forms-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "TypeInferAssertions の form collection check は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_typeinfer_form_collections_preserve_cross_chunk_results() {
    let assertions = (0..65)
        .map(|_| ":assert [1]")
        .collect::<Vec<_>>()
        .join(" ");
    let cases = (0..65)
        .map(|_| ":case [(expect 1 1)]")
        .collect::<Vec<_>>()
        .join(" ");
    let assertion_source = format!(
        "(defn assertions [] {assertions} true)",
        assertions = assertions
    );
    let case_source = format!("(defn cases [] {cases} 0)", cases = cases);
    let assertion_literal = assertion_source.replace('"', "\\\"");
    let case_literal = case_source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [assertion-result (check-canonical-assertions (parse-program "{assertion_literal}"))
        case-result (check-canonical-cases (parse-program "{case_literal}"))]
    (do
      (print (vector-get assertion-result 0))
      (print (vector-get assertion-result 1))
      (print (vector-get case-result 0))
      (print (vector-get case-result 1))
      0)))
"#,
        assertion_literal = assertion_literal,
        case_literal = case_literal,
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec!["65", "1002", "0", "0"],
        "assertion/case form collection は64要素境界を跨いでも件数と先頭コードを保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_typeinfer_property_checks_preserve_cross_chunk_results() {
    let binders = (0..65)
        .map(|index| format!("p{index} Int"))
        .collect::<Vec<_>>();
    let binder_payload = format!("(for-all [{}] result)", binders.join(" "));
    let expected_parameter_source = format!(
        "[{} result]",
        binders
            .iter()
            .map(|binder| {
                let (name, ty) = binder.split_once(' ').expect("binder fixture should split");
                format!("(: {name} {ty})")
            })
            .collect::<Vec<_>>()
            .join(" ")
    );
    let conflict_binders = (0..64)
        .map(|index| format!("p{index} Int"))
        .chain(std::iter::once("p0 Int".to_string()))
        .collect::<Vec<_>>();
    let conflict_payload = format!("[{}]", conflict_binders.join(" "));
    let precondition_expression = "(>= value 0)";
    let preconditions = (0..65)
        .map(|_| precondition_expression)
        .collect::<Vec<_>>()
        .join(" ");
    let precondition_payload = format!(
        "(for-all [value Int] :precondition [{}])",
        preconditions
    );
    let bracket_payload = format!("{}{}", "[".repeat(65), "]".repeat(65));
    let known_options = (0..65)
        .map(|_| ":cases 0")
        .collect::<Vec<_>>()
        .join(" ");
    let known_option_payload = format!("[seed] {known_options}");
    let unknown_option_payload = format!("[seed] {known_options} :unknown 0");
    let escaped_precondition_payload = precondition_payload.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [binder-payload "{binder_payload}"
        conflict-payload "{conflict_payload}"
        precondition-payload "{precondition_payload}"
        bracket-payload "{bracket_payload}"
        known-option-payload "{known_option_payload}"
        unknown-option-payload "{unknown_option_payload}"]
    (do
      (print (string-length (property-probe-parameter-source binder-payload)))
      (print (property-binder-name-conflict? binder-payload))
      (print (property-binder-name-conflict? conflict-payload))
      (print (check-property-precondition precondition-payload))
      (print (property-balanced-bracket-end bracket-payload 0 {bracket_len} 0))
      (print (property-unknown-option? known-option-payload))
      (print (property-unknown-option? unknown-option-payload))
      0)))
"#,
        binder_payload = binder_payload,
        conflict_payload = conflict_payload,
        precondition_payload = escaped_precondition_payload,
        bracket_payload = bracket_payload,
        known_option_payload = known_option_payload,
        unknown_option_payload = unknown_option_payload,
        bracket_len = bracket_payload.len(),
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    let expected = vec![
        expected_parameter_source.len().to_string(),
        "0".to_string(),
        "1".to_string(),
        "0".to_string(),
        bracket_payload.len().to_string(),
        "0".to_string(),
        "1".to_string(),
    ];
    assert_eq!(
        lines.iter().map(|line| line.to_string()).collect::<Vec<_>>(),
        expected
    );
}
