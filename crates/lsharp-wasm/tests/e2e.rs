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

// === エッジケース: ランタイムエラー ===

#[test]
#[should_panic]
fn test_e2e_division_by_zero_traps() {
    // Wasm の i64.div_s はゼロ除算で trap する
    compile_and_run("(defn main [] (print (/ 1 0)))");
}
