use super::support::try_compile_and_run_with_wasm_stack_limit;

fn recursive_source(depth: usize) -> String {
    format!(
        r#"
        (defn recurse [n]
          (if (<= n 0)
            0
            (+ 1 (recurse (- n 1)))))
        (defn main []
          (print (recurse {depth})))
        "#
    )
}

#[test]
fn test_e2e_runtime_recursion_stack_limit_reports_trap() {
    for depth in [0, 32, 128] {
        let shallow =
            try_compile_and_run_with_wasm_stack_limit(&recursive_source(depth), 64 * 1024)
                .expect("低い stack limit でも浅い再帰は実行できるべき");
        assert_eq!(shallow, format!("{depth}\n"));
    }

    let error = try_compile_and_run_with_wasm_stack_limit(&recursive_source(100_000), 64 * 1024)
        .expect_err("深い再帰は Wasmtime stack limit で失敗するべき");
    let lower = error.to_ascii_lowercase();
    assert!(
        lower.contains("stack") && lower.contains("trap"),
        "再帰 stack limit の failure は stack trap を含むべき: {error}"
    );
}
