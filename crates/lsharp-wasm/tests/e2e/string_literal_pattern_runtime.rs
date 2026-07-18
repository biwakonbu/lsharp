use super::support::*;

/// EC-M1 runtime: String literal pattern は pointer identity ではなく内容で照合する。
#[test]
fn test_e2e_string_literal_pattern_matches_by_content() {
    let output = compile_and_run(
        r#"
(defn classify [value]
  (match value
    ["same" 1]
    ["" 2]
    [_ 0]))

(defn main []
  (do
    (print (classify "same"))
    (print (classify (string-concat "sa" "me")))
    (print (classify ""))
    (print (classify "other"))
    0))
"#,
    );

    assert_eq!(
        output, "1\n1\n2\n0\n",
        "static/dynamic String は内容一致し、empty と mismatch は正しい arm を選ぶべき"
    );
}
