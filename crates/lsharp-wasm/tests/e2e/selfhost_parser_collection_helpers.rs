use super::support::*;

#[test]
fn test_e2e_selfhost_parser_collection_helpers_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let vector_body = source
        .split("(defn vector-set-at-loop")
        .nth(1)
        .and_then(|tail| tail.split("(defn vector-set-at-rooted-v3").next())
        .expect("Parser.ls に vector-set-at loop helper が存在すること");
    let signature_body = source
        .split("(defn defn-signature-param-present-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn defn-signature-present-v3").next())
        .expect("Parser.ls に defn signature presence helper が存在すること");

    assert!(
        source.contains("(defn vector-set-at-step-v3")
            && source.contains("(defn vector-set-at-step-64-loop-bounded")
            && vector_body.contains("vector-set-at-step-64")
            && !vector_body
                .contains("(vector-set-at-loop vec next-result idx new-val (+ i 1) len)"),
        "vector 更新は Linux x86 native stack の長い vector を bounded chunk へ委譲するべき"
    );
    assert!(
        source.contains("(defn defn-signature-param-present-step-v3")
            && source.contains("(defn defn-signature-param-present-step-64-loop-bounded")
            && signature_body.contains("defn-signature-param-present-step-64")
            && !signature_body
                .contains("defn-signature-param-present-v3 signature (+ idx 1) count"),
        "defn signature presence scan は bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_collection_helpers_preserve_cross_chunk_results() {
    let vector = (0..129).fold("(vector-new 0)".to_string(), |expr, _| {
        format!("(vector-push {} 0)", expr)
    });
    let empty_signature = (0..65).fold(
        "(vector-push (vector-push (vector-new 2) 65) 65)".to_string(),
        |expr, _| format!("(vector-push {} 0)", expr),
    );
    let empty_signature = format!("(vector-push {} 0)", empty_signature);
    let present_signature = (0..64).fold(
        "(vector-push (vector-push (vector-new 2) 65) 65)".to_string(),
        |expr, _| format!("(vector-push {} 0)", expr),
    );
    let present_signature = format!("(vector-push (vector-push {} 1) 0)", present_signature);
    let harness = format!(
        r#"
(defn main []
  (let [vec {vector}
        updated (vector-set-at-rooted-v3 vec 128 42)
        empty-signature {empty_signature}
        present-signature {present_signature}]
    (do
      (print (vector-length updated))
      (print (vector-get updated 128))
      (print (defn-signature-present-v3 empty-signature))
      (print (defn-signature-present-v3 present-signature))
      0)))
"#,
        vector = vector,
        empty_signature = empty_signature,
        present_signature = present_signature,
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&format!(
            "{}\n{}",
            selfhost_parser_runtime_bundle(),
            harness
        ))
    });

    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        ["129", "42", "0", "1"],
        "vector 更新と signature presence scan は 64 要素境界を跨いでも結果を保持するべき"
    );
}
