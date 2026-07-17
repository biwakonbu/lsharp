//! メタデータテスト実行エンジン
//!
//! canonical `:case` と legacy `:example` / `:invariant` metadata から生成された
//! テストケースをコンパイル・実行して検証する。

use lsharp_syntax::ast::{Decl, Program};
use lsharp_types::metadata_check::{GeneratedTest, TestKind};

/// テスト実行結果
#[derive(Debug, Clone)]
pub struct TestResult {
    /// テスト名
    pub name: String,
    /// テスト対象の関数名
    pub function_name: String,
    /// テスト種別
    pub kind: TestKind,
    /// 成功したか
    pub passed: bool,
    /// エラーメッセージ（失敗時）
    pub error: Option<String>,
}

/// テスト実行サマリー
#[derive(Debug, Clone)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub errors: usize,
}

/// テスト用ソースコードを生成
///
/// 元のプログラムの宣言を保持しつつ、各テストケースの検証コードを
/// main 関数として追加したプログラムを返す。
///
/// 各テストは `(print (if test_expr 1 0))` として出力される。
/// 出力の各行が "1" ならテスト成功、"0" なら失敗。
pub fn generate_test_program(original: &Program, tests: &[GeneratedTest]) -> String {
    let mut source = String::new();

    // 元のプログラムの宣言を出力（main を除く）
    for decl in &original.decls {
        let actual = unwrap_private_decl(decl);
        if let Decl::Defn { name, .. } = actual
            && name == "main"
        {
            continue;
        }
        source.push_str(&format!("{decl}\n"));
    }

    // テスト用 main 関数を生成
    if tests.is_empty() {
        source.push_str("(defn main [] 0)\n");
        return source;
    }

    // 各テストを評価して結果を print する main 関数
    source.push_str("(defn main []\n  (do\n");

    for test in tests {
        match test.kind {
            TestKind::Case => {
                let actual = format!("{}", test.expr);
                let expected = test
                    .expected
                    .as_ref()
                    .expect("canonical case test には expected value が必要");
                source.push_str(&format!("    (print (if (= {actual} {expected}) 1 0))\n"));
            }
            TestKind::Example => {
                // :example 式をそのまま評価
                // 式が真（非ゼロ）なら 1 を、偽なら 0 を print
                let expr_str = format!("{}", test.expr);
                source.push_str(&format!("    (print (if {expr_str} 1 0))\n"));
            }
            TestKind::Invariant => {
                // :invariant は `result` を参照する事後条件
                // サンプル入力を元関数の引数名へ束縛し、result も束縛して検証
                // サンプル値: 0, 1, -1, 5, 42
                let fn_name = &test.function_name;
                let param_names = find_param_names(original, fn_name);
                let invariant_str = format!("{}", test.expr);

                for sample_args in generate_sample_args(param_names.len()) {
                    let args_str = sample_args.join(" ");
                    let result_scope =
                        format!("(let [result ({fn_name} {args_str})] {invariant_str})");
                    let scoped_invariant = param_names.iter().zip(&sample_args).rev().fold(
                        result_scope,
                        |body, (param_name, sample_arg)| {
                            format!("(let [{param_name} {sample_arg}] {body})")
                        },
                    );
                    source.push_str(&format!("    (print (if {scoped_invariant} 1 0))\n"));
                }
            }
        }
    }

    source.push_str("    0))\n");
    source
}

/// 関数のパラメータ数を取得
fn find_param_count(program: &Program, fn_name: &str) -> usize {
    find_param_names(program, fn_name).len()
}

/// 関数のパラメータ名を取得
fn find_param_names(program: &Program, fn_name: &str) -> Vec<String> {
    for decl in &program.decls {
        let actual = unwrap_private_decl(decl);
        if let Decl::Defn { name, params, .. } = actual
            && name == fn_name
        {
            return params.iter().map(|param| param.name.clone()).collect();
        }
    }
    Vec::new()
}

/// Private 宣言をアンラップ
fn unwrap_private_decl(decl: &Decl) -> &Decl {
    match decl {
        Decl::Private { inner, .. } => unwrap_private_decl(inner),
        other => other,
    }
}

/// サンプル引数の組み合わせを生成
fn generate_sample_args(param_count: usize) -> Vec<Vec<String>> {
    let samples = ["0", "1", "5", "-1", "42"];

    if param_count == 0 {
        return vec![vec![]];
    }

    if param_count == 1 {
        return samples.iter().map(|s| vec![s.to_string()]).collect();
    }

    // 2引数以上の場合は代表的な組み合わせのみ
    let mut combos = Vec::new();
    for &s1 in &samples[..3] {
        for &s2 in &samples[..3] {
            let mut args = vec![s1.to_string(), s2.to_string()];
            // 3引数以上は 0 で埋める
            for _ in 2..param_count {
                args.push("0".to_string());
            }
            combos.push(args);
        }
    }
    combos
}

