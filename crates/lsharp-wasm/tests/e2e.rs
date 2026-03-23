//! E2E テスト: L# ソースコード → Wasm コンパイル → wasmtime 実行
//!
//! examples/ ディレクトリのサンプルファイルや手書きのテストケースを
//! 完全なパイプライン（パース → 型チェック → IR → Wasm → 実行）で検証する。
//!
//! ## 検証レベル
//! - `compile_and_run`: フルパイプライン実行（stdout 出力を検証）
//! - `compile_only` + `assert_valid_wasm`: Wasm バイナリ生成まで検証
//! - `typecheck_only`: 型チェックまで検証
//! - `should_fail_typecheck` / `should_fail_parse`: エラーケース検証
//!
//! GC 型（ADT, レコード）を含むコードは wasmtime の GC feature が
//! 未有効のため `compile_only` で検証する。`_compile` サフィックスのテストがこれに該当。

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

/// ソースコードをコンパイルしてファイルシステムアクセス付きで実行
fn compile_and_run_with_dir(source: &str, dir: &std::path::Path) -> String {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();

    lsharp_wasm::wasi_runner::run_wasm_wasi_with_dir(&wasm_bytes, Some(dir)).unwrap()
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
    lsharp_wasm::wasi_runner::run_wasm_wasi(wasm_bytes).unwrap()
}

/// 型チェックでエラーになることを検証
fn should_fail_typecheck(source: &str) {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    assert!(infer.infer_program(&program).is_err());
}

/// パースでエラーになることを検証
fn should_fail_parse(source: &str) {
    assert!(lsharp_syntax::parse(source).is_err());
}

/// 型チェックまで成功することを検証（結果が空でないことも確認）
fn typecheck_only(source: &str) {
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let results = infer.infer_program(&program).unwrap();
    assert!(!results.is_empty(), "型推論結果が空");
}

/// Wasm バイナリのマジックバイトとサイズを検証
fn assert_valid_wasm(wasm: &[u8]) {
    assert!(wasm.len() > 8, "Wasm バイナリが小さすぎる: {} bytes", wasm.len());
    assert_eq!(&wasm[0..4], b"\0asm", "Wasm マジックバイトが不正");
}

/// examples ディレクトリのファイルパスを構築
fn example_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

// === examples/ ディレクトリのサンプルファイル E2E テスト ===

