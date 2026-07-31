use super::support::*;

fn wrapping_name_hash(chars: impl IntoIterator<Item = u8>) -> i64 {
    chars.into_iter().fold(0_i64, |acc, ch| {
        acc.wrapping_mul(31).wrapping_add(i64::from(ch))
    })
}

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_primitive_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "name-hash-step-64-loop-bounded",
        "name-hash-rooted-v3",
        "symbol-dot-position-step-64-loop-bounded",
        "symbol-dot-position-rooted-v3",
        "parse-int-digits-step-64-loop-bounded",
        "parse-int-digits-rooted-v3",
        "string-literal-map-hash-step-64-loop-bounded",
        "string-literal-map-hash-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の primitive scanner は {} を持つべき",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_primitive_scanners_preserve_cross_chunk_results() {
    let width = 65;
    let name_src = "n".repeat(width);
    let name_hash = wrapping_name_hash(name_src.bytes());
    let dot_src = format!("{}.", "a".repeat(width));
    let int_src = format!("{}7", "0".repeat(width - 1));
    let negative_int_src = format!("-{}", int_src);
    let escaped_literal = format!("{}\\n", "a".repeat(width - 2));
    let escaped_literal_lsharp = escaped_literal.replace('\\', "\\\\");
    let escaped_hash = wrapping_name_hash("a".repeat(width - 2).bytes().chain([10_u8]));
    let escaped_hash = if escaped_hash == 0 {
        2
    } else if escaped_hash == -1 {
        1
    } else {
        escaped_hash
    };
    let harness = format!(
        r#"
(defn main []
  (do
    (print (name-hash "{name_src}" 0 {name_len}))
    (print (symbol-dot-position "{dot_src}" 0 {dot_len}))
    (print (parse-int-from-str "{int_src}" 0 {int_len} 0))
    (print (parse-int-from-str "{negative_int_src}" 0 {negative_int_len} 0))
    (print (string-literal-map-hash "{escaped_literal_lsharp}" 0 {escaped_len}))
    0))
"#,
        name_src = name_src,
        name_len = name_src.len(),
        dot_src = dot_src,
        dot_len = dot_src.len(),
        int_src = int_src,
        int_len = int_src.len(),
        negative_int_src = negative_int_src,
        negative_int_len = negative_int_src.len(),
        escaped_literal_lsharp = escaped_literal_lsharp,
        escaped_len = escaped_literal.len(),
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        vec![
            name_hash.to_string(),
            (width as i64).to_string(),
            "7".to_string(),
            "-7".to_string(),
            escaped_hash.to_string(),
        ],
        "Parser の primitive scanner は64要素境界を跨いでも結果を保持するべき"
    );
}
