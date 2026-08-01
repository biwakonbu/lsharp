use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_metadata_initializers_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "source-evidence-seen-new-v3-step-64-loop-bounded",
        "source-evidence-seen-new-v3-rooted-v3",
        "source-evidence-required-fields-present-step-64-loop-bounded",
        "source-evidence-required-fields-present-rooted-v3",
        "source-review-attestation-seen-new-v3-step-64-loop-bounded",
        "source-review-attestation-seen-new-v3-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の metadata initializer は {} を持つべき",
            name
        );
    }
    for name in [
        "source-evidence-seen-new-v3-loop",
        "source-evidence-required-fields-present-loop-v3",
        "source-review-attestation-seen-new-loop-v3",
    ] {
        assert_eq!(
            source.matches(name).count(),
            0,
            "{} は旧 direct-recursive initializer として残ってはいけない",
            name
        );
    }
}

#[test]
fn test_e2e_selfhost_parser_metadata_initializers_preserve_lengths_and_required_check() {
    let harness = r#"
(defn main []
  (let [seen (source-evidence-seen-new-v3)
        seen-1 (vector-set-at-rooted-v3 seen 1 1)
        seen-2 (vector-set-at-rooted-v3 seen-1 2 1)
        seen-3 (vector-set-at-rooted-v3 seen-2 3 1)
        seen-4 (vector-set-at-rooted-v3 seen-3 4 1)
        seen-5 (vector-set-at-rooted-v3 seen-4 5 1)
        seen-6 (vector-set-at-rooted-v3 seen-5 6 1)
        seen-7 (vector-set-at-rooted-v3 seen-6 7 1)
        seen-8 (vector-set-at-rooted-v3 seen-7 8 1)
        seen-9 (vector-set-at-rooted-v3 seen-8 9 1)
        seen-10 (vector-set-at-rooted-v3 seen-9 10 1)
        seen-11 (vector-set-at-rooted-v3 seen-10 11 1)
        seen-12 (vector-set-at-rooted-v3 seen-11 12 1)
        seen-13 (vector-set-at-rooted-v3 seen-12 13 1)
        seen-14 (vector-set-at-rooted-v3 seen-13 14 1)
        seen-15 (vector-set-at-rooted-v3 seen-14 15 1)
        seen-all (vector-set-at-rooted-v3 seen-15 16 1)
        review-seen (source-review-attestation-seen-new-v3)]
    (do
      (print (vector-length seen))
      (print (source-evidence-required-fields-present-v3 seen))
      (print (source-evidence-required-fields-present-v3 seen-all))
      (print (vector-length review-seen))
      0)))
"#;
    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(
        lines,
        ["17", "0", "1", "12"],
        "Parser の metadata initializer は vector length と required-field 判定を維持するべき"
    );
}
