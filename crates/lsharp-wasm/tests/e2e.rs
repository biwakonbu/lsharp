//! E2E テスト: L# ソースコード → Wasm コンパイル → wasmtime 実行
//!
//! examples/ ディレクトリのサンプルファイルや手書きのテストケースを
//! 完全なパイプライン（パース → 型チェック → IR → Wasm → 実行）で検証する。

use lsharp_ir::lower::Lower;
use lsharp_types::infer::Infer;

/// ソースコードをコンパイルして WASI 環境で実行し、stdout 出力を返す
fn compile_and_run(source: &str) -> String {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    run_wasi(&wasm_bytes)
}

/// ソースコードをコンパイルのみ（Wasm バイナリ生成まで）
fn compile_only(source: &str) -> Vec<u8> {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap()
}

/// Wasm バイナリを WASI 環境で実行
fn run_wasi(wasm_bytes: &[u8]) -> String {
    use wasmtime::*;
    use wasmtime_wasi::{WasiCtxBuilder, preview1::WasiP1Ctx};

    let engine = Engine::default();
    let mut linker = Linker::<WasiP1Ctx>::new(&engine);
    wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |t| t).unwrap();

    let stdout = wasmtime_wasi::pipe::MemoryOutputPipe::new(4096);
    let wasi = WasiCtxBuilder::new().stdout(stdout.clone()).build_p1();
    let mut store = Store::new(&engine, wasi);
    let module = wasmtime::Module::new(&engine, wasm_bytes).unwrap();
    let instance = linker.instantiate(&mut store, &module).unwrap();
    let start = instance.get_typed_func::<(), ()>(&mut store, "_start").unwrap();
    start.call(&mut store, ()).unwrap();

    drop(store);
    let bytes = stdout.try_into_inner().unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// === examples/ ディレクトリのサンプルファイル E2E テスト ===

#[test]
fn test_e2e_hello() {
    let source = std::fs::read_to_string("../../examples/hello.ls").unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

#[test]
fn test_e2e_factorial() {
    let source = std::fs::read_to_string("../../examples/factorial.ls").unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "3628800\n120\n1\n");
}

#[test]
fn test_e2e_fibonacci() {
    let source = std::fs::read_to_string("../../examples/fib.ls").unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "55\n");
}

#[test]
fn test_e2e_type_alias() {
    let source = std::fs::read_to_string("../../examples/type-alias.ls").unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "7\n");
}

// GC 型を含むテスト: wasmtime の GC feature が未有効のため、コンパイルのみ検証
// （GC struct 型が TypeSection に出力されるが、wasmtime がパース不可）

#[test]
fn test_e2e_adt_option_typecheck() {
    // ADT コンストラクタの IR 変換は部分実装のため、型チェックまで検証
    let source = std::fs::read_to_string("../../examples/types.ls").unwrap();
    let program = lsharp_syntax::parse(&source).unwrap();
    let mut infer = Infer::new();
    let results = infer.infer_program(&program).unwrap();
    // 型推論が成功すること
    assert!(!results.is_empty());
}

#[test]
fn test_e2e_record_compile() {
    // レコード型は GC 型を含むため、コンパイルのみ検証
    let source = std::fs::read_to_string("../../examples/record.ls").unwrap();
    let wasm = compile_only(&source);
    assert!(wasm.len() > 8);
    assert_eq!(&wasm[0..4], b"\0asm");
}

#[test]
fn test_e2e_trait() {
    let source = std::fs::read_to_string("../../examples/trait.ls").unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

// === 手書き E2E テストケース ===

#[test]
fn test_e2e_arithmetic() {
    let output = compile_and_run(
        "(defn main [] (do (print (+ 10 20)) (print (- 50 8)) (print (* 6 7)) 0))",
    );
    assert_eq!(output, "30\n42\n42\n");
}

#[test]
fn test_e2e_comparison() {
    let output = compile_and_run(
        "(defn main [] (do
           (print (if (< 1 2) 1 0))
           (print (if (> 3 2) 1 0))
           (print (if (<= 2 2) 1 0))
           (print (if (>= 2 3) 1 0))
           (print (if (== 5 5) 1 0))
           (print (if (!= 3 4) 1 0))
           0))",
    );
    assert_eq!(output, "1\n1\n1\n0\n1\n1\n");
}