/// テスト結果を解析
///
/// main 関数の print 出力を解析して各テストの結果を判定する。
/// 出力の各行が "1" なら成功、"0" なら失敗。
pub fn parse_test_output(
    output: &str,
    tests: &[GeneratedTest],
    original: &Program,
) -> Vec<TestResult> {
    let lines: Vec<&str> = output.lines().collect();
    let mut results = Vec::new();
    let mut line_idx = 0;

    for test in tests {
        match test.kind {
            TestKind::Case => {
                let passed = lines
                    .get(line_idx)
                    .map(|line| line.trim() == "1")
                    .unwrap_or(false);
                let expected = test
                    .expected
                    .as_ref()
                    .expect("canonical case test には expected value が必要");
                results.push(TestResult {
                    name: test.name.clone(),
                    function_name: test.function_name.clone(),
                    kind: test.kind.clone(),
                    passed,
                    error: if passed {
                        None
                    } else {
                        Some(format!(
                            ":case が期待値と一致しません: actual={}, expected={expected}",
                            test.expr
                        ))
                    },
                });
                line_idx += 1;
            }
            TestKind::Example => {
                let passed = lines
                    .get(line_idx)
                    .map(|l| l.trim() == "1")
                    .unwrap_or(false);
                results.push(TestResult {
                    name: test.name.clone(),
                    function_name: test.function_name.clone(),
                    kind: test.kind.clone(),
                    passed,
                    error: if passed {
                        None
                    } else {
                        Some(format!(":example 式が偽を返しました: {}", test.expr))
                    },
                });
                line_idx += 1;
            }
            TestKind::Invariant => {
                let param_count = find_param_count(original, &test.function_name);
                let sample_args = generate_sample_args(param_count);
                let mut all_passed = true;
                let mut fail_msg = None;

                for args in &sample_args {
                    let passed = lines
                        .get(line_idx)
                        .map(|l| l.trim() == "1")
                        .unwrap_or(false);
                    if !passed {
                        all_passed = false;
                        fail_msg = Some(format!(
                            ":invariant が偽を返しました (入力: {})",
                            args.join(", ")
                        ));
                    }
                    line_idx += 1;
                }

                results.push(TestResult {
                    name: test.name.clone(),
                    function_name: test.function_name.clone(),
                    kind: test.kind.clone(),
                    passed: all_passed,
                    error: fail_msg,
                });
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsharp_types::metadata_check::generate_tests;

    fn compile_and_run(source: &str) -> String {
        use lsharp_ir::lower::Lower;
        use lsharp_types::infer::Infer;

        let program = lsharp_syntax::parse(source).unwrap();
        let mut infer = Infer::new();
        let type_results = infer.infer_program(&program).unwrap();
        let mut lower = Lower::new();
        let module = lower.lower_program(&program, &type_results).unwrap();
        let wasm_bytes = crate::wasi::emit_wasm_wasi(&module).unwrap();

        crate::wasi_runner::run_wasm_wasi(&wasm_bytes).unwrap()
    }

    #[test]
    fn test_example_execution_pass() {
        let source = r#"(defn add [x y] :example [(= (add 1 2) 3)] (+ x y))"#;
        let program = lsharp_syntax::parse(source).unwrap();
        let tests = generate_tests(&program);
        assert_eq!(tests.len(), 1);

        let test_source = generate_test_program(&program, &tests);
        let output = compile_and_run(&test_source);
        let results = parse_test_output(&output, &tests, &program);

        assert_eq!(results.len(), 1);
        assert!(results[0].passed, "example テストが成功するはず");
    }

    #[test]
    fn test_example_execution_fail() {
        let source = r#"(defn add [x y] :example [(= (add 1 2) 999)] (+ x y))"#;
        let program = lsharp_syntax::parse(source).unwrap();
        let tests = generate_tests(&program);

        let test_source = generate_test_program(&program, &tests);
        let output = compile_and_run(&test_source);
        let results = parse_test_output(&output, &tests, &program);

        assert_eq!(results.len(), 1);
        assert!(!results[0].passed, "example テストが失敗するはず");
    }

    #[test]
    fn test_invariant_execution_pass() {
        // abs は result >= 0 が常に成立
        let source = r#"(defn abs [x] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"#;
        let program = lsharp_syntax::parse(source).unwrap();
        let tests = generate_tests(&program);
        assert_eq!(tests.len(), 1);

        let test_source = generate_test_program(&program, &tests);
        let output = compile_and_run(&test_source);
        let results = parse_test_output(&output, &tests, &program);

        assert_eq!(results.len(), 1);
        assert!(
            results[0].passed,
            "invariant テストが成功するはず: {:?}",
            results[0].error
        );
    }

    #[test]
    fn test_invariant_execution_binds_parameter_scope() {
        let source = r#"(defn succ [x] :invariant (= result (+ x 1)) (+ x 1))"#;
        let program = lsharp_syntax::parse(source).unwrap();
        let tests = generate_tests(&program);
        assert_eq!(tests.len(), 1);

        let test_source = generate_test_program(&program, &tests);
        let output = compile_and_run(&test_source);
        let results = parse_test_output(&output, &tests, &program);

        assert_eq!(results.len(), 1);
        assert!(
            results[0].passed,
            "invariant は元関数引数 x と result の両方を参照できるべき: {:?}",
            results[0].error
        );
    }

    #[test]
    fn test_no_tests() {
        let source = r#"(defn add [x y] (+ x y))"#;
        let program = lsharp_syntax::parse(source).unwrap();
        let tests = generate_tests(&program);
        assert!(tests.is_empty());

        let test_source = generate_test_program(&program, &tests);
        let output = compile_and_run(&test_source);
        assert!(output.is_empty() || output.trim().is_empty() || output.contains("0"));
    }

    #[test]
    fn test_generate_test_program_structure() {
        let source = r#"(defn add [x y] :example [(= (add 1 2) 3)] (+ x y))"#;
        let program = lsharp_syntax::parse(source).unwrap();
        let tests = generate_tests(&program);

        let test_source = generate_test_program(&program, &tests);
        assert!(test_source.contains("defn main"));
        assert!(test_source.contains("print"));
    }
}
