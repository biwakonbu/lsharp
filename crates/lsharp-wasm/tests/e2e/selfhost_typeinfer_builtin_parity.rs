use super::support::*;

fn lsharp_name_hash(text: &str) -> i64 {
    text.chars().fold(0_i64, |acc, ch| {
        acc.wrapping_mul(31).wrapping_add(i64::from(u32::from(ch)))
    })
}

/// selfhost TypeInfer の builtin 環境は、通常開発で使う基本値・I/O・collection を解決する。
#[test]
fn test_e2e_selfhost_typeinfer_builtin_environment_covers_core_development_primitives() {
    let harness = format!(
        r#"
(defn make-node [tag]
  (vector-push (vector-new 1) tag))

(defn make-var-node [name-hash]
  (vector-push (vector-push (vector-new 2) 4) name-hash))

(defn make-apply1 [func arg]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 5)
        func)
      1)
    arg))

(defn make-apply2 [func left right]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5) 5)
          func)
        2)
      left)
    right))

(defn print-result [result]
  (do
    (print (result-failed result))
    (print (ty-name (result-type result)))))

(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        subst (subst-new)
        int-node (make-node 1)
        bool-node (make-node 2)
        string-node (make-node 3)
        float-node (make-node 19)
        string-length-result
          (infer-expr
            (make-apply1 (make-var-node {string_length}) string-node)
            env subst counter)
        float-add-result
          (infer-expr
            (make-apply2 (make-var-node {float_add}) float-node float-node)
            env subst counter)
        print-result-node
          (infer-expr
            (make-apply1 (make-var-node {print}) string-node)
            env subst counter)
        not-result
          (infer-expr
            (make-apply1 (make-var-node {not}) bool-node)
            env subst counter)
        vector-length-result
          (infer-expr
            (make-apply1
              (make-var-node {vector_length})
              (make-apply1 (make-var-node {vector_new}) int-node))
            env subst counter)
        map-size-result
          (infer-expr
            (make-apply1
              (make-var-node {map_size})
              (make-apply1 (make-var-node {map_new}) (make-node 32)))
            env subst counter)
        read-file-result
          (infer-expr
            (make-apply1 (make-var-node {read_file}) string-node)
            env subst counter)
        write-file-result
          (infer-expr
            (make-apply2 (make-var-node {write_file}) string-node string-node)
            env subst counter)
        command-line-arg-result
          (infer-expr
            (make-apply1 (make-var-node {command_line_arg}) int-node)
            env subst counter)]
    (do
      (print-result string-length-result)
      (print-result float-add-result)
      (print-result print-result-node)
      (print-result not-result)
      (print-result vector-length-result)
      (print-result map-size-result)
      (print-result read-file-result)
      (print-result write-file-result)
      (print-result command-line-arg-result)
      0)))
"#,
        string_length = lsharp_name_hash("string-length"),
        float_add = lsharp_name_hash("+."),
        print = lsharp_name_hash("print"),
        not = lsharp_name_hash("not"),
        vector_length = lsharp_name_hash("vector-length"),
        vector_new = lsharp_name_hash("vector-new"),
        map_size = lsharp_name_hash("map-size"),
        map_new = lsharp_name_hash("map-new"),
        read_file = lsharp_name_hash("read-file"),
        write_file = lsharp_name_hash("write-file"),
        command_line_arg = lsharp_name_hash("command-line-arg"),
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "0", "100", "0", "400", "0", "500", "0", "200", "0", "100", "0", "100", "0", "300",
            "0", "100", "0", "300",
        ],
        "selfhost の builtin 型環境は Rust host と同じ基本 development primitive を解決するべき"
    );
}