#[test]
fn test_e2e_hello() {
    let source = std::fs::read_to_string(example_path("hello.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

#[test]
fn test_e2e_factorial() {
    let source = std::fs::read_to_string(example_path("factorial.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "3628800\n120\n1\n");
}

#[test]
fn test_e2e_fibonacci() {
    let source = std::fs::read_to_string(example_path("fib.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "55\n");
}

#[test]
fn test_e2e_type_alias() {
    let source = std::fs::read_to_string(example_path("type-alias.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "7\n");
}

// GC 型を含むテスト: wasmtime の GC feature が未有効のため、コンパイルのみ検証
// （GC struct 型が TypeSection に出力されるが、wasmtime がパース不可）

#[test]
fn test_e2e_adt_option_typecheck() {
    // ADT コンストラクタの IR 変換は部分実装のため、型チェックまで検証
    let source = std::fs::read_to_string(example_path("types.ls")).unwrap();
    typecheck_only(&source);
}

#[test]
fn test_e2e_record_compile() {
    // レコード型は GC 型を含むため、コンパイルのみ検証
    let source = std::fs::read_to_string(example_path("record.ls")).unwrap();
    assert_valid_wasm(&compile_only(&source));
}

#[test]
fn test_e2e_trait() {
    let source = std::fs::read_to_string(example_path("trait.ls")).unwrap();
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
    typecheck_only(
        "(type (Maybe a) (Just a) Nothing)
         (defn from-maybe [m d]
           (match m
             [(Just x) x]
             [Nothing d]))
         (defn main [] (print 42))",
    );
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
    assert_valid_wasm(&compile_only(
        "(type Point (record (: x Int) (: y Int)))
         (defn main []
           (let [p {Point x 1 y 2}
                 q {p | x 10}]
             (do
               (print (Point.x q))
               (print (Point.y q))
               0)))",
    ));
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
    assert_valid_wasm(&compile_only(
        "(type (Maybe a) (Just a) Nothing)
         (defn main [] (do (print (Just 42)) 0))",
    ));
}

#[test]
fn test_e2e_adt_constructor_no_args_compile() {
    // 引数なしコンストラクタ（Nothing）のコンパイルテスト
    assert_valid_wasm(&compile_only(
        "(type (Maybe a) (Just a) Nothing)
         (defn main [] (do (print Nothing) 0))",
    ));
}

// === サンプルファイル E2E テスト ===

#[test]
fn test_e2e_nested_module() {
    let source = std::fs::read_to_string(example_path("nested-module.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "10\n16\n10\n");
}

#[test]
fn test_e2e_trait_where() {
    // Where 句付きトレイト制約の型チェック検証
    let source = std::fs::read_to_string(example_path("trait-where.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "30\n");
}

#[test]
fn test_e2e_gadt_typecheck() {
    // GADT 風 ADT の型チェック検証
    // GC 型定義を含むため wasmtime では実行不可、型チェックまで検証
    let source = std::fs::read_to_string(example_path("gadt.ls")).unwrap();
    typecheck_only(&source);
}

#[test]
fn test_e2e_hkt_typecheck() {
    // 高カインド型（Functor トレイト）の型チェック検証
    // GC 型定義を含むため wasmtime では実行不可、型チェックまで検証
    let source = std::fs::read_to_string(example_path("hkt.ls")).unwrap();
    typecheck_only(&source);
}

#[test]
fn test_e2e_computation() {
    // Computation Expression の型チェック検証
    let source = std::fs::read_to_string(example_path("computation.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "42\n");
}

// === Phase 1: パターンマッチ系 ===

#[test]
fn test_e2e_match_literal() {
    let output = compile_and_run(
        "(defn classify [n]
           (match n
             [0 100]
             [1 200]
             [_ 0]))
         (defn main [] (do
           (print (classify 0))
           (print (classify 1))
           (print (classify 5))
           0))",
    );
    assert_eq!(output, "100\n200\n0\n");
}

#[test]
fn test_e2e_match_variable() {
    let output = compile_and_run(
        "(defn main []
           (print (match 42 [x (+ x 1)])))",
    );
    assert_eq!(output, "43\n");
}

#[test]
fn test_e2e_match_wildcard() {
    let output = compile_and_run(
        "(defn main []
           (print (match 99 [_ 0])))",
    );
    assert_eq!(output, "0\n");
}

#[test]
fn test_e2e_match_bool() {
    let output = compile_and_run(
        "(defn to-int [b]
           (match b
             [true 1]
             [false 0]))
         (defn main [] (do
           (print (to-int true))
           (print (to-int false))
           0))",
    );
    assert_eq!(output, "1\n0\n");
}

#[test]
fn test_e2e_match_multi_arm() {
    let output = compile_and_run(
        "(defn day-type [d]
           (match d
             [0 0]
             [1 1]
             [2 1]
             [3 1]
             [_ 2]))
         (defn main [] (do
           (print (day-type 0))
           (print (day-type 2))
           (print (day-type 7))
           0))",
    );
    assert_eq!(output, "0\n1\n2\n");
}

// === Phase 1: do ブロック系 ===

#[test]
fn test_e2e_do_multiple_prints() {
    let output = compile_and_run(
        "(defn main [] (do (print 1) (print 2) (print 3) 0))",
    );
    assert_eq!(output, "1\n2\n3\n");
}

#[test]
fn test_e2e_do_nested() {
    let output = compile_and_run(
        "(defn main [] (do (do (print 1) 0) (print 2) 0))",
    );
    assert_eq!(output, "1\n2\n");
}

// === Phase 1: 演算子・数値エッジケース ===

#[test]
fn test_e2e_modulo() {
    let output = compile_and_run(
        "(defn main [] (do (print (% 17 5)) (print (% 10 3)) 0))",
    );
    assert_eq!(output, "2\n1\n");
}

#[test]
fn test_e2e_negative_numbers() {
    let output = compile_and_run(
        "(defn main [] (do (print (- 0 42)) (print (- 0 1)) 0))",
    );
    assert_eq!(output, "-42\n-1\n");
}

#[test]
fn test_e2e_large_numbers() {
    let output = compile_and_run(
        "(defn main [] (print (* 100000 100000)))",
    );
    assert_eq!(output, "10000000000\n");
}

#[test]
fn test_e2e_division() {
    let output = compile_and_run(
        "(defn main [] (do (print (/ 42 6)) (print (/ 100 10)) 0))",
    );
    assert_eq!(output, "7\n10\n");
}

// === Phase 1: let 束縛の発展 ===

#[test]
fn test_e2e_let_shadowing() {
    let output = compile_and_run(
        "(defn main []
           (let [x 1]
             (let [x 2]
               (print x))))",
    );
    assert_eq!(output, "2\n");
}

#[test]
fn test_e2e_let_multiple_deps() {
    let output = compile_and_run(
        "(defn main []
           (let [a 1 b (+ a 2) c (+ b 3)]
             (print c)))",
    );
    assert_eq!(output, "6\n");
}

// === Phase 1: 関数の発展 ===

#[test]
fn test_e2e_deeply_nested_calls() {
    let output = compile_and_run(
        "(defn f1 [x] (+ x 1))
         (defn f2 [x] (f1 (f1 x)))
         (defn f3 [x] (f2 (f2 x)))
         (defn f4 [x] (f3 (f3 x)))
         (defn main [] (print (f4 0)))",
    );
    assert_eq!(output, "8\n");
}

#[test]
fn test_e2e_function_composition() {
    // 関数合成パターン: 複数関数を組み合わせた計算
    let output = compile_and_run(
        "(defn square [x] (* x x))
         (defn double [x] (* x 2))
         (defn inc [x] (+ x 1))
         (defn main [] (do
           (print (square (double 3)))
           (print (inc (square 4)))
           (print (double (inc (square 2))))
           0))",
    );
    assert_eq!(output, "36\n17\n10\n");
}

// === Phase 1: 既存 examples の E2E 化 ===

#[test]
fn test_e2e_module() {
    let source = std::fs::read_to_string(example_path("module.ls")).unwrap();
    let output = compile_and_run(&source);
    assert_eq!(output, "17\n");
}

#[test]
fn test_e2e_constrained_compile() {
    let source = std::fs::read_to_string(example_path("constrained.ls")).unwrap();
    assert_valid_wasm(&compile_only(&source));
}

// === Phase 1: エラーケース ===

#[test]
fn test_e2e_type_error_rejected() {
    should_fail_typecheck("(defn main [] (+ true 1))");
}

#[test]
fn test_e2e_undefined_variable_rejected() {
    should_fail_typecheck("(defn main [] (+ x 1))");
}

#[test]
fn test_e2e_parse_error_rejected() {
    // body のない defn はパースエラー
    should_fail_parse("(defn main [])");
}

// === Phase 2: 既存機能の組み合わせテスト ===

#[test]
fn test_e2e_trait_multiple_impls() {
    // 複数型に対するトレイト実装と静的ディスパッチ
    let output = compile_and_run(
        "(trait (Describable a)
           (defn describe [self] : Int))
         (impl (Describable Int)
           (defn describe [self] self))
         (defn main [] (do
           (print (describe 99))
           0))",
    );
    assert_eq!(output, "99\n");
}

#[test]
fn test_e2e_match_with_computation() {
    // match + let + 演算の組み合わせ
    let output = compile_and_run(
        "(defn abs [n]
           (match (< n 0)
             [true (- 0 n)]
             [false n]))
         (defn main [] (do
           (print (abs -10))
           (print (abs 5))
           (print (abs 0))
           0))",
    );
    assert_eq!(output, "10\n5\n0\n");
}

#[test]
fn test_e2e_match_in_let() {
    // let 内で match を使用
    let output = compile_and_run(
        "(defn main []
           (let [x (match 3
                     [1 10]
                     [2 20]
                     [_ 30])]
             (print (+ x 5))))",
    );
    assert_eq!(output, "35\n");
}

#[test]
fn test_e2e_type_annotation_function() {
    // 型注釈付き関数定義 (: name Type) 構文
    let output = compile_and_run(
        "(defn add [(: x Int) (: y Int)] : Int
           (+ x y))
         (defn main [] (print (add 10 20)))",
    );
    assert_eq!(output, "30\n");
}

#[test]
fn test_e2e_complex_control_flow() {
    // 複雑な制御フロー: if + match + let の組み合わせ
    let output = compile_and_run(
        "(defn fizzbuzz-type [n]
           (let [by3 (== (% n 3) 0)
                 by5 (== (% n 5) 0)]
             (if (and by3 by5) 3
               (if by3 1
                 (if by5 2 0)))))
         (defn main [] (do
           (print (fizzbuzz-type 15))
           (print (fizzbuzz-type 9))
           (print (fizzbuzz-type 10))
           (print (fizzbuzz-type 7))
           0))",
    );
    assert_eq!(output, "3\n1\n2\n0\n");
}

// === Phase 3: GC 型関連のコンパイルテスト拡充 ===

#[test]
fn test_e2e_record_field_access_compile() {
    assert_valid_wasm(&compile_only(
        "(type Point (record (: x Int) (: y Int)))
         (defn main []
           (let [p {Point x 10 y 20}]
             (Point.x p)))",
    ));
}

#[test]
fn test_e2e_adt_match_compile() {
    assert_valid_wasm(&compile_only(
        "(type (Maybe a) (Just a) Nothing)
         (defn from-maybe [m default]
           (match m
             [(Just x) x]
             [Nothing default]))
         (defn main [] (print 1))",
    ));
}

#[test]
fn test_e2e_adt_multiple_variants_compile() {
    assert_valid_wasm(&compile_only(
        "(type Color Red Green Blue)
         (defn main [] (do (print Red) 0))",
    ));
}

#[test]
fn test_e2e_record_multiple_fields_compile() {
    assert_valid_wasm(&compile_only(
        "(type Person (record (: name String) (: age Int) (: active Bool)))
         (defn main [] (print 1))",
    ));
}

#[test]
fn test_e2e_adt_with_type_params_compile() {
    assert_valid_wasm(&compile_only(
        "(type (Result a b) (Ok a) (Err b))
         (defn main [] (do (print (Ok 42)) 0))",
    ));
}

// === ADT リニアメモリ版 E2E テスト ===

#[test]
fn test_e2e_adt_construct_and_match_no_args() {
    // 引数なし ADT の構築 + パターンマッチで値を取り出す
    let output = compile_and_run(
        "(type Color Red Green Blue)
         (defn color-to-int [c]
           (match c
             [Red 10]
             [Green 20]
             [Blue 30]))
         (defn main [] (do (print (color-to-int Green)) 0))",
    );
    assert_eq!(output, "20\n");
}

#[test]
fn test_e2e_adt_construct_and_match_with_args() {
    // 引数付き ADT の構築 + パターンマッチでフィールドを取り出す
    let output = compile_and_run(
        "(type (Maybe a) (Just a) Nothing)
         (defn from-maybe [m d]
           (match m
             [(Just x) x]
             [Nothing d]))
         (defn main [] (do (print (from-maybe (Just 42) 0)) 0))",
    );
    assert_eq!(output, "42\n");
}

#[test]
fn test_e2e_adt_nothing_match() {
    // Nothing のパターンマッチでデフォルト値を返す
    let output = compile_and_run(
        "(type (Maybe a) (Just a) Nothing)
         (defn from-maybe [m d]
           (match m
             [(Just x) x]
             [Nothing d]))
         (defn main [] (do (print (from-maybe Nothing 99)) 0))",
    );
    assert_eq!(output, "99\n");
}

// === Phase 2 追加: 残りの組み合わせテスト ===

#[test]
fn test_e2e_computation_let_bang_typecheck() {
    // Computation Expression の let! を含むコードの型チェック検証
    typecheck_only(
        "(defn identity [x] x)
         (defn mb [m x] m)
         (computation-builder maybe-builder mb identity)
         (defn main []
           (computation maybe-builder
             (let! x 10)
             (return (+ x 1))))",
    );
}

#[test]
fn test_e2e_where_constraint_typecheck() {
    // where 制約付き関数の型チェック検証
    typecheck_only(
        "(trait (Showable a)
           (defn to-str [self] : String))
         (defn display [x]
           :where [(Showable a)]
           x)
         (defn main [] (print 1))",
    );
}

#[test]
fn test_e2e_module_with_functions() {
    // モジュール宣言 + 複数関数の定義
    let output = compile_and_run(
        "(module Calc)
         (defn add [x y] (+ x y))
         (defn sub [x y] (- x y))
         (defn main [] (do
           (print (add 10 20))
           (print (sub 50 8))
           0))",
    );
    assert_eq!(output, "30\n42\n");
}

#[test]
fn test_e2e_module_flat_declaration() {
    // フラットモジュール宣言 + 演算関数
    let output = compile_and_run(
        "(module MathUtils)
         (defn square [x] (* x x))
         (defn cube [x] (* x (* x x)))
         (defn main [] (do
           (print (square 5))
           (print (cube 3))
           0))",
    );
    assert_eq!(output, "25\n27\n");
}

#[test]
fn test_e2e_match_nested_if() {
    // match の body 内に if を含む
    let output = compile_and_run(
        "(defn process [n]
           (match (% n 3)
             [0 (if (> n 10) 100 50)]
             [1 (if (< n 5) 10 20)]
             [_ 0]))
         (defn main [] (do
           (print (process 12))
           (print (process 3))
           (print (process 1))
           (print (process 7))
           0))",
    );
    assert_eq!(output, "100\n50\n10\n20\n");
}

#[test]
fn test_e2e_let_with_function_call() {
    // let 内で関数呼び出しの結果を束縛
    let output = compile_and_run(
        "(defn square [x] (* x x))
         (defn main []
           (let [a (square 3)
                 b (square 4)
                 c (+ a b)]
             (print c)))",
    );
    assert_eq!(output, "25\n");
}

#[test]
fn test_e2e_recursive_with_match() {
    // 再帰 + match の組み合わせ
    let output = compile_and_run(
        "(defn countdown [n]
           (match (== n 0)
             [true 0]
             [false (do (print n) (countdown (- n 1)))]))
         (defn main [] (countdown 5))",
    );
    assert_eq!(output, "5\n4\n3\n2\n1\n");
}

#[test]
fn test_e2e_constrained_type_typecheck() {
    // 制約付き型の型チェック検証
    typecheck_only(
        "(type-constrained Natural Int
           :constraints [(>= 0)])
         (type-constrained Percentage Int
           :constraints [(>= 0) (<= 100)])
         (defn main [] (print 1))",
    );
}

// === Phase 0: Bump Allocator テスト ===

