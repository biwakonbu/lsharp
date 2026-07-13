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
            "0", "100", "0", "400", "0", "500", "0", "200", "0", "100", "0", "100",
            "0", "300", "0", "100", "0", "300",
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
