use super::support::*;

const DEEP_BRANCHING_PATTERN_SOURCE: &str = "(type Tree (Leaf Int) (Node Tree Tree)) (type (Envelope a) (Packed a)) (defn sum-packed [value] (match value [(Packed (Packed (Node (Leaf x) (Leaf y)))) (+ x y)] [_ 0])) (defn main [] (print (sum-packed (Packed (Packed (Node (Leaf 40) (Leaf 2)))))))";

fn parse_printed_wasm_bytes(output: &str) -> Vec<u8> {
    let lines: Vec<&str> = output.trim().lines().collect();
    let Some((count_text, byte_lines)) = lines.split_first() else {
        panic!("selfhost emitted wasm bytes 出力が空");
    };
    let expected_count: usize = count_text
        .parse()
        .expect("selfhost emitted wasm bytes の先頭行は長さであるべき");
    assert_eq!(
        byte_lines.len(),
        expected_count,
        "selfhost emitted wasm bytes の長さと payload 行数が一致しない"
    );
    byte_lines
        .iter()
        .map(|line| {
            let value: u16 = line
                .parse()
                .expect("selfhost emitted wasm byte 行は整数であるべき");
            u8::try_from(value).expect("selfhost emitted wasm byte は 0..=255 に収まること")
        })
        .collect()
}

fn compile_deep_branching_pattern(source_aware: bool) -> Vec<u8> {
    let pair_expr = if source_aware {
        "(compile-program-functions-with-source source program)"
    } else {
        "(compile-program-functions-with-base program 11)"
    };
    let data_expr = if source_aware {
        "(vector-get pair 2)"
    } else {
        "(vector-new 0)"
    };
    let harness = format!(
        r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "{DEEP_BRANCHING_PATTERN_SOURCE}"
        program (parse-program source)
        pair {pair_expr}
        functions (vector-get pair 1)
        data {data_expr}
        wasm-bytes (build-wasm-bytes-wasi functions data)]
    (do
      (print (vector-length wasm-bytes))
      (print-bytes-loop wasm-bytes 0 (vector-length wasm-bytes))
      0)))
"#
    );
    let compiler_mode = format!("{}\n{}", selfhost_module("CompilerMode.ls"), harness);
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        selfhost_module("ModuleResolver.ls"),
        compiler_mode
    );
    parse_printed_wasm_bytes(&compile_and_run(&combined))
}

fn assert_deep_branching_pattern_runs(source_aware: bool) {
    let wasm_bytes = compile_deep_branching_pattern(source_aware);
    let output = super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode(
        &wasm_bytes,
        "",
        &[],
    )
    .expect("deep branching constructor pattern module should run");
    assert_eq!(
        output, "42\n",
        "recursive pattern scratch は ancestor value と binder local を上書きしてはならない"
    );
}

/// LEGACY-ROOT-01: source-aware compiler の深い constructor pattern で scratch/local を分離する。
#[test]
fn test_e2e_selfhost_source_pattern_scratch_survives_deep_branching_constructor() {
    assert_deep_branching_pattern_runs(true);
}

/// LEGACY-ROOT-01: ftable compiler の深い constructor pattern でも同じ local 契約を守る。
#[test]
fn test_e2e_selfhost_ftable_pattern_scratch_survives_deep_branching_constructor() {
    assert_deep_branching_pattern_runs(false);
}