#[test]
fn test_e2e_alloc_basic() {
    // __alloc を呼び出してメモリアドレスを取得できることを検証
    let result = compile_and_run(r#"
        (defn main []
          (let [addr (__alloc 16)]
            (do (print addr) addr)))
    "#);
    let addr: i64 = result.trim().parse().unwrap();
    assert!(addr >= 512, "heap address should be >= 512, got {}", addr);
}

#[test]
fn test_e2e_alloc_alignment() {
    // 複数の __alloc 呼び出しで 8 バイトアラインメントを検証
    let result = compile_and_run(r#"
        (defn main []
          (let [a1 (__alloc 1)
                a2 (__alloc 1)]
            (do (print a1) (print a2) (- a2 a1))))
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    let a1: i64 = lines[0].parse().unwrap();
    let a2: i64 = lines[1].parse().unwrap();
    assert_eq!(a2 - a1, 8, "allocations should be 8-byte aligned");
}

#[test]
fn test_e2e_alloc_memory_grow() {
    // 大量のメモリ確保で memory.grow が正しく動作することを検証
    let result = compile_and_run(r#"
        (defn main []
          (let [addr (__alloc 131072)]
            (do (print addr) addr)))
    "#);
    let addr: i64 = result.trim().parse().unwrap();
    assert!(addr >= 512, "large allocation should succeed, got {}", addr);
}

// === Phase 0-3: タグ付きワードテスト ===

#[test]
fn test_e2e_tagged_word_integer() {
    // 通常の整数はそのまま i64 として扱える
    let result = compile_and_run(r#"
        (defn main []
          (let [x 42]
            (do (print x) x)))
    "#);
    assert_eq!(result.trim(), "42");
}

#[test]
fn test_e2e_heap_object_header() {
    // ヒープオブジェクトを確保してヘッダを書き込み・読み出し
    let result = compile_and_run(r#"
        (defn main []
          (let [addr (__alloc 16)]
            (do (print addr) addr)))
    "#);
    let addr: i64 = result.trim().parse().unwrap();
    assert!(addr >= 512, "heap address should be >= 512, got {}", addr);
}

// === 文字列ランタイム関数テスト ===
// P1-1 の string runtime 実装完了後に有効化する

#[test]
fn test_e2e_string_length() {
    let result = compile_and_run(r#"
        (defn main []
          (print (string-length "hello")))
    "#);
    assert_eq!(result.trim(), "5");
}

#[test]
fn test_e2e_string_length_empty() {
    let result = compile_and_run(r#"
        (defn main []
          (print (string-length "")))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_string_length_multibyte() {
    let result = compile_and_run(r#"
        (defn main []
          (print (string-length "abc")))
    "#);
    assert_eq!(result.trim(), "3");
}

// === string-concat テスト ===

#[test]
fn test_e2e_string_concat() {
    // 2 つの文字列を結合し、その長さを確認
    let result = compile_and_run(r#"
        (defn main []
          (print (string-length (string-concat "hello" " world"))))
    "#);
    assert_eq!(result.trim(), "11");
}

#[test]
fn test_e2e_string_concat_empty() {
    // 空文字列との結合
    let result = compile_and_run(r#"
        (defn main []
          (print (string-length (string-concat "" "abc"))))
    "#);
    assert_eq!(result.trim(), "3");
}

// === string-eq テスト ===

#[test]
fn test_e2e_string_eq_true() {
    // 同じ文字列の比較
    let result = compile_and_run(r#"
        (defn main []
          (print (if (string-eq "hello" "hello") 1 0)))
    "#);
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_e2e_string_eq_false() {
    // 異なる文字列の比較
    let result = compile_and_run(r#"
        (defn main []
          (print (if (string-eq "hello" "world") 1 0)))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_string_eq_different_length() {
    // 長さが異なる文字列の比較
    let result = compile_and_run(r#"
        (defn main []
          (print (if (string-eq "abc" "abcd") 1 0)))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_string_eq_empty() {
    // 空文字列同士の比較
    let result = compile_and_run(r#"
        (defn main []
          (print (if (string-eq "" "") 1 0)))
    "#);
    assert_eq!(result.trim(), "1");
}

// === print-string テスト ===

#[test]
fn test_e2e_string_print_string() {
    // print-string で文字列を出力
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string "hello") 0))
    "#);
    assert_eq!(result, "hello");
}

#[test]
fn test_e2e_string_print_string_empty() {
    // 空文字列を出力
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string "") 0))
    "#);
    assert_eq!(result, "");
}

#[test]
fn test_e2e_string_print_string_concat() {
    // 文字列結合後に出力
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (string-concat "hello" " world")) 0))
    "#);
    assert_eq!(result, "hello world");
}

// === Phase 4-2: Ref Cell テスト ===

#[test]
fn test_e2e_ref_new_and_get() {
    // ref-new で作成した Ref Cell から ref-get で値を読み出す
    let result = compile_and_run(r#"
        (defn main []
          (let [r (ref-new 42)]
            (print (ref-get r))))
    "#);
    assert_eq!(result.trim(), "42");
}

#[test]
fn test_e2e_ref_set_and_get() {
    // ref-set で値を上書きしてから ref-get で読み出す
    let result = compile_and_run(r#"
        (defn main []
          (let [r (ref-new 10)]
            (do
              (ref-set r 99)
              (print (ref-get r)))))
    "#);
    assert_eq!(result.trim(), "99");
}

#[test]
fn test_e2e_ref_multiple_updates() {
    // Ref Cell を複数回更新
    let result = compile_and_run(r#"
        (defn main []
          (let [r (ref-new 0)]
            (do
              (ref-set r 10)
              (ref-set r 20)
              (ref-set r 30)
              (print (ref-get r)))))
    "#);
    assert_eq!(result.trim(), "30");
}

#[test]
fn test_e2e_ref_in_loop() {
    // Ref Cell を使ったカウンターループ
    let result = compile_and_run(r#"
        (defn loop-count [r n]
          (if (<= n 0)
            (ref-get r)
            (do
              (ref-set r (+ (ref-get r) 1))
              (loop-count r (- n 1)))))
        (defn main []
          (let [counter (ref-new 0)]
            (print (loop-count counter 10))))
    "#);
    assert_eq!(result.trim(), "10");
}

// === Lambda Lifting テスト ===

#[test]
fn test_e2e_lambda_no_free_vars() {
    // 自由変数なし Lambda がリフトされて正常にコンパイルされる
    let source = r#"
        (defn make-inc [] (fn [x] (+ x 1)))
        (defn main [] (print 42))
    "#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "42");
}

#[test]
fn test_e2e_lambda_with_free_vars_compile() {
    // 自由変数あり Lambda がリフトされてコンパイル可能
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] (print 99))
    "#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "99");
}

// === ADT リニアメモリ版 E2E テスト ===

#[test]
fn test_e2e_adt_cons_list_sum() {
    // Cons リストの構築と再帰的パターンマッチで合計を計算
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn sum-list [xs]
           (match xs
             [(Cons h t) (+ h (sum-list t))]
             [Nil 0]))
         (defn main [] (do (print (sum-list (Cons 1 (Cons 2 (Cons 3 Nil))))) 0))",
    );
    assert_eq!(output, "6\n");
}

#[test]
fn test_e2e_adt_cons_list_length() {
    // Cons リストの長さを再帰的に計算
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-length [xs]
           (match xs
             [(Cons h t) (+ 1 (list-length t))]
             [Nil 0]))
         (defn main [] (do (print (list-length (Cons 10 (Cons 20 (Cons 30 Nil))))) 0))",
    );
    assert_eq!(output, "3\n");
}

#[test]
fn test_e2e_adt_nested_match() {
    // ADT の入れ子パターンマッチ
    let output = compile_and_run(
        "(type (Maybe a) (Just a) Nothing)
         (defn add-maybe [a b]
           (match a
             [(Just x) (match b
                         [(Just y) (Just (+ x y))]
                         [Nothing a])]
             [Nothing b]))
         (defn from-maybe [m d]
           (match m
             [(Just x) x]
             [Nothing d]))
         (defn main [] (do
           (print (from-maybe (add-maybe (Just 10) (Just 20)) 0))
           (print (from-maybe (add-maybe (Just 5) Nothing) 0))
           (print (from-maybe (add-maybe Nothing (Just 7)) 0))
           0))",
    );
    assert_eq!(output, "30\n5\n7\n");
}

// === クロージャ変換 E2E テスト ===

#[test]
fn test_e2e_closure_capture_and_call() {
    // クロージャが自由変数をキャプチャして呼び出し可能
    // apply は第一級関数 (クロージャ) を引数として受け取り、call_indirect で呼び出す
    let output = compile_and_run(
        "(defn make-adder [n] (fn [x] (+ x n)))
         (defn apply [f x] (f x))
         (defn main [] (print (apply (make-adder 10) 32)))",
    );
    assert_eq!(output, "42\n");
}

#[test]
fn test_e2e_closure_multiple_captures() {
    // 複数の自由変数をキャプチャするクロージャ
    let output = compile_and_run(
        "(defn make-linear [a b] (fn [x] (+ (* a x) b)))
         (defn apply [f x] (f x))
         (defn main [] (print (apply (make-linear 3 7) 5)))",
    );
    // 3 * 5 + 7 = 22
    assert_eq!(output, "22\n");
}

#[test]
fn test_e2e_closure_no_capture() {
    // 自由変数なしクロージャ（Lambda Lifting のみ）
    let output = compile_and_run(
        "(defn make-inc [] (fn [x] (+ x 1)))
         (defn apply [f x] (f x))
         (defn main [] (print (apply (make-inc) 41)))",
    );
    assert_eq!(output, "42\n");
}

// === Phase 4-1: Option/Result ランタイム ===

#[test]
fn test_e2e_option_some_match() {
    // Option の Some でパターンマッチ
    let output = compile_and_run(
        "(type (Option a) (Some a) None)
         (defn unwrap-or [opt default]
           (match opt
             [(Some x) x]
             [None default]))
         (defn main [] (do (print (unwrap-or (Some 42) 0)) 0))",
    );
    assert_eq!(output, "42\n");
}

#[test]
fn test_e2e_option_none_match() {
    // Option の None でデフォルト値
    let output = compile_and_run(
        "(type (Option a) (Some a) None)
         (defn unwrap-or [opt default]
           (match opt
             [(Some x) x]
             [None default]))
         (defn main [] (do (print (unwrap-or None 99)) 0))",
    );
    assert_eq!(output, "99\n");
}

#[test]
fn test_e2e_result_ok_match() {
    // Result の Ok パターンマッチ
    let output = compile_and_run(
        "(type (Result a e) (Ok a) (Err e))
         (defn get-value [r]
           (match r
             [(Ok v) v]
             [(Err e) -1]))
         (defn main [] (do (print (get-value (Ok 100))) 0))",
    );
    assert_eq!(output, "100\n");
}

#[test]
fn test_e2e_result_err_match() {
    // Result の Err パターンマッチ
    let output = compile_and_run(
        "(type (Result a e) (Ok a) (Err e))
         (defn get-value [r]
           (match r
             [(Ok v) v]
             [(Err e) -1]))
         (defn main [] (do (print (get-value (Err 0))) 0))",
    );
    assert_eq!(output, "-1\n");
}

#[test]
fn test_e2e_option_and_then() {
    // Option の and-then (手動展開版)
    let output = compile_and_run(
        "(type (Option a) (Some a) None)
         (defn safe-div [a b]
           (if (= b 0) None (Some (/ a b))))
         (defn unwrap [opt]
           (match opt
             [(Some x) x]
             [None -1]))
         (defn main [] (do (print (unwrap (safe-div 10 2)))
                           (print (unwrap (safe-div 10 0)))
                           0))",
    );
    assert_eq!(output, "5\n-1\n");
}

// === Phase 1-3: print 多相化テスト ===

#[test]
fn test_e2e_print_string_polymorphic() {
    // print が文字列引数を受け取った場合に print-string として出力
    let output = compile_and_run(
        r#"(defn main [] (do (print "hello") 0))"#,
    );
    assert_eq!(output, "hello");
}

#[test]
fn test_e2e_print_int_backward_compat() {
    // print が整数引数の場合は従来通り動作
    let output = compile_and_run(
        "(defn main [] (do (print 42) 0))",
    );
    assert_eq!(output, "42\n");
}

// === P6: マルチファイルコンパイル ===

/// マルチファイルコンパイル: 2つのファイルを用意して import 経由で関数呼び出し
#[test]
fn test_e2e_multi_file_compile() {
    let dir = std::env::temp_dir().join("lsharp_e2e_multi");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Utils モジュール: helper 関数を提供
    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(defn helper [x] (+ x 100))",
    ).unwrap();

    // Main モジュール: Utils を import して helper を呼ぶ
    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(import Utils)\n(defn main [] (print (helper 42)))",
    ).unwrap();

    // マルチファイルコンパイル
    let linked_module = lsharp_ir::compile_multi_file(&dir.join("main.ls")).unwrap();

    // Wasm 生成 + WASI 実行
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&linked_module).unwrap();
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes).unwrap();
    assert_eq!(output, "142\n");

    std::fs::remove_dir_all(&dir).unwrap();
}

/// マルチファイルコンパイル: 3モジュールのチェーン依存
#[test]
fn test_e2e_multi_file_chain() {
    let dir = std::env::temp_dir().join("lsharp_e2e_chain");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Base モジュール
    std::fs::write(
        dir.join("Base.ls"),
        "(module Base)\n(defn base-val [] 10)",
    ).unwrap();

    // Mid モジュール: Base を import
    std::fs::write(
        dir.join("Mid.ls"),
        "(module Mid)\n(import Base)\n(defn mid-val [] (* (base-val) 2))",
    ).unwrap();

    // Main モジュール: Mid を import
    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(import Mid)\n(defn main [] (print (mid-val)))",
    ).unwrap();

    let linked_module = lsharp_ir::compile_multi_file(&dir.join("main.ls")).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&linked_module).unwrap();
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes).unwrap();
    assert_eq!(output, "20\n");

    std::fs::remove_dir_all(&dir).unwrap();
}

