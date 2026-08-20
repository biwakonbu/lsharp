use super::support::*;

#[test]
fn test_e2e_selfhost_parser_collection_scanners_use_bounded_rooted_chunks() {
    let source = selfhost_module("Parser.ls");
    for name in [
        "vector-set-at-step-64-loop-bounded",
        "vector-set-at-rooted-v3",
        "defn-signature-param-present-step-64-loop-bounded",
        "defn-signature-param-present-rooted-v3",
    ] {
        assert!(
            source.contains(name),
            "Parser の collection scanner は {} を持つべき",
            name
        );
    }
}

fn run_parser_collection_harness(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

#[test]
fn test_e2e_selfhost_parser_vector_set_preserves_cross_chunk_results() {
    let harness = format!(
        r#"
(defn main []
  (let [values (tokenize-with-spans "(a a a a a a a a a a a a a a a a a a a a a a a a a a a a a a)")
        updated (vector-set-at values 64 999)]
    (do
      (print (vector-length values))
      (print (vector-length updated))
      (print (vector-get updated 64))
      0)))
"#
    );
    let output = run_parser_collection_harness(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], lines[1]);
    assert_eq!(lines[2], "999");
}

#[test]
fn test_e2e_selfhost_signature_scan_preserves_cross_chunk_results() {
    let harness = format!(
        r#"
(defn main []
  (let [signature0 (tokenize-with-spans "(a a a a a a a a a a a a a a a a a a a a a a a a a a a a a a)")
        signature1 (vector-set-at signature0 1 65)
        signature (vector-set-at signature1 66 1)]
    (do
      (print (defn-signature-param-present-v3 signature 0 65))
      (print (defn-signature-present-v3 signature))
      0)))
"#
    );
    let output = run_parser_collection_harness(&harness);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, ["1", "1"]);
}

/// `(defn <name>` から次の `(defn ` 直前までを切り出す。
///
/// 名前は完全一致で見る (`vector-set-at` が `vector-set-at-step-v3` に化けないように、
/// 直後が空白か `[` か改行であることを要求する)。
fn defn_body<'a>(source: &'a str, name: &str) -> &'a str {
    let head = format!("(defn {}", name);
    let mut cursor = 0usize;
    while let Some(rel) = source[cursor..].find(&head) {
        let start = cursor + rel;
        let rest = &source[start + head.len()..];
        let boundary = rest.chars().next().is_some_and(|c| c == ' ' || c == '\n');
        if boundary {
            let body = &source[start..];
            let end = body[1..]
                .find("\n(defn ")
                .map(|i| i + 1)
                .unwrap_or(body.len());
            return &body[..end];
        }
        cursor = start + head.len();
    }
    panic!("Parser.ls に (defn {} が存在すること", name);
}

/// bounded chunk への「委譲の配線」を pin する。
///
/// 由来: `codex/lsharp-typeinfer-property-aggregation-batch` の
/// `selfhost_parser_collection_helpers.rs`。同 branch は入口を `vector-set-at-loop` と
/// 名付け、入口から直接 `-step-64` を呼んでいた。main は入口を `vector-set-at` とし、
/// `-rooted-continuation-v3` を 1 段挟む形へ分岐している。契約 (要素ごとの自己再帰を
/// 持たず 64 要素 chunk へ委譲する) は同じなので、assertion を main の綴りへ読み替えた。
#[test]
fn test_e2e_selfhost_parser_collection_scanners_delegate_to_bounded_chunks() {
    let source = selfhost_module("Parser.ls");

    let entry = defn_body(&source, "vector-set-at");
    assert!(
        entry.contains("vector-set-at-rooted-continuation-v3"),
        "vector-set-at は rooted continuation へ委譲するべき"
    );
    assert!(
        !entry.contains("(vector-set-at vec"),
        "vector-set-at は要素ごとに自己再帰するべきでない"
    );

    let continuation = defn_body(&source, "vector-set-at-rooted-continuation-v3");
    assert!(
        continuation.contains("vector-set-at-step-64-loop-bounded"),
        "rooted continuation は 64 要素 chunk へ委譲するべき"
    );

    let step = defn_body(&source, "vector-set-at-step-v3");
    assert!(
        !step.contains("vector-set-at-rooted-continuation-v3"),
        "step helper は continuation へ戻ってはならない (chunk 境界が消える)"
    );

    let signature_entry = defn_body(&source, "defn-signature-param-present-v3");
    assert!(
        signature_entry.contains("defn-signature-param-present-rooted-v3"),
        "defn signature presence scan は rooted helper へ委譲するべき"
    );
    assert!(
        !signature_entry.contains("defn-signature-param-present-v3 signature (+ idx 1) count"),
        "defn signature presence scan は要素ごとに自己再帰するべきでない"
    );

    let signature_rooted = defn_body(&source, "defn-signature-param-present-rooted-v3");
    assert!(
        signature_rooted.contains("defn-signature-param-present-step-64-loop-bounded"),
        "defn signature presence の rooted helper は 64 要素 chunk へ委譲するべき"
    );

    let signature_step = defn_body(&source, "defn-signature-param-present-step-v3");
    assert!(
        !signature_step.contains("defn-signature-param-present-rooted-v3"),
        "step helper は rooted helper へ戻ってはならない (chunk 境界が消える)"
    );
}
