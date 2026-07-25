use super::support::*;

fn parse_printed_wasm_bytes(output: &str) -> Vec<u8> {
    let lines: Vec<&str> = output.trim().lines().collect();
    let Some((count_text, byte_lines)) = lines.split_first() else {
        panic!("selfhost emitted wasm bytes 出力が空");
    };
    let expected_count: usize = count_text
        .parse()
        .expect("selfhost emitted wasm bytes の先頭行は長さであること");
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
                .expect("selfhost emitted wasm byte 行は整数であること");
            u8::try_from(value).expect("selfhost emitted wasm byte は 0..=255 に収まること")
        })
        .collect()
}

/// EC-M1-01: private record の literal/pattern を CompilerMode runtime へ接続すること
#[test]
fn test_e2e_selfhost_compiler_mode_private_record_literal_pattern_runs() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(private (type Secret (record (: x Int)))) (defn main [] (let [value {Secret x 41}] (print (match value [{Secret x x} x] [_ 0])) 0))"
        program (parse-program source)
        pair (compile-program-functions-with-base program 11)
        functions (vector-get pair 1)
        wasm-bytes (build-wasm-bytes-wasi functions (vector-new 0))]
    (do
      (print (vector-length wasm-bytes))
      (print-bytes-loop wasm-bytes 0 (vector-length wasm-bytes))
      0)))
"#;
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
    let emitted = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let wasm_bytes = parse_printed_wasm_bytes(&emitted);
    let output = super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode(
        &wasm_bytes,
        "",
        &[],
    )
    .expect("selfhost private record literal/pattern module should run");
    assert_eq!(output, "41\n");
}