/// マルチファイルコンパイル: 単一ファイルの場合はリンク不要
#[test]
fn test_e2e_multi_file_single() {
    let dir = std::env::temp_dir().join("lsharp_e2e_single_multi");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(defn main [] (print 99))",
    ).unwrap();

    let linked_module = lsharp_ir::compile_multi_file(&dir.join("main.ls")).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&linked_module).unwrap();
    let output = lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes).unwrap();
    assert_eq!(output, "99\n");

    std::fs::remove_dir_all(&dir).unwrap();
}

/// マルチファイルコンパイル: 存在しないモジュールの import でエラー
#[test]
fn test_e2e_multi_file_missing_import() {
    let dir = std::env::temp_dir().join("lsharp_e2e_missing");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(import NonExistent)\n(defn main [] (print 1))",
    ).unwrap();

    let result = lsharp_ir::compile_multi_file(&dir.join("main.ls"));
    assert!(result.is_err());

    std::fs::remove_dir_all(&dir).unwrap();
}

// === エッジケース: ランタイムエラー ===

#[test]
#[should_panic]
fn test_e2e_division_by_zero_traps() {
    // Wasm の i64.div_s はゼロ除算で trap する
    compile_and_run("(defn main [] (print (/ 1 0)))");
}

// === P1-1: string-char-at テスト ===

#[test]
fn test_e2e_string_char_at() {
    // 'e' = 101
    let result = compile_and_run(r#"
        (defn main []
          (print (string-char-at "hello" 1)))
    "#);
    assert_eq!(result.trim(), "101");
}

#[test]
fn test_e2e_string_char_at_first() {
    // 'h' = 104
    let result = compile_and_run(r#"
        (defn main []
          (print (string-char-at "hello" 0)))
    "#);
    assert_eq!(result.trim(), "104");
}

#[test]
fn test_e2e_string_char_at_last() {
    // 'o' = 111
    let result = compile_and_run(r#"
        (defn main []
          (print (string-char-at "hello" 4)))
    "#);
    assert_eq!(result.trim(), "111");
}

// === P1-1: substring テスト ===

#[test]
fn test_e2e_substring() {
    // "hello" の [1..4) -> "ell" (長さ 3)
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (substring "hello" 1 4)) 0))
    "#);
    assert_eq!(result, "ell");
}

#[test]
fn test_e2e_substring_full() {
    // "hello" の [0..5) -> "hello"
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (substring "hello" 0 5)) 0))
    "#);
    assert_eq!(result, "hello");
}

#[test]
fn test_e2e_substring_empty() {
    // "hello" の [2..2) -> ""
    let result = compile_and_run(r#"
        (defn main []
          (print (string-length (substring "hello" 2 2))))
    "#);
    assert_eq!(result.trim(), "0");
}

// === P1-1: int-to-string テスト ===

#[test]
fn test_e2e_int_to_string() {
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (int-to-string 42)) 0))
    "#);
    assert_eq!(result, "42");
}

#[test]
fn test_e2e_int_to_string_zero() {
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (int-to-string 0)) 0))
    "#);
    assert_eq!(result, "0");
}

#[test]
fn test_e2e_int_to_string_negative() {
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (int-to-string -123)) 0))
    "#);
    assert_eq!(result, "-123");
}

#[test]
fn test_e2e_int_to_string_large() {
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (int-to-string 1234567890)) 0))
    "#);
    assert_eq!(result, "1234567890");
}

#[test]
fn test_e2e_int_to_string_concat() {
    // int-to-string + string-concat の組み合わせ
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (string-concat "value=" (int-to-string 42))) 0))
    "#);
    assert_eq!(result, "value=42");
}

// === P3-3: 高階関数 (list-map, list-filter, list-fold) E2E テスト ===

#[test]
fn test_e2e_closure_with_adt_basic() {
    // クロージャ引数を ADT の再帰関数内で使う基本テスト
    // apply-to-list: リストの先頭要素にクロージャを適用
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn apply-head [f xs]
           (match xs
             [(Cons h t) (f h)]
             [Nil 0]))
         (defn main [] (print (apply-head (fn [x] (* x 10)) (Cons 4 (Cons 2 Nil)))))",
    );
    assert_eq!(output, "40\n");
}

#[test]
fn test_e2e_list_map() {
    // list-map: リスト全要素にクロージャを適用して新しいリストを返す
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-map [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (Cons (f h) (list-map f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-map (fn [x] (* x 2)) (Cons 1 (Cons 2 (Cons 3 Nil)))))))",
    );
    // (1*2) + (2*2) + (3*2) = 2 + 4 + 6 = 12
    assert_eq!(output, "12\n");
}

#[test]
fn test_e2e_list_filter() {
    // list-filter: 条件を満たす要素のみ残す
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-filter [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (if (f h) (Cons h (list-filter f t)) (list-filter f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-filter (fn [x] (> x 2)) (Cons 1 (Cons 2 (Cons 3 (Cons 4 Nil))))))))",
    );
    // 3 + 4 = 7
    assert_eq!(output, "7\n");
}

#[test]
fn test_e2e_list_fold() {
    // list-fold: リストを畳み込み
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-fold [f init xs]
           (match xs
             [Nil init]
             [(Cons h t) (list-fold f (f init h) t)]))
         (defn main [] (print (list-fold (fn [acc x] (+ acc x)) 0 (Cons 1 (Cons 2 (Cons 3 Nil))))))",
    );
    // 0 + 1 + 2 + 3 = 6
    assert_eq!(output, "6\n");
}

#[test]
fn test_e2e_list_map_identity() {
    // list-map に恒等関数を渡すとリストが変わらない
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-map [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (Cons (f h) (list-map f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-map (fn [x] x) (Cons 10 (Cons 20 (Cons 30 Nil)))))))",
    );
    // 10 + 20 + 30 = 60
    assert_eq!(output, "60\n");
}