#[test]
fn test_e2e_let_binding() {
    let output = compile_and_run(
        "(defn main [] (let [x 10 y 20] (print (+ x y))))",
    );
    assert_eq!(output, "30\n");
}

#[test]
fn test_e2e_nested_let() {
    let output = compile_and_run(
        "(defn main []
           (let [x 5]
             (let [y (* x 2)]
               (print (+ x y)))))",
    );
    assert_eq!(output, "15\n");
}

#[test]
fn test_e2e_recursive_function() {
    let output = compile_and_run(
        "(defn sum [n]
           (if (<= n 0) 0 (+ n (sum (- n 1)))))
         (defn main [] (print (sum 10)))",
    );
    assert_eq!(output, "55\n");
}

#[test]
fn test_e2e_multiple_functions() {
    let output = compile_and_run(
        "(defn double [x] (* x 2))
         (defn triple [x] (* x 3))
         (defn main [] (do
           (print (double 7))
           (print (triple 5))
           0))",
    );
    assert_eq!(output, "14\n15\n");
}

#[test]
fn test_e2e_pattern_match_adt_typecheck() {
    // ADT コンストラクタの IR 変換は部分実装のため、型チェックまで検証
    let source = "(type (Maybe a) (Just a) Nothing)
         (defn from-maybe [m d]
           (match m
             [(Just x) x]
             [Nothing d]))
         (defn main [] (print 42))";
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let results = infer.infer_program(&program).unwrap();
    assert!(!results.is_empty());
}

#[test]
fn test_e2e_boolean_logic() {
    let output = compile_and_run(
        "(defn main [] (do
           (print (if (and (> 3 2) (< 1 5)) 1 0))
           (print (if (or (> 1 10) (< 1 5)) 1 0))
           (print (if (not (== 1 2)) 1 0))
           0))",
    );
    assert_eq!(output, "1\n1\n1\n");
}

#[test]
fn test_e2e_equality_operator() {
    // = 演算子（== のエイリアス）のテスト
    let output = compile_and_run(
        "(defn main [] (do
           (print (if (= 42 42) 1 0))
           (print (if (= 1 2) 1 0))
           0))",
    );
    assert_eq!(output, "1\n0\n");
}

#[test]
fn test_e2e_record_update_compile() {
    // レコード型は GC 型を含むため、コンパイルのみ検証
    let wasm = compile_only(
        "(type Point (record (: x Int) (: y Int)))
         (defn main []
           (let [p {Point x 1 y 2}
                 q {p | x 10}]
             (do
               (print (Point.x q))
               (print (Point.y q))
               0)))",
    );
    assert!(wasm.len() > 8);
    assert_eq!(&wasm[0..4], b"\0asm");
}

#[test]
fn test_e2e_nested_if() {
    let output = compile_and_run(
        "(defn classify [n]
           (if (< n 0) -1
             (if (== n 0) 0 1)))
         (defn main [] (do
           (print (classify -5))
           (print (classify 0))
           (print (classify 10))
           0))",
    );
    assert_eq!(output, "-1\n0\n1\n");
}

#[test]
fn test_e2e_adt_constructor_compile() {
    // ADT コンストラクタが関数として呼べることを検証（コンパイルのみ）
    // GC 型定義を含むため wasmtime では実行不可
    let wasm = compile_only(
        "(type (Maybe a) (Just a) Nothing)
         (defn main [] (do (print (Just 42)) 0))",
    );
    assert!(wasm.len() > 8);
    assert_eq!(&wasm[0..4], b"\0asm");
}

#[test]
fn test_e2e_adt_constructor_no_args_compile() {
    // 引数なしコンストラクタ（Nothing）のコンパイルテスト
    let wasm = compile_only(
        "(type (Maybe a) (Just a) Nothing)
         (defn main [] (do (print Nothing) 0))",
    );
    assert!(wasm.len() > 8);
    assert_eq!(&wasm[0..4], b"\0asm");
}
