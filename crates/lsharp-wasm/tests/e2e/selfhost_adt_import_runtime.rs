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

#[test]
fn test_e2e_selfhost_compiler_mode_imported_adt_constructor_pattern_runs() {
    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-selfhost-adt-import-runtime-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("ADT import fixture の directory を作れない");
    std::fs::write(
        app_dir.join("Shapes.ls"),
        "(module App.Shapes)\n(type (Maybe a) (Just a) Nothing)\n",
    )
    .expect("ADT import fixture の Shapes.ls を書けない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.Shapes :open :only [Just Nothing])\n(defn unwrap [value] (match value [(Just x) x] [Nothing 0]))\n(defn main [] (do (print (unwrap (Just 41))) (print (unwrap Nothing)) 0))\n",
    )
    .expect("ADT import fixture の Main.ls を書けない");

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
    let output =
        super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode_fs(
            &wasm_bytes,
            &temp_root,
            &[],
        )
        .expect("import 先 ADT pattern を含む selfhost compiler-mode module should run");

    assert_eq!(output, "41\n0\n");
    std::fs::remove_dir_all(&temp_root).expect("ADT import fixture を削除できない");
}
