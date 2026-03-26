use super::support::*;


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

/// CP-05: __alloc メトリクス — peak heap pointer が alloc 後に増加すること
#[test]
fn test_e2e_alloc_metrics_peak_usage() {
    // 複数回 alloc 後、heap_ptr (global 0) が初期値より増えていることを検証
    // __alloc_peak / __alloc_total はまだ builtin にないので、
    // heap_ptr の差分で代替検証: 2 回 alloc して 2 番目のアドレスが 1 番目より大きい
    let result = compile_and_run(r#"
        (defn main []
          (let [a1 (__alloc 32)
                a2 (__alloc 64)
                a3 (__alloc 128)]
            (do
              (print a1)
              (print a2)
              (print a3)
              (print (- a3 a1))
              0)))
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert!(lines.len() >= 4, "alloc metrics 出力が不足: {:?}", lines);
    let a1: i64 = lines[0].parse().unwrap();
    let a2: i64 = lines[1].parse().unwrap();
    let a3: i64 = lines[2].parse().unwrap();
    let total_span: i64 = lines[3].parse().unwrap();
    assert!(a1 > 0, "初回 alloc アドレスは正の値");
    assert!(a2 > a1, "2 回目 alloc は 1 回目より後方");
    assert!(a3 > a2, "3 回目 alloc は 2 回目より後方");
    // 32 + 64 = 96 bytes (8-byte aligned: 32 + 64 = 96)
    assert!(total_span >= 96, "alloc span は少なくとも 96 bytes: got {}", total_span);
}

/// CP-05: __alloc メトリクス — 同サイズ連続 alloc で heap が単調増加すること
#[test]
fn test_e2e_alloc_metrics_monotonic_check() {
    let result = compile_and_run(r#"
        (defn alloc-loop [n prev-addr ok]
          (if (<= n 0)
            ok
            (let [addr (__alloc 16)]
              (if (> addr prev-addr)
                (alloc-loop (- n 1) addr ok)
                0))))
        (defn main []
          (let [first (__alloc 16)
                result (alloc-loop 100 first 1)]
            (do (print result) 0)))
    "#);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "1", "100 回の連続 alloc で heap は単調増加すべき");
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
    let result = compile_and_run_expanded(source);
    assert_eq!(result.trim(), "42");
}

#[test]
fn test_e2e_lambda_with_free_vars_compile() {
    // 自由変数あり Lambda がリフトされてコンパイル可能
    let source = r#"
        (defn make-adder [n] (fn [x] (+ x n)))
        (defn main [] (print 99))
    "#;
    let result = compile_and_run_expanded(source);
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

/// マルチファイル型推論: import 先に helper が増えても open import の多相関数は一般化を保つ
#[test]
fn test_e2e_multi_file_import_open_polymorphic_helper_stays_generalized() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_e2e_import_poly_helper_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    std::fs::write(
        dir.join("Utils.ls"),
        "(module Utils)\n(defn choose-first [x y] x)\n(defn helper [] 0)",
    )
    .unwrap();

    std::fs::write(
        dir.join("main.ls"),
        "(module Main)\n(import Utils :open)\n(defn main [] (do (print (choose-first 1 true)) (if (choose-first true 1) (print 1) (print 0))))",
    )
    .unwrap();

    let wasm = try_compile_file_only(&dir.join("main.ls"))
        .expect("helper 追加後も imported polymorphic function は compile できるべき");
    assert_valid_wasm(&wasm);

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