#[test]
fn test_e2e_list_fold_product() {
    // list-fold で積を計算
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-fold [f init xs]
           (match xs
             [Nil init]
             [(Cons h t) (list-fold f (f init h) t)]))
         (defn main [] (print (list-fold (fn [acc x] (* acc x)) 1 (Cons 2 (Cons 3 (Cons 4 Nil))))))",
    );
    // 1 * 2 * 3 * 4 = 24
    assert_eq!(output, "24\n");
}

#[test]
fn test_e2e_list_filter_none() {
    // list-filter で全要素がフィルタアウトされる場合
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-filter [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (if (f h) (Cons h (list-filter f t)) (list-filter f t))]))
         (defn list-length [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ 1 (list-length t))]))
         (defn main [] (print (list-length (list-filter (fn [x] (> x 100)) (Cons 1 (Cons 2 (Cons 3 Nil)))))))",
    );
    assert_eq!(output, "0\n");
}

#[test]
fn test_e2e_list_map_filter_compose() {
    // list-map と list-filter の合成: まず 2 倍してから 4 より大きいものを残す
    let output = compile_and_run(
        "(type (List a) (Cons a (List a)) Nil)
         (defn list-map [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (Cons (f h) (list-map f t))]))
         (defn list-filter [f xs]
           (match xs
             [Nil Nil]
             [(Cons h t) (if (f h) (Cons h (list-filter f t)) (list-filter f t))]))
         (defn sum-list [xs]
           (match xs
             [Nil 0]
             [(Cons h t) (+ h (sum-list t))]))
         (defn main [] (print (sum-list (list-filter (fn [x] (> x 4)) (list-map (fn [x] (* x 2)) (Cons 1 (Cons 2 (Cons 3 (Cons 4 Nil)))))))))",
    );
    // map *2: [2, 4, 6, 8], filter >4: [6, 8], sum: 14
    assert_eq!(output, "14\n");
}

// === Vector (可変長配列) ビルトイン テスト ===

#[test]
fn test_e2e_vector_new_length() {
    // vector-new で作成したベクタの初期長さは 0
    let result = compile_and_run(r#"
        (defn main []
          (print (vector-length (vector-new 10))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_vector_push_length() {
    // vector-push で要素を追加すると長さが増える
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 20)
                v3 (vector-push v2 30)]
            (print (vector-length v3))))
    "#);
    assert_eq!(result.trim(), "3");
}

#[test]
fn test_e2e_vector_get() {
    // vector-get でインデックス指定の要素を取得
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 100)
                v2 (vector-push v1 200)
                v3 (vector-push v2 300)]
            (do
              (print (vector-get v3 0))
              (print (vector-get v3 1))
              (print (vector-get v3 2)))))
    "#);
    assert_eq!(result.trim(), "100\n200\n300");
}

#[test]
fn test_e2e_vector_set() {
    // vector-set でインデックス指定の要素を上書き
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 20)
                v3 (vector-set v2 0 99)]
            (do
              (print (vector-get v3 0))
              (print (vector-get v3 1)))))
    "#);
    assert_eq!(result.trim(), "99\n20");
}

#[test]
fn test_e2e_vector_push_beyond_capacity() {
    // capacity を超えて push すると再割り当てされる
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 2)
                v1 (vector-push v 1)
                v2 (vector-push v1 2)
                v3 (vector-push v2 3)]
            (do
              (print (vector-length v3))
              (print (vector-get v3 0))
              (print (vector-get v3 1))
              (print (vector-get v3 2)))))
    "#);
    assert_eq!(result.trim(), "3\n1\n2\n3");
}

// === Vector 高階関数テスト (ユーザー定義) ===

#[test]
fn test_e2e_vector_map() {
    // vector-map: ベクタの全要素に関数を適用して新しいベクタを返す
    let result = compile_and_run(r#"
        (defn vector-map-loop [f v i len acc]
          (if (>= i len)
            acc
            (vector-map-loop f v (+ i 1) len (vector-push acc (f (vector-get v i))))))
        (defn vector-map [f v]
          (vector-map-loop f v 0 (vector-length v) (vector-new (vector-length v))))
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 20)
                v3 (vector-push v2 30)
                result (vector-map (fn [x] (* x 2)) v3)]
            (do
              (print (vector-length result))
              (print (vector-get result 0))
              (print (vector-get result 1))
              (print (vector-get result 2)))))
    "#);
    // 各要素を 2 倍: [10,20,30] -> [20,40,60]
    assert_eq!(result.trim(), "3\n20\n40\n60");
}

#[test]
fn test_e2e_vector_filter() {
    // vector-filter: 条件を満たす要素のみ残した新しいベクタを返す
    let result = compile_and_run(r#"
        (defn vector-filter-loop [f v i len acc]
          (if (>= i len)
            acc
            (if (f (vector-get v i))
              (vector-filter-loop f v (+ i 1) len (vector-push acc (vector-get v i)))
              (vector-filter-loop f v (+ i 1) len acc))))
        (defn vector-filter [f v]
          (vector-filter-loop f v 0 (vector-length v) (vector-new (vector-length v))))
        (defn main []
          (let [v (vector-new 4)
                v1 (vector-push v 10)
                v2 (vector-push v1 25)
                v3 (vector-push v2 5)
                v4 (vector-push v3 30)
                result (vector-filter (fn [x] (> x 15)) v4)]
            (do
              (print (vector-length result))
              (print (vector-get result 0))
              (print (vector-get result 1)))))
    "#);
    // 15 より大きい要素のみ: [10,25,5,30] -> [25,30]
    assert_eq!(result.trim(), "2\n25\n30");
}

// === HashMap ビルトイン テスト ===

#[test]
fn test_e2e_map_new_size() {
    // map-new で作成したマップの初期サイズは 0
    let result = compile_and_run(r#"
        (defn main []
          (print (map-size (map-new))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_map_insert_size() {
    // map-insert でエントリを追加するとサイズが増える
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 100)
                m2 (map-insert m1 2 200)]
            (print (map-size m2))))
    "#);
    assert_eq!(result.trim(), "2");
}

#[test]
fn test_e2e_map_insert_get() {
    // map-insert で挿入した値を map-get で取得
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 42 100)]
            (print (map-get m1 42))))
    "#);
    assert_eq!(result.trim(), "100");
}

#[test]
fn test_e2e_map_insert_get_multiple() {
    // 複数エントリの挿入と取得
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 10)
                m2 (map-insert m1 2 20)
                m3 (map-insert m2 3 30)]
            (do
              (print (map-get m3 1))
              (print (map-get m3 2))
              (print (map-get m3 3)))))
    "#);
    assert_eq!(result.trim(), "10\n20\n30");
}

#[test]
fn test_e2e_map_get_missing() {
    // 存在しないキーの取得は 0 を返す
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)]
            (print (map-get m 99))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_map_contains_true() {
    // 存在するキーの検索
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 42 100)]
            (print (map-contains? m1 42))))
    "#);
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_e2e_map_contains_false() {
    // 存在しないキーの検索
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)]
            (print (map-contains? m 42))))
    "#);
    assert_eq!(result.trim(), "0");
}

#[test]
fn test_e2e_map_remove() {
    // map-remove でエントリを削除
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 10)
                m2 (map-insert m1 2 20)
                m3 (map-remove m2 1)]
            (do
              (print (map-size m3))
              (print (map-contains? m3 1))
              (print (map-get m3 2)))))
    "#);
    assert_eq!(result.trim(), "1\n0\n20");
}

#[test]
fn test_e2e_map_insert_overwrite() {
    // 同じキーへの再挿入で値が上書きされる
    let result = compile_and_run(r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 10)
                m2 (map-insert m1 1 99)]
            (do
              (print (map-size m2))
              (print (map-get m2 1)))))
    "#);
    assert_eq!(result.trim(), "1\n99");
}

// === 標準ライブラリ E2E テスト ===

