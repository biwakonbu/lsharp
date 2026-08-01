use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_metadata_fields_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "parse-source-evidence-fields-step-64-loop-bounded",
        "parse-source-evidence-fields-loop-rooted-v3",
        "parse-source-review-attestation-fields-step-64-loop-bounded",
        "parse-source-review-attestation-fields-loop-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の metadata field scanner は {} を持つべき",
            name
        );
    }
    assert_eq!(
        source
            .matches("(parse-source-evidence-fields-loop-v3 spans pos-ref src")
            .count(),
        1,
        "evidence field handler は旧 field loop へ直接再帰してはいけない"
    );
    assert!(
        !source
            .contains("parse-source-review-attestation-fields-loop-v3 spans pos-ref src updated"),
        "review field handler は旧 field loop へ直接再帰してはいけない"
    );
}

#[test]
fn test_e2e_selfhost_parser_metadata_fields_preserve_cross_chunk_results() {
    let evidence_fields = (0..65)
        .map(|index| format!(":subject \"subject-{}\"", index))
        .collect::<Vec<_>>()
        .join(" ");
    let review_fields = (0..65)
        .map(|index| format!(":subject-digest \"digest-{}\"", index))
        .collect::<Vec<_>>()
        .join(" ");
    let evidence_source = format!(
        "(defn evidence [] :evidence \"evidence-id\" {} 0)",
        evidence_fields
    );
    let review_source = format!("(defn review [] :review-attestation {} 0)", review_fields);
    let escape_lsharp_string = |source: &str| source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [evidence-src "{evidence_source}"
        evidence-spans (tokenize-with-spans evidence-src)
        evidence-pos (ref-new 5)
        evidence-meta (parse-defn-metadata-v3 evidence-spans evidence-pos evidence-src)
        evidence-form (vector-get (vector-get evidence-meta 5) 0)
        evidence-payload (vector-get evidence-form 1)
        review-src "{review_source}"
        review-spans (tokenize-with-spans review-src)
        review-pos (ref-new 5)
        review-meta (parse-defn-metadata-v3 review-spans review-pos review-src)
        review-form (vector-get (vector-get review-meta 5) 0)
        review-payload (vector-get review-form 1)]
    (do
      (print (vector-length evidence-payload))
      (print (p-current evidence-spans evidence-pos))
      (print (vector-length review-payload))
      (print (p-current review-spans review-pos))
      0)))
"#,
        evidence_source = escape_lsharp_string(&evidence_source),
        review_source = escape_lsharp_string(&review_source),
    );
    let output = run_parser_runtime(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["82", "10", "75", "10"],
        "Parser の evidence/review field scanner は64要素境界を跨いでも duplicate marker と body cursor を保持するべき"
    );
}
