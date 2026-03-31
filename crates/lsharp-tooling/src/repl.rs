/// REPL で入力された単一式をコンパイル・実行する。
pub fn evaluate_expression(line: &str) -> miette::Result<String> {
    let source = format!("(defn main [] {})", line.trim());
    let program = lsharp_syntax::parse(&source).map_err(|e| miette::miette!("パースエラー: {e}"))?;
    let mut infer = lsharp_types::infer::Infer::new();
    let type_results = infer
        .infer_program(&program)
        .map_err(|e| miette::miette!("型エラー: {e}"))?;
    let mut lower = lsharp_ir::lower::Lower::new();
    let module = lower
        .lower_program(&program, &type_results)
        .map_err(|e| miette::miette!("IR 変換エラー: {e}"))?;
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module)
        .map_err(|e| miette::miette!("コード生成エラー: {e}"))?;
    lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes)
        .map_err(|e| miette::miette!("実行エラー: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_expression_runs_wrapped_expression() {
        let output =
            evaluate_expression("(print (+ 1 2))").expect("REPL helper should evaluate source");
        assert_eq!(output, "3\n");
    }
}
