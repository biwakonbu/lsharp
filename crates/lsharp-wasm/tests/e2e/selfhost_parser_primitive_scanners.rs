use super::support::*;

#[test]
fn test_e2e_selfhost_parser_primitive_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "name-hash-step-64-loop-bounded",
        "name-hash-rooted-v3",
        "symbol-dot-position-step-64-loop-bounded",
        "symbol-dot-position-rooted-v3",
        "parse-int-digits-from-str-step-64-loop-bounded",
        "parse-int-digits-from-str-rooted-v3",
        "string-literal-map-hash-step-64-loop-bounded",
        "string-literal-map-hash-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser primitive scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_primitive_scanners_preserve_cross_chunk_results() {
    let width = 65;
    let name_src = format!("{}b", "a".repeat(width - 1));
    let expected_name_hash = name_src.bytes().fold(0_i64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(i64::from(byte))
    });
    let dot_src = format!("{}.", "a".repeat(width - 1));
    let digits_src = format!("{}7", "0".repeat(width - 1));
    let map_src = "a".repeat(width);
    let expected_map_hash = map_src.bytes().fold(0_i64, |acc, byte| {
        acc.wrapping_mul(31).wrapping_add(i64::from(byte))
    });
    let harness = format!(
        r#"
(defn main []
  (do
    (print (name-hash "{name_src}" 0 {width}))
    (print (symbol-dot-position "{dot_src}" 0 {width}))
    (print (parse-int-digits-from-str "{digits_src}" 0 {width} 0))
    (print (parse-int-from-str "-{digits_src}" 0 {negative_end} 0))
    (print (string-literal-map-hash "{map_src}" 0 {width}))
    0))
"#,
        name_src = name_src,
        dot_src = dot_src,
        digits_src = digits_src,
        map_src = map_src,
        width = width,
        negative_end = width + 1,
    );
    let combined = format!("{}\n{}", selfhost_parser_runtime_bundle(), harness);
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            expected_name_hash.to_string(),
            "64".to_string(),
            "7".to_string(),
            "-7".to_string(),
            expected_map_hash.to_string(),
        ]
    );
}
