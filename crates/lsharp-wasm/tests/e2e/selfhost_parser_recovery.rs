use super::support::*;

#[test]
fn test_e2e_selfhost_parser_recovery_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let step_body = source
        .split("(defn recover-to-next-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn recover-to-next-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に recovery step helper が存在すること");
    let rooted_body = source
        .split("(defn recover-to-next-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn recover-to-next [").next())
        .expect("Parser.ls に recovery rooted helper が存在すること");
    let public_body = source
        .split("(defn recover-to-next [")
        .nth(1)
        .expect("Parser.ls に public recovery helper が存在すること");

    assert!(
        source.contains("(defn recover-to-next-step-v3")
            && source.contains("(defn recover-to-next-step-64-loop-bounded")
            && source.contains("(defn recover-to-next-rooted-v3")
            && rooted_body.contains("recover-to-next-step-64")
            && public_body.contains("recover-to-next-rooted-v3")
            && !step_body.contains("recover-to-next-rooted-v3"),
        "parser recovery は Linux x86 native stack の長い token 列を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_recovery_preserves_sync_points_across_chunk_boundary() {
    let symbols = (0..65).map(|_| "a").collect::<Vec<_>>().join(" ");
    let close_source = format!("{} )", symbols);
    let harness = format!(
        r#"
(defn main []
  (let [close-spans (tokenize-with-spans "{close_source}")
        close-pos (ref-new 0)
        eof-spans (tokenize-with-spans "{symbols}")
        eof-pos (ref-new 0)]
    (do
      (recover-to-next close-spans close-pos)
      (print (ref-get close-pos))
      (print (p-current close-spans close-pos))
      (recover-to-next eof-spans eof-pos)
      (print (ref-get eof-pos))
      (print (p-current eof-spans eof-pos))
      0)))
"#,
        close_source = close_source,
        symbols = symbols,
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
        ["65", "1", "65", "99"],
        "parser recovery は 64 token 境界を跨いでも閉じ括弧/EOF の同期点を残すべき"
    );
}