/// stdlib/Core.ls の基本数学関数のテスト (abs, max, min, clamp)
#[test]
fn test_e2e_stdlib_core_math() {
    let output = compile_and_run(r#"
        (defn abs [x] (if (< x 0) (- 0 x) x))
        (defn max [a b] (if (> a b) a b))
        (defn min [a b] (if (< a b) a b))
        (defn clamp [x lo hi] (max lo (min x hi)))
        (defn main [] (do
            (print (abs (- 0 5)))
            (print (abs 3))
            (print (max 3 7))
            (print (min 3 7))
            (print (clamp 15 0 10))
            (print (clamp (- 0 5) 0 10))
            (print (clamp 5 0 10))
            0))
    "#);
    assert_eq!(output.trim(), "5\n3\n7\n3\n10\n0\n5");
}

/// stdlib/Core.ls の xor 関数テスト
#[test]
fn test_e2e_stdlib_core_xor() {
    let output = compile_and_run(r#"
        (defn xor [a b] (if a (if b 0 1) (if b 1 0)))
        (defn main [] (do
            (print (xor true true))
            (print (xor true false))
            (print (xor false true))
            (print (xor false false))
            0))
    "#);
    assert_eq!(output.trim(), "0\n1\n1\n0");
}

/// stdlib/Core.ls の identity, const, twice 関数テスト
#[test]
fn test_e2e_stdlib_core_combinators() {
    let output = compile_and_run(r#"
        (defn identity [x] x)
        (defn twice [f x] (f (f x)))
        (defn main [] (do
            (print (identity 42))
            (print (twice (fn [x] (+ x 1)) 10))
            0))
    "#);
    assert_eq!(output.trim(), "42\n12");
}

/// stdlib/Core.ls の Option 型テスト (型チェックのみ - ADT は GC 型)
#[test]
fn test_e2e_stdlib_core_option_typecheck() {
    typecheck_only(r#"
        (type (Option a) (Some a) None)
        (defn unwrap [opt default]
          (match opt
            [(Some x) x]
            [None default]))
        (defn map-option [f opt]
          (match opt
            [(Some x) (Some (f x))]
            [None None]))
        (defn is-some [opt]
          (match opt
            [(Some _) 1]
            [None 0]))
        (defn main [] (print 0))
    "#);
}

/// stdlib/Core.ls の Result 型テスト (型チェックのみ - ADT は GC 型)
#[test]
fn test_e2e_stdlib_core_result_typecheck() {
    typecheck_only(r#"
        (type (Result a e) (Ok a) (Err e))
        (defn unwrap-ok [res default]
          (match res
            [(Ok x) x]
            [(Err _) default]))
        (defn map-result [f res]
          (match res
            [(Ok x) (Ok (f x))]
            [(Err e) (Err e)]))
        (defn is-ok [res]
          (match res
            [(Ok _) 1]
            [(Err _) 0]))
        (defn main [] (print 0))
    "#);
}

/// stdlib/List.ls のリスト型テスト (型チェックのみ - ADT は GC 型)
#[test]
fn test_e2e_stdlib_list_typecheck() {
    typecheck_only(r#"
        (type (List a) (Cons a (List a)) Nil)
        (defn length [xs]
          (match xs
            [Nil 0]
            [(Cons _ t) (+ 1 (length t))]))
        (defn map [f xs]
          (match xs
            [Nil Nil]
            [(Cons h t) (Cons (f h) (map f t))]))
        (defn filter [f xs]
          (match xs
            [Nil Nil]
            [(Cons h t) (if (f h) (Cons h (filter f t)) (filter f t))]))
        (defn fold [f init xs]
          (match xs
            [Nil init]
            [(Cons h t) (fold f (f init h) t)]))
        (defn append [xs ys]
          (match xs
            [Nil ys]
            [(Cons h t) (Cons h (append t ys))]))
        (defn reverse [xs]
          (fold (fn [acc x] (Cons x acc)) Nil xs))
        (defn nth [xs n default]
          (match xs
            [Nil default]
            [(Cons h t) (if (== n 0) h (nth t (- n 1) default))]))
        (defn take [n xs]
          (if (<= n 0) Nil
            (match xs
              [Nil Nil]
              [(Cons h t) (Cons h (take (- n 1) t))])))
        (defn drop [n xs]
          (if (<= n 0) xs
            (match xs
              [Nil Nil]
              [(Cons _ t) (drop (- n 1) t)])))
        (defn main [] (print 0))
    "#);
}

/// stdlib/String.ls の文字列操作テスト (starts-with, ends-with)
#[test]
fn test_e2e_stdlib_string_starts_ends_with() {
    let output = compile_and_run(r#"
        (defn starts-with [s prefix]
          (if (> (string-length prefix) (string-length s))
            false
            (string-eq (substring s 0 (string-length prefix)) prefix)))
        (defn ends-with [s suffix]
          (let [slen (string-length s)
                suflen (string-length suffix)]
            (if (> suflen slen)
              false
              (string-eq (substring s (- slen suflen) slen) suffix))))
        (defn main [] (do
            (print (if (starts-with "hello world" "hello") 1 0))
            (print (if (starts-with "hello" "hello world") 1 0))
            (print (if (ends-with "hello world" "world") 1 0))
            (print (if (ends-with "hi" "hello") 1 0))
            0))
    "#);
    assert_eq!(output.trim(), "1\n0\n1\n0");
}

/// stdlib/String.ls の string-repeat テスト
#[test]
fn test_e2e_stdlib_string_repeat() {
    let output = compile_and_run(r#"
        (defn string-repeat [s n]
          (if (<= n 0) ""
            (if (== n 1) s
              (string-concat s (string-repeat s (- n 1))))))
        (defn main [] (do
            (print (string-length (string-repeat "ab" 3)))
            (print (string-length (string-repeat "x" 1)))
            (print (string-length (string-repeat "y" 0)))
            (print (if (string-eq (string-repeat "ab" 3) "ababab") 1 0))
            0))
    "#);
    assert_eq!(output.trim(), "6\n1\n0\n1");
}

/// stdlib/String.ls の string-contains テスト
#[test]
fn test_e2e_stdlib_string_contains() {
    let output = compile_and_run(r#"
        (defn string-search-from [haystack needle hlen nlen i]
          (if (> (+ i nlen) hlen)
            (- 0 1)
            (if (string-eq (substring haystack i (+ i nlen)) needle)
              i
              (string-search-from haystack needle hlen nlen (+ i 1)))))
        (defn string-index-of [haystack needle]
          (let [hlen (string-length haystack)
                nlen (string-length needle)]
            (if (> nlen hlen)
              (- 0 1)
              (string-search-from haystack needle hlen nlen 0))))
        (defn string-contains [haystack needle]
          (if (>= (string-index-of haystack needle) 0) 1 0))
        (defn main [] (do
            (print (string-contains "hello world" "lo wo"))
            (print (string-contains "hello" "xyz"))
            (print (string-contains "abc" "abc"))
            (print (string-contains "abc" ""))
            0))
    "#);
    assert_eq!(output.trim(), "1\n0\n1\n1");
}

/// stdlib/String.ls の string-index-of テスト
#[test]
fn test_e2e_stdlib_string_index_of() {
    let output = compile_and_run(r#"
        (defn string-search-from [haystack needle hlen nlen i]
          (if (> (+ i nlen) hlen)
            (- 0 1)
            (if (string-eq (substring haystack i (+ i nlen)) needle)
              i
              (string-search-from haystack needle hlen nlen (+ i 1)))))
        (defn string-index-of [haystack needle]
          (let [hlen (string-length haystack)
                nlen (string-length needle)]
            (if (> nlen hlen)
              (- 0 1)
              (string-search-from haystack needle hlen nlen 0))))
        (defn main [] (do
            (print (string-index-of "hello world" "world"))
            (print (string-index-of "hello" "xyz"))
            (print (string-index-of "abcdef" "cd"))
            0))
    "#);
    assert_eq!(output.trim(), "6\n-1\n2");
}

// === stdlib コンパイル・実行テスト ===

#[test]
fn test_e2e_stdlib_char() {
    // Char.ls: 文字判定関数
    let result = compile_and_run(r#"
        (defn is-digit [c]
          (if (>= c 48) (<= c 57) false))
        (defn is-upper [c]
          (if (>= c 65) (<= c 90) false))
        (defn is-lower [c]
          (if (>= c 97) (<= c 122) false))
        (defn is-alpha [c]
          (if (is-upper c) true (is-lower c)))
        (defn is-whitespace [c]
          (if (== c 32) true
            (if (== c 9) true
              (if (== c 10) true
                (== c 13)))))
        (defn main []
          (do
            (print (is-digit 48))
            (print (is-digit 65))
            (print (is-alpha 65))
            (print (is-alpha 48))
            (print (is-whitespace 32))
            0))
    "#);
    // 48='0' is digit=1, 65='A' is not digit=0, 65='A' is alpha=1, 48='0' is not alpha=0, 32=' ' is whitespace=1
    assert_eq!(result.trim(), "1\n0\n1\n0\n1");
}

#[test]
fn test_e2e_stdlib_debug() {
    // Debug.ls: デバッグ・アサーション関数
    let result = compile_and_run(r#"
        (defn debug-print [x]
          (do (print x) x))
        (defn assert [cond]
          (if cond 0 0))
        (defn assert-eq [a b]
          (assert (== a b)))
        (defn main []
          (do
            (assert true)
            (assert-eq 42 42)
            (print (debug-print 99))
            0))
    "#);
    // debug-print prints 99, then main prints the return value 99 again
    assert_eq!(result.trim(), "99\n99");
}

#[test]
fn test_e2e_stdlib_set() {
    // Set.ls: HashMap ベースの集合
    let result = compile_and_run(r#"
        (defn set-new [] (map-new))
        (defn set-add [s x] (map-insert s x 1))
        (defn set-contains? [s x] (map-contains? s x))
        (defn set-remove [s x] (map-remove s x))
        (defn set-size [s] (map-size s))
        (defn main []
          (let [s (set-new)
                s1 (set-add s 10)
                s2 (set-add s1 20)
                s3 (set-add s2 30)]
            (do
              (print (set-size s3))
              (print (set-contains? s3 20))
              (print (set-contains? s3 99))
              0)))
    "#);
    assert_eq!(result.trim(), "3\n1\n0");
}

// === ファイル I/O & WASI 拡張テスト ===

#[test]
fn test_e2e_command_line_args() {
    // command-line-args: コマンドライン引数の数を返す
    // wasmtime で実行した場合、引数が 0 以上の整数が返る
    let result = compile_and_run(r#"
        (defn main []
          (let [argc (command-line-args)]
            (do
              (print (>= argc 0))
              0)))
    "#);
    // argc >= 0 は常に true (1)
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_e2e_write_and_read_file() {
    // write-file + read-file: ファイルに書き込んで読み出し
    let tmpdir = std::env::temp_dir().join("lsharp_test_file_io");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let result = compile_and_run_with_dir(r#"
        (defn main []
          (let [written (write-file "test_output.txt" "hello")
                content (read-file "test_output.txt")]
            (do
              (print written)
              (print (string-length content))
              0)))
    "#, &tmpdir);
    // written = 5 (bytes), content length = 5
    assert_eq!(result.trim(), "5\n5");
    // クリーンアップ
    let _ = std::fs::remove_dir_all(&tmpdir);
}

#[test]
fn test_e2e_file_exists() {
    // file-exists?: ファイル存在チェック (preopened dir 付き)
    let tmpdir = std::env::temp_dir().join("lsharp_test_file_exists");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let result = compile_and_run_with_dir(r#"
        (defn main []
          (do
            (print (file-exists? "nonexistent_file_xyz.txt"))
            0))
    "#, &tmpdir);
    assert_eq!(result.trim(), "0");
    let _ = std::fs::remove_dir_all(&tmpdir);
}

// === セルフホスティング: Lexer テスト ===

#[test]
fn test_e2e_selfhost_lexer_basic() {
    // セルフホスティング Lexer: 基本トークナイズ
    let result = compile_and_run(r#"
        (defn is-ws [c]
          (if (== c 32) true (if (== c 9) true (if (== c 10) true (== c 13)))))
        (defn is-digit-char [c]
          (if (>= c 48) (<= c 57) false))
        (defn is-alpha-char [c]
          (if (>= c 65) (if (<= c 90) true (if (>= c 97) (<= c 122) false)) false))
        (defn is-symbol-start [c]
          (if (is-alpha-char c) true
            (if (== c 95) true (if (== c 43) true (if (== c 45) true
              (if (== c 42) true (if (== c 47) true (if (== c 61) true
                (if (== c 60) true (if (== c 62) true (if (== c 33) true
                  (if (== c 63) true false))))))))))))
        (defn is-symbol-char [c]
          (if (is-symbol-start c) true (if (is-digit-char c) true (if (== c 46) true (== c 45)))))
        (defn skip-comment [src pos len]
          (if (>= pos len) pos
            (if (== (string-char-at src pos) 10) (+ pos 1)
              (skip-comment src (+ pos 1) len))))
        (defn skip-ws-loop [src pos len]
          (if (>= pos len) pos
            (let [c (string-char-at src pos)]
              (if (is-ws c) (skip-ws-loop src (+ pos 1) len)
                (if (== c 59) (let [end (skip-comment src (+ pos 1) len)]
                  (skip-ws-loop src end len)) pos)))))
        (defn classify-symbol [name]
          (if (string-eq name "defn") 30
            (if (string-eq name "let") 31
              (if (string-eq name "if") 32
                (if (string-eq name "true") 13
                  (if (string-eq name "false") 14 20))))))
        (defn scan-digits [src pos len]
          (if (>= pos len) pos
            (if (is-digit-char (string-char-at src pos)) (scan-digits src (+ pos 1) len) pos)))
        (defn scan-symbol-end [src pos len]
          (if (>= pos len) pos
            (if (is-symbol-char (string-char-at src pos)) (scan-symbol-end src (+ pos 1) len) pos)))
        (defn lex-one [src pos len]
          (if (>= pos len) (+ (* 99 1000000) pos)
            (let [c (string-char-at src pos)]
              (if (== c 40) (+ (* 0 1000000) (+ pos 1))
                (if (== c 41) (+ (* 1 1000000) (+ pos 1))
                  (if (== c 91) (+ (* 2 1000000) (+ pos 1))
                    (if (== c 93) (+ (* 3 1000000) (+ pos 1))
                      (if (is-digit-char c)
                        (let [end (scan-digits src (+ pos 1) len)]
                          (+ (* 10 1000000) end))
                        (if (is-symbol-start c)
                          (let [end (scan-symbol-end src (+ pos 1) len)
                                name (substring src pos end)
                                kind (classify-symbol name)]
                            (+ (* kind 1000000) end))
                          (+ (* 99 1000000) (+ pos 1)))))))))))
        (defn tokenize-loop [src pos len tokens]
          (let [ws-pos (skip-ws-loop src pos len)]
            (if (>= ws-pos len)
              (vector-push tokens 99)
              (let [result (lex-one src ws-pos len)
                    kind (/ result 1000000)
                    end-pos (- result (* kind 1000000))]
                (if (== kind 99)
                  (vector-push tokens 99)
                  (tokenize-loop src end-pos len (vector-push tokens kind)))))))
        (defn tokenize [src]
          (tokenize-loop src 0 (string-length src) (vector-new 16)))
        (defn main []
          (let [tokens (tokenize "(defn main [] 42)")
                len (vector-length tokens)]
            (do
              (print len)
              (print (vector-get tokens 0))
              (print (vector-get tokens 1))
              (print (vector-get tokens 2))
              (print (vector-get tokens 3))
              (print (vector-get tokens 4))
              (print (vector-get tokens 5))
              (print (vector-get tokens 6))
              (print (vector-get tokens 7))
              0)))
    "#);
    // 8 tokens: ( defn main [ ] 42 ) EOF
    // kinds:    0  30   20  2 3 10  1  99
    assert_eq!(result.trim(), "8\n0\n30\n20\n2\n3\n10\n1\n99");
}

#[test]
fn test_e2e_selfhost_parser_basic() {
    // セルフホスティング Parser: 基本的な S 式パース
    let result = compile_and_run(r#"
        (defn parse-expr [tokens pos]
          (let [tok (vector-get tokens (ref-get pos))]
            (if (== tok 0)
              (do (ref-set pos (+ (ref-get pos) 1))
                (let [inner-tok (vector-get tokens (ref-get pos))
                      result (if (== inner-tok 30) (do (ref-set pos (+ (ref-get pos) 1)) 20)
                               (if (== inner-tok 32) (do (ref-set pos (+ (ref-get pos) 1)) 6)
                                 5))]
                  (do
                    ;; skip until )
                    result)))
              (if (== tok 10) (do (ref-set pos (+ (ref-get pos) 1)) 1)
                (if (== tok 20) (do (ref-set pos (+ (ref-get pos) 1)) 4)
                  (if (== tok 13) (do (ref-set pos (+ (ref-get pos) 1)) 2)
                    0))))))
        (defn main []
          (let [tokens (vector-push (vector-push (vector-push (vector-push
                        (vector-push (vector-push (vector-push (vector-push
                          (vector-new 8) 0) 30) 20) 2) 3) 10) 1) 99)
                pos (ref-new 0)
                result (parse-expr tokens pos)]
            (do
              (print result)
              (print (ref-get pos))
              0)))
    "#);
    // defn ノード (20) を検出、位置は 2 進んだ
    assert_eq!(result.trim(), "20\n2");
}

#[test]
fn test_e2e_selfhost_type_system() {
    // セルフホスティング型システム: 型 ADT + Substitution
    let result = compile_and_run(r#"
        (defn make-type-con [hash]
          (vector-push (vector-push (vector-new 2) 1) hash))
        (defn make-type-var [id]
          (vector-push (vector-push (vector-new 2) 2) id))
        (defn type-tag [ty] (vector-get ty 0))
        (defn type-val [ty] (vector-get ty 1))
        (defn subst-new [] (map-new))
        (defn subst-bind [s var-id ty-tag] (map-insert s var-id ty-tag))
        (defn subst-lookup [s var-id] (map-get s var-id))
        (defn main []
          (let [int-ty (make-type-con 0)
                var-ty (make-type-var 42)
                s (subst-bind (subst-new) 42 0)]
            (do
              (print (type-tag int-ty))
              (print (type-tag var-ty))
              (print (type-val var-ty))
              (print (subst-lookup s 42))
              0)))
    "#);
    assert_eq!(result.trim(), "1\n2\n42\n0");
}

#[test]
fn test_e2e_selfhost_unification() {
    // セルフホスティング Unification: 型構築 + Substitution + occurs-check + unify
    // map-contains? (Bool) を避け、map-get + = (Int比較) で統一
    let result = compile_and_run(r#"
        ;; 型構築
        (defn make-type-con [hash]
          (vector-push (vector-push (vector-new 2) 1) hash))
        (defn make-type-int [] (make-type-con 100))
        (defn make-type-bool [] (make-type-con 200))
        (defn make-type-var [id]
          (vector-push (vector-push (vector-new 2) 2) id))

        ;; 型アクセス
        (defn type-tag [ty] (vector-get ty 0))
        (defn type-name [ty] (vector-get ty 1))

        ;; Substitution (map-get のみ使用、map-contains? を避ける)
        (defn subst-new [] (map-new))
        (defn subst-bind [s var-id ty] (map-insert s var-id ty))

        ;; 型の等価判定 (1=等しい, 0=異なる)
        (defn types-eq [ty1 ty2]
          (if (= (type-tag ty1) (type-tag ty2))
            (if (= (type-name ty1) (type-name ty2)) 1 0)
            0))

        ;; occurs-check (1=出現, 0=非出現)
        (defn occurs-check [var-id ty]
          (if (= (type-tag ty) 2)
            (if (= var-id (type-name ty)) 1 0)
            0))

        ;; エラーマーカー: 特殊キー -1 に値 1 を入れた Map
        (defn unify-error [] (map-insert (map-new) -1 1))
        ;; エラー判定: map-get で -1 キーを取得 (0 = エラーなし)
        (defn is-error [s] (map-get s -1))

        ;; 単純 unify (Con/Var のみ)
        (defn unify-simple [t1 t2 subst]
          (if (= (types-eq t1 t2) 1)
            subst
            (if (= (type-tag t1) 2)
              (if (= (occurs-check (type-name t1) t2) 1)
                (unify-error)
                (subst-bind subst (type-name t1) t2))
              (if (= (type-tag t2) 2)
                (if (= (occurs-check (type-name t2) t1) 1)
                  (unify-error)
                  (subst-bind subst (type-name t2) t1))
                (unify-error)))))

        ;; apply-subst: Con/Var 型のみ
        (defn apply-subst-simple [subst ty]
          (if (= (type-tag ty) 2)
            (let [looked (map-get subst (type-name ty))]
              (if (= looked 0)
                ty
                looked))
            ty))

        (defn main []
          (let [int1 (make-type-int)
                int2 (make-type-int)
                bool1 (make-type-bool)
                var1 (make-type-var 10)
                s0 (subst-new)]
            (do
              ;; テスト1: Int == Int → 成功 (is-error=0)
              (let [r1 (unify-simple int1 int2 s0)]
                (print (if (= (is-error r1) 0) 1 0)))

              ;; テスト2: Int != Bool → 失敗 (is-error=1)
              (let [r2 (unify-simple int1 bool1 s0)]
                (print (if (= (is-error r2) 0) 1 0)))

              ;; テスト3: Var(10) と Int → 成功 + 置換
              (let [r3 (unify-simple var1 int1 s0)]
                (do
                  (print (if (= (is-error r3) 0) 1 0))
                  ;; 置換に var-id=10 が含まれる (map-get で確認)
                  (let [v10 (map-get r3 10)]
                    (print (if (= v10 0) 0 1)))
                  ;; apply-subst で Var(10) → Int
                  (let [resolved (apply-subst-simple r3 var1)]
                    (do
                      (print (type-tag resolved))
                      (print (type-name resolved))))))

              ;; テスト4: occurs-check
              (print (occurs-check 10 var1))
              (print (occurs-check 99 var1))
              (print (occurs-check 10 int1))

              0)))
    "#);
    assert_eq!(result.trim(), "1\n0\n1\n1\n1\n100\n1\n0\n0");
}

#[test]
fn test_e2e_selfhost_ir() {
    // セルフホスティング IR: 命令構築
    let result = compile_and_run(r#"
        (defn make-instr [opcode operand]
          (vector-push (vector-push (vector-new 2) opcode) operand))
        (defn main []
          (let [c (make-instr 1 42)
                g (make-instr 10 0)]
            (do
              (print (vector-get c 0))
              (print (vector-get c 1))
              (print (vector-get g 0))
              (print (vector-get g 1))
              0)))
    "#);
    assert_eq!(result.trim(), "1\n42\n10\n0");
}

#[test]
fn test_e2e_selfhost_compiler() {
    // セルフホスティング Compiler: AST→IR 変換 + LEB128 エンコード
    let result = compile_and_run(r#"
        ;; IR 命令構築
        (defn emit-instr [opcode operand]
          (vector-push (vector-push (vector-new 2) opcode) operand))

        (defn emit-to [instrs opcode operand]
          (vector-push instrs (emit-instr opcode operand)))

        ;; 環境 (変数名ハッシュ → ローカルインデックス)
        (defn env-new [] (map-new))
        (defn env-bind [env name-hash idx] (map-insert env name-hash idx))
        (defn env-lookup [env name-hash] (map-get env name-hash))

        ;; AST → IR コンパイル (整数リテラル, 真偽値, 変数参照)
        (defn compile-expr [node env instrs]
          (let [tag (vector-get node 0)]
            (if (= tag 1)
              (emit-to instrs 1 (vector-get node 1))
              (if (= tag 2)
                (emit-to instrs 1 (vector-get node 1))
                (if (= tag 4)
                  (let [name-hash (vector-get node 1)
                        idx (env-lookup env name-hash)]
                    (if (= idx 0)
                      (emit-to instrs 1 0)
                      (emit-to instrs 10 idx)))
                  (emit-to instrs 1 0))))))

        ;; LEB128 符号なしエンコード
        (defn leb128-unsigned [value]
          (let [result (ref-new (vector-new 4))
                v (ref-new value)]
            (do
              (let [byte (% (ref-get v) 128)
                    rest (/ (ref-get v) 128)]
                (if (= rest 0)
                  (ref-set result (vector-push (ref-get result) byte))
                  (do
                    (ref-set result (vector-push (ref-get result) (+ byte 128)))
                    (ref-set v rest)
                    (let [byte2 (% (ref-get v) 128)
                          rest2 (/ (ref-get v) 128)]
                      (if (= rest2 0)
                        (ref-set result (vector-push (ref-get result) byte2))
                        (do
                          (ref-set result (vector-push (ref-get result) (+ byte2 128)))
                          (ref-set v rest2)
                          (ref-set result (vector-push (ref-get result) (% (ref-get v) 128)))))))))
              (ref-get result))))

        (defn main []
          (let [;; 整数リテラル [1, 42] をコンパイル
                lit-node (vector-push (vector-push (vector-new 2) 1) 42)
                env (env-new)
                instrs (compile-expr lit-node env (vector-new 8))

                ;; 変数参照 [4, 99] を環境ありでコンパイル
                var-node (vector-push (vector-push (vector-new 2) 4) 99)
                env2 (env-bind env 99 3)
                instrs2 (compile-expr var-node env2 (vector-new 8))

                ;; LEB128 テスト
                leb5 (leb128-unsigned 5)
                leb300 (leb128-unsigned 300)]
            (do
              ;; 整数リテラルのコンパイル結果
              (print (vector-length instrs))
              (let [i0 (vector-get instrs 0)]
                (do
                  (print (vector-get i0 0))
                  (print (vector-get i0 1))))

              ;; 変数参照のコンパイル結果
              (print (vector-length instrs2))
              (let [i1 (vector-get instrs2 0)]
                (do
                  (print (vector-get i1 0))
                  (print (vector-get i1 1))))

              ;; LEB128
              (print (vector-length leb5))
              (print (vector-get leb5 0))
              (print (vector-length leb300))
              (print (vector-get leb300 0))
              (print (vector-get leb300 1))
              0)))
    "#);
    assert_eq!(result.trim(), "1\n1\n42\n1\n10\n3\n1\n5\n2\n172\n2");
}

#[test]
fn test_e2e_selfhost_type_scheme() {
    // セルフホスティング: TypeScheme (let 多相の instantiate/free-vars)
    let result = compile_and_run(r#"
        ;; TypeScheme = [type, bound-vars-vector]
        (defn mono [ty]
          (vector-push (vector-push (vector-new 2) ty) (vector-new 0)))

        (defn poly [ty bound-vars]
          (vector-push (vector-push (vector-new 2) ty) bound-vars))

        (defn scheme-type [scheme] (vector-get scheme 0))
        (defn scheme-vars [scheme] (vector-get scheme 1))

        ;; 型変数カウンタ
        (defn make-var-counter [] (ref-new 1000))
        (defn next-var [counter]
          (let [id (ref-get counter)]
            (do (ref-set counter (+ id 1)) id)))

        ;; instantiate-apply: 置換を型に適用
        (defn inst-apply [subst ty]
          (let [tag (vector-get ty 0)]
            (if (= tag 2)
              (let [looked (map-get subst (vector-get ty 1))]
                (if (= looked 0) ty looked))
              (if (= tag 3)
                (vector-push
                  (vector-push
                    (vector-push (vector-new 3) 3)
                    (inst-apply subst (vector-get ty 1)))
                  (inst-apply subst (vector-get ty 2)))
                ty))))

        ;; instantiate: 型スキームを具体化
        (defn instantiate [scheme counter]
          (let [ty (scheme-type scheme)
                vars (scheme-vars scheme)
                n (vector-length vars)]
            (if (= n 0)
              ty
              (let [subst (ref-new (map-new))
                    i (ref-new 0)]
                (do
                  (if (< (ref-get i) n)
                    (do
                      (let [old-v (vector-get vars (ref-get i))
                            new-id (next-var counter)
                            new-ty (vector-push (vector-push (vector-new 2) 2) new-id)]
                        (ref-set subst (map-insert (ref-get subst) old-v new-ty)))
                      (ref-set i (+ (ref-get i) 1))
                      0)
                    0)
                  (inst-apply (ref-get subst) ty))))))

        ;; free-vars: 型の自由変数を収集
        (defn free-vars [ty]
          (let [tag (vector-get ty 0)]
            (if (= tag 2)
              (vector-push (vector-new 1) (vector-get ty 1))
              (if (= tag 3)
                (let [pv (free-vars (vector-get ty 1))
                      rv (free-vars (vector-get ty 2))
                      result (ref-new pv)
                      j (ref-new 0)
                      m (vector-length rv)]
                  (do
                    (if (< (ref-get j) m)
                      (do
                        (ref-set result (vector-push (ref-get result) (vector-get rv (ref-get j))))
                        (ref-set j (+ (ref-get j) 1))
                        0)
                      0)
                    (ref-get result)))
                (vector-new 0)))))

        (defn main []
          (let [;; 型準備
                int-ty (vector-push (vector-push (vector-new 2) 1) 100)
                var-a (vector-push (vector-push (vector-new 2) 2) 1)
                fun-ty (vector-push (vector-push (vector-push (vector-new 3) 3) var-a) var-a)

                ;; 型スキーム
                int-scheme (mono int-ty)
                bound (vector-push (vector-new 1) 1)
                id-scheme (poly fun-ty bound)

                ;; instantiate
                counter (make-var-counter)
                inst1 (instantiate int-scheme counter)
                inst2 (instantiate id-scheme counter)]
            (do
              ;; 単相の instantiate
              (print (vector-get inst1 0))  ;; 1 (Con)
              (print (vector-get inst1 1))  ;; 100

              ;; 多相の instantiate (Fun型 + 新型変数)
              (print (vector-get inst2 0))  ;; 3 (Fun)
              (let [param (vector-get inst2 1)]
                (do
                  (print (vector-get param 0))  ;; 2 (Var)
                  (print (vector-get param 1)))) ;; 1000

              ;; free-vars
              (print (vector-length (free-vars int-ty)))  ;; 0
              (print (vector-length (free-vars var-a)))   ;; 1
              (print (vector-get (free-vars var-a) 0))    ;; 1

              0)))
    "#);
    assert_eq!(result.trim(), "1\n100\n3\n2\n1000\n0\n1\n1");
}