/// selfhost TypeInfer は Rust host の型適用を要しない builtin をすべて登録する。
#[test]
fn test_e2e_selfhost_typeinfer_builtin_environment_registers_core_builtin_surface() {
    let builtin_names = [
        "+.",
        "-.",
        "*.",
        "/.",
        "print",
        "__alloc",
        "string-length",
        "string-concat",
        "string-eq",
        "print-string",
        "string-char-at",
        "substring",
        "int-to-string",
        "proc-exit",
        "vector-new",
        "vector-length",
        "vector-get",
        "vector-set",
        "vector-push",
        "map-new",
        "map-size",
        "map-insert",
        "map-get",
        "map-contains?",
        "map-remove",
        "read-file",
        "write-file",
        "write-file-bytes",
        "file-exists?",
        "command-line-args",
        "command-line-arg",
        "read-stdin",
        "root_push",
        "root_pop",
        "root_set",
        "ref-new",
        "ref-get",
        "ref-set",
        "not",
        "and",
        "or",
    ];
    let lookup_lines = builtin_names
        .iter()
        .map(|name| {
            format!(
                "(print (if (= (type-env-lookup env {}) 0) 0 1))",
                lsharp_name_hash(name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n      ");
    let harness = format!(
        r#"
(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)]
    (do
      {lookup_lines}
      0)))
"#
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        vec!["1"; builtin_names.len()],
        "selfhost の builtin 型環境は型適用を要しない Rust host builtin をすべて登録するべき"
    );
}

/// Ref builtin は型引数を保持し、get/set 間で同じ要素型を要求する。
#[test]
fn test_e2e_selfhost_typeinfer_ref_builtins_preserve_inner_type() {
    let harness = format!(
        r#"
(defn make-node [tag]
  (vector-push (vector-new 1) tag))

(defn make-var-node [name-hash]
  (vector-push (vector-push (vector-new 2) 4) name-hash))

(defn make-apply1 [func arg]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 5)
        func)
      1)
    arg))

(defn make-apply2 [func left right]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5) 5)
          func)
        2)
      left)
    right))

(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        subst (subst-new)
        string-node (make-node 3)
        int-node (make-node 1)
        ref-new-node (make-var-node {ref_new})
        ref-get-node (make-var-node {ref_get})
        ref-set-node (make-var-node {ref_set})
        ref-new-result (infer-expr (make-apply1 ref-new-node string-node) env subst counter)
        ref-new-type (result-type ref-new-result)
        ref-get-result
          (infer-expr
            (make-apply1 ref-get-node (make-apply1 ref-new-node string-node))
            env subst counter)
        ref-set-result
          (infer-expr
            (make-apply2 ref-set-node (make-apply1 ref-new-node string-node) string-node)
            env subst counter)
        ref-set-mismatch-result
          (infer-expr
            (make-apply2 ref-set-node (make-apply1 ref-new-node string-node) int-node)
            env subst counter)]
    (do
      (print (result-failed ref-new-result))
      (print (ty-tag ref-new-type))
      (print (ty-name ref-new-type))
      (print (ty-name (type-app-arg ref-new-type 0)))
      (print (result-failed ref-get-result))
      (print (ty-name (result-type ref-get-result)))
      (print (result-failed ref-set-result))
      (print (ty-name (result-type ref-set-result)))
      (print (result-failed ref-set-mismatch-result))
      0)))
"#,
        ref_new = lsharp_name_hash("ref-new"),
        ref_get = lsharp_name_hash("ref-get"),
        ref_set = lsharp_name_hash("ref-set"),
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "5", "800", "300", "0", "300", "0", "500", "1"],
        "Ref a は a を保持し、ref-set は同じ a 以外を拒否するべき"
    );
}

/// write-file-bytes は String と Vector を受け取り、Int の結果型を返す。
#[test]
fn test_e2e_selfhost_typeinfer_write_file_bytes_contract() {
    let harness = format!(
        r#"
(defn make-node [tag]
  (vector-push (vector-new 1) tag))

(defn make-var-node [name-hash]
  (vector-push (vector-push (vector-new 2) 4) name-hash))

(defn make-apply1 [func arg]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 5)
        func)
      1)
    arg))

(defn make-apply2 [func left right]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push (vector-new 5) 5)
          func)
        2)
      left)
    right))

