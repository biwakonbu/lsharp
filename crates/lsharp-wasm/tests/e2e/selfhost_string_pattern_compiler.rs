use super::support::*;

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

fn selfhost_compiler_bundle(harness: &str) -> String {
    format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    )
}

/// EC-M1 runtime: selfhost compiler は String pattern を length + byte content 比較へ落とす。
#[test]
fn test_e2e_selfhost_compiler_lowers_string_pattern_content_checks() {
    let harness = r#"
(defn count-opcode [instrs idx opcode hits]
  (if (>= idx (vector-length instrs))
    hits
    (count-opcode
      instrs
      (+ idx 1)
      opcode
      (if (= (vector-get (vector-get instrs idx) 0) opcode) (+ hits 1) hits))))

(defn main []
  (let [source "(defn classify [value] (match value [\"ab\" 1] [_ 0]))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        classify (vector-get (vector-get pair 1) 0)
        instrs (vector-get classify 2)]
    (do
      (print (count-opcode instrs 0 (op-string-length) 0))
      (print (count-opcode instrs 0 (op-string-char-at) 0))
      (print (count-opcode instrs 0 (op-i64-eq) 0))
      (print (count-opcode instrs 0 (op-i64-and) 0))
      0)))
"#;

    let output = compile_and_run(&selfhost_compiler_bundle(harness));
    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        vec!["1", "2", "3", "2"],
        "String pattern は hash でなく length と全 byte を比較するべき"
    );
}

/// EC-M1 runtime: selfhost compiler が生成した実 Wasm で String pattern を内容照合する。
#[test]
fn test_e2e_selfhost_compiler_executes_string_patterns_by_content() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(defn classify [value] (match value [\"ab\" 1] [\"\" 2] [_ 0])) (defn main [] (do (print (classify \"ab\")) (print (classify (string-concat \"a\" \"b\"))) (print (classify \"\")) (print (classify \"other\")) 0))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        wasm-bytes (build-wasm-bytes-wasi functions data)]
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
    let emitted = compile_and_run(&combined);
    let wasm_bytes = parse_printed_wasm_bytes(&emitted);
    let output = super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode(
        &wasm_bytes,
        "",
        &[],
    )
    .expect("selfhost compiler-mode String pattern module should run");

    assert_eq!(
        output.trim().lines().collect::<Vec<_>>(),
        vec!["1", "1", "2", "0"],
        "static/dynamic/empty/mismatch String は正しい arm を選ぶべき"
    );
}

/// LEGACY-LANG-02: ftable compiler も source data に頼らず nested String pattern を内容照合する。
#[test]
fn test_e2e_selfhost_ftable_compiler_executes_nested_string_patterns_from_argv() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(type (Maybe a) (Some a) None) (defn classify [value] (match value [(Some \"ab\") 1] [(Some \"\") 2] [_ 0])) (defn main [] (print (classify (Some (command-line-arg 1)))))"
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
    let emitted = compile_and_run(&combined);
    let wasm_bytes = parse_printed_wasm_bytes(&emitted);

    let run = |value: &str| {
        super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode(
            &wasm_bytes,
            "",
            &["program", value],
        )
        .expect("selfhost ftable compiler-mode nested String pattern module should run")
    };

    assert_eq!(
        run("ab"),
        "1\n",
        "dynamic argv は nested \"ab\" arm に一致するべき"
    );
    assert_eq!(
        run(""),
        "2\n",
        "empty argv は nested empty String arm に一致するべき"
    );
    assert_eq!(
        run("other"),
        "0\n",
        "mismatch argv は wildcard arm に落ちるべき"
    );
}

/// LEGACY-LANG-02: import module 内の nested String pattern を compile-file-mode で保持する。
#[test]
fn test_e2e_selfhost_compiler_mode_executes_imported_nested_string_patterns() {
    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-selfhost-string-pattern-import-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("String pattern import fixture directory を作れない");
    std::fs::write(
        app_dir.join("Patterns.ls"),
        "(module App.Patterns)\n(type (Maybe a) (Some a) None)\n(defn classify [value] (match value [(Some \"ab\") 1] [(Some \"\") 2] [_ 0]))\n",
    )
    .expect("String pattern import fixture の Patterns.ls を書けない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.Patterns)\n(defn main [] (print (classify (Some (command-line-arg 1)))))\n",
    )
    .expect("String pattern import fixture の Main.ls を書けない");

    let compiler_mode = format!(
        "{}\n(defn main [] (compile-file-mode))",
        selfhost_module("CompilerMode.ls")
    );
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
    let emitted =
        compile_and_run_with_dir_and_args(&combined, &temp_root, &["compiler", "src/App/Main.ls"]);
    let wasm_bytes = parse_printed_wasm_bytes(&emitted);

    let run = |value: &str| {
        super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode_fs(
            &wasm_bytes,
            &temp_root,
            &["program", value],
        )
        .expect("imported nested String pattern module should run")
    };

    assert_eq!(run("ab"), "1\n", "imported exact String arm に一致するべき");
    assert_eq!(run(""), "2\n", "imported empty String arm に一致するべき");
    assert_eq!(
        run("other"),
        "0\n",
        "imported mismatch は wildcard arm に落ちるべき"
    );

    std::fs::remove_dir_all(&temp_root).expect("String pattern import fixture を削除できない");
}