(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        subst (subst-new)
        string-node (make-node 3)
        int-node (make-node 1)
        vector-new-node (make-var-node {vector_new})
        vector-node (make-apply1 vector-new-node int-node)
        write-file-bytes-node (make-var-node {write_file_bytes})
        valid-result
          (infer-expr
            (make-apply2 write-file-bytes-node string-node vector-node)
            env subst counter)
        mismatch-result
          (infer-expr
            (make-apply2 write-file-bytes-node int-node vector-node)
            env subst counter)]
    (do
      (print (result-failed valid-result))
      (print (ty-name (result-type valid-result)))
      (print (result-failed mismatch-result))
      0)))
"#,
        vector_new = lsharp_name_hash("vector-new"),
        write_file_bytes = lsharp_name_hash("write-file-bytes"),
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "100", "1"],
        "write-file-bytes は String -> Vector -> Int の型契約を維持するべき"
    );
}

/// substring の3引数 apply は、途中引数の型不一致を失敗値へ反映する。
#[test]
fn test_e2e_selfhost_typeinfer_three_argument_substring_builtin_reports_result_and_mismatch() {
    let harness = format!(
        r#"
(defn make-node [tag]
  (vector-push (vector-new 1) tag))

(defn make-var-node [name-hash]
  (vector-push (vector-push (vector-new 2) 4) name-hash))

(defn make-apply3 [func first second third]
  (vector-push
    (vector-push
      (vector-push
        (vector-push
          (vector-push
            (vector-push (vector-new 6) 5)
            func)
          3)
        first)
      second)
    third))

(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        subst (subst-new)
        string-node (make-node 3)
        int-node (make-node 1)
        bool-node (make-node 2)
        substring-node (make-var-node {substring})
        valid-result
          (infer-expr
            (make-apply3 substring-node string-node int-node int-node)
            env subst counter)
        mismatch-result
          (infer-expr
            (make-apply3 substring-node string-node bool-node int-node)
            env subst counter)]
    (do
      (print (result-failed valid-result))
      (print (ty-name (result-type valid-result)))
      (print (result-failed mismatch-result))
      0)))
"#,
        substring = lsharp_name_hash("substring"),
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "300", "1"],
        "substring の3引数 apply は String -> Int -> Int -> String を維持し、途中引数の不一致を拒否するべき"
    );
}

/// 引数なし apply は Unit を渡した builtin 呼び出しとして結果型を返す。
#[test]
fn test_e2e_selfhost_typeinfer_zero_argument_builtin_call_applies_unit() {
    let harness = format!(
        r#"
(defn make-var-node [name-hash]
  (vector-push (vector-push (vector-new 2) 4) name-hash))

(defn make-apply0 [func]
  (vector-push
    (vector-push
      (vector-push (vector-new 3) 5)
      func)
    0))

(defn make-apply1 [func arg]
  (vector-push
    (vector-push
      (vector-push
        (vector-push (vector-new 4) 5)
        func)
      1)
    arg))

(defn main []
  (let [counter (make-var-counter)
        env (init-builtin-env counter)
        subst (subst-new)
        map-new-node (make-var-node {map_new})
        map-size-node (make-var-node {map_size})
        command-line-args-node (make-var-node {command_line_args})
        map-size-result
          (infer-expr
            (make-apply1 map-size-node (make-apply0 map-new-node))
            env subst counter)
        command-line-args-result
          (infer-expr (make-apply0 command-line-args-node) env subst counter)]
    (do
      (print (result-failed map-size-result))
      (print (ty-name (result-type map-size-result)))
      (print (result-failed command-line-args-result))
      (print (ty-name (result-type command-line-args-result)))
      0)))
"#,
        map_new = lsharp_name_hash("map-new"),
        map_size = lsharp_name_hash("map-size"),
        command_line_args = lsharp_name_hash("command-line-args"),
    );
    let combined = format!("{}\n{}", selfhost_typeinfer_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "100", "0", "100"],
        "引数なし builtin 呼び出しは Unit 入力を消費して結果型を返すべき"
    );
}

/// Float builtin は Float 引数を要求し、Int の混在を拒否する。
#[test]
fn test_e2e_selfhost_typeinfer_float_builtin_source_contract() {
    typecheck_only("(defn main [] (+. 1.0 2.0))");
    should_fail_typecheck("(defn main [] (+. 1.0 2))");
}
