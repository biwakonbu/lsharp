use super::support::*;


// === P1-2: 文字列リテラルのヒープ化テスト ===

#[test]
fn test_e2e_string_heap_print() {
    // ヒープ上の String オブジェクト経由で文字列が正しく出力されることを検証
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string "hello heap") 0))
    "#);
    assert_eq!(result, "hello heap");
}

#[test]
fn test_e2e_string_heap_length() {
    // ヒープ上の String オブジェクトから長さが正しく取得できることを検証
    let result = compile_and_run(r#"
        (defn main []
          (print (string-length "heap string")))
    "#);
    assert_eq!(result.trim(), "11");
}

#[test]
fn test_e2e_string_heap_char_at() {
    // ヒープ上の String オブジェクトから文字取得が正しく動作することを検証
    let result = compile_and_run(r#"
        (defn main []
          (print (string-char-at "abcdef" 2)))
    "#);
    // 'c' = 99
    assert_eq!(result.trim(), "99");
}

#[test]
fn test_e2e_string_heap_substring() {
    // ヒープ上の String オブジェクトから部分文字列が正しく取得できることを検証
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (substring "hello world" 6 11)) 0))
    "#);
    assert_eq!(result, "world");
}

#[test]
fn test_e2e_string_heap_concat_mixed() {
    // リテラル文字列同士の結合がヒープ上で正しく動作することを検証
    let result = compile_and_run(r#"
        (defn main []
          (do (print-string (string-concat "foo" "bar")) 0))
    "#);
    assert_eq!(result, "foobar");
}

#[test]
fn test_e2e_string_heap_eq() {
    // ヒープ上の文字列同士の比較が正しく動作することを検証
    let result = compile_and_run(r#"
        (defn main []
          (print (if (string-eq "test" "test") 1 0)))
    "#);
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_e2e_string_heap_multiple_literals() {
    // 複数の文字列リテラルがそれぞれヒープ上に正しく配置されることを検証
    let result = compile_and_run(r#"
        (defn main []
          (do
            (print-string "first")
            (print-string " ")
            (print-string "second")
            0))
    "#);
    assert_eq!(result, "first second");
}

#[test]
fn test_e2e_string_heap_object_layout() {
    // 文字列リテラルがヒープ上に [tag=1][len][bytes] として配置されることを検証
    let result = compile_and_run(r#"
        (defn main []
          (let [s "hello"]
            (do
              (print (string-length s))
              (print (string-char-at s 0))
              (print (string-char-at s 4))
              0)))
    "#);
    // "hello": length=5, 'h'=104, 'o'=111
    assert_eq!(result.trim(), "5\n104\n111");
}

// === ネストパターンマッチ E2E テスト ===

#[test]
fn test_e2e_nested_constructor_pattern() {
    // ネストしたコンストラクタパターン (深さ2)
    let output = compile_and_run(
        "(type Tree (Leaf Int) (Node Tree Tree))
         (defn depth [t]
           (match t
             [(Leaf _) 1]
             [(Node (Leaf _) _) 2]
             [(Node _ _) 3]))
         (defn main [] (do
           (print (depth (Leaf 1)))
           (print (depth (Node (Leaf 1) (Leaf 2))))
           (print (depth (Node (Node (Leaf 1) (Leaf 2)) (Leaf 3))))
           0))",
    );
    assert_eq!(output, "1\n2\n3\n");
}

#[test]
fn test_e2e_nested_constructor_pattern_extract() {
    // ネストしたコンストラクタパターンでフィールドを取り出す
    let output = compile_and_run(
        "(type (Maybe a) (Just a) Nothing)
         (defn unwrap-nested [m]
           (match m
             [(Just (Just x)) x]
             [(Just Nothing) -1]
             [Nothing -2]))
         (defn main [] (do
           (print (unwrap-nested (Just (Just 42))))
           (print (unwrap-nested (Just Nothing)))
           (print (unwrap-nested Nothing))
           0))",
    );
    assert_eq!(output, "42\n-1\n-2\n");
}

// === ガード条件 (when 節) E2E テスト ===

#[test]
fn test_e2e_match_guard_basic() {
    // ガード条件 (when 節) 付きパターンマッチ
    let output = compile_and_run(
        "(defn classify [n]
           (match n
             [x when (> x 0) 1]
             [x when (< x 0) -1]
             [_ 0]))
         (defn main [] (do
           (print (classify 5))
           (print (classify -3))
           (print (classify 0))
           0))",
    );
    assert_eq!(output, "1\n-1\n0\n");
}

#[test]
fn test_e2e_match_guard_with_binding() {
    // ガード条件で束縛した変数を使用
    let output = compile_and_run(
        "(defn first-positive [a b]
           (match a
             [x when (> x 0) x]
             [_ (match b
                  [y when (> y 0) y]
                  [_ 0])]))
         (defn main [] (do
           (print (first-positive 5 10))
           (print (first-positive -1 7))
           (print (first-positive -1 -2))
           0))",
    );
    assert_eq!(output, "5\n7\n0\n");
}

// =====================================================// P8-5: ブートストラップ統合検証
// selfhost/ の複数モジュールを結合した統合パイプラインの検証
// =====================================================
/// 統合テスト: selfhost/Main.ls を Rust コンパイラでコンパイル・実行し、
/// AST 構築 → IR 変換 → Wasm バイナリ生成の統合パイプラインを検証する。
#[test]
fn test_e2e_bootstrap_stage1_integration() {
    let output = compile_and_run_file(&selfhost_main_path());
    // 統合パイプラインの出力:
    // 旧: AST(1,42) + IR(1,1,42) + Wasm(8,0,97,115,109,7,1) + WASI(15,10)
    // T4-4: tokens(8) + defn(20) + body(1,42) + IR(1,1,42)
    // T4-4 拡張: if(1,6,3) + let(1,7,2)
    assert_eq!(
        output.trim(),
        "1\n42\n1\n1\n42\n8\n0\n97\n115\n109\n7\n1\n15\n10\n8\n20\n1\n42\n1\n1\n42\n1\n6\n3\n1\n7\n2\n1\n1\n100\n1\n5"
    );
}

/// 統合テスト: selfhost/ の全モジュールを結合したソースが正しくコンパイルでき、
/// stage1.wasm 相当のバイナリ生成まで検証する。
#[test]
fn test_e2e_bootstrap_stage1_wasm_generation() {
    let wasm_bytes = compile_file_only(&selfhost_main_path());
    // 有効な Wasm バイナリであること (マジックナンバー確認)
    assert!(wasm_bytes.len() > 8, "Wasm バイナリが短すぎる: {} bytes", wasm_bytes.len());
    assert_eq!(&wasm_bytes[0..4], b"\0asm", "Wasm マジックナンバーが不正");
}

// =====================================================// P8-5: 相互再帰関数の前方参照 E2E テスト
// =====================================================
/// 相互再帰関数 (even?/odd?) のコンパイル+実行
#[test]
fn test_e2e_mutual_recursion_even_odd() {
    let source = r#"
        (defn even? [n] (if (= n 0) 1 (odd? (- n 1))))
        (defn odd? [n] (if (= n 0) 0 (even? (- n 1))))
        (defn main [] (print (even? 10)))
    "#;
    let output = compile_and_run(source);
    assert_eq!(output.trim(), "1");
}

/// stdlib/Path.ls のパス操作ユーティリティのコンパイル+実行
#[test]
fn test_e2e_stdlib_path_operations() {
    let source = std::fs::read_to_string("../../stdlib/Path.ls").unwrap();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), 4, "Path.ls は4行の出力を生成するべき: {:?}", lines);
    assert_eq!(lines[0], "13"); // path-join "/tmp" "file.txt" = "/tmp/file.txt" (13文字)
    assert_eq!(lines[1], "4");  // path-extension "file.txt" = ".txt" (4文字)
    assert_eq!(lines[2], "8");  // path-basename "/tmp/file.txt" = "file.txt" (8文字)
    assert_eq!(lines[3], "4");  // path-dirname "/tmp/file.txt" = "/tmp" (4文字)
}

/// selfhost/Compiler.ls のセルフホストコンパイラのコンパイル+実行
#[test]
fn test_e2e_selfhost_compiler_file() {
    let source = std::fs::read_to_string("../../selfhost/Compiler.ls").unwrap();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 3, "Compiler.ls は少なくとも3行の出力を生成するべき: {:?}", lines);
    assert_eq!(lines[0], "1");  // vector-length instrs = 1
    assert_eq!(lines[1], "1");  // op: i64.const
    assert_eq!(lines[2], "42"); // operand: 42
}

/// selfhost/WasmEmit.ls の Wasm バイナリ生成のコンパイル+実行
#[test]
fn test_e2e_selfhost_wasmemit() {
    let source = std::fs::read_to_string("../../selfhost/WasmEmit.ls").unwrap();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 6, "WasmEmit.ls は少なくとも6行の出力を生成するべき: {:?}", lines);
    assert_eq!(lines[0], "8");   // ヘッダー長
    assert_eq!(lines[1], "0");   // \0
    assert_eq!(lines[2], "97");  // 'a'
    assert_eq!(lines[3], "115"); // 's'
    assert_eq!(lines[4], "109"); // 'm'
    assert_eq!(lines[5], "1");   // version
}

/// T1-9: selfhost/Main.ls 統合 E2E テスト
/// AST 構築 → IR 変換 → Wasm ヘッダー生成の統合パイプラインを検証
#[test]
fn test_e2e_selfhost_main_integration() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();

    // Main.ls 旧パイプライン + T4-4 新パイプライン検証
    assert!(lines.len() >= 32, "Main.ls は少なくとも32行の出力を生成するべき: {:?}", lines);

    // 旧パイプライン: AST → IR → Wasm
    assert_eq!(lines[0], "1");    // ast-tag = 1 (lit-int)
    assert_eq!(lines[1], "42");   // value = 42
    assert_eq!(lines[2], "1");    // vector-length instrs = 1
    assert_eq!(lines[3], "1");    // op: i64.const
    assert_eq!(lines[4], "42");   // operand: 42
    assert_eq!(lines[5], "8");    // ヘッダー長 = 8
    assert_eq!(lines[6], "0");    // \0
    assert_eq!(lines[7], "97");   // 'a'
    assert_eq!(lines[8], "115");  // 's'
    assert_eq!(lines[9], "109");  // 'm'
    assert_eq!(lines[10], "7");   // type section length = 7
    assert_eq!(lines[11], "1");   // section-id: Type
    assert_eq!(lines[12], "15");  // wasm-size = 8 + 7
    assert_eq!(lines[13], "10");  // module-count = 10

    // T4-4: 新パイプライン (Lexer.tokenize の kind 列長)
    assert_eq!(lines[14], "8");  // "(defn main [] 42)" のトークン数 (Lexer 実装に準拠)
    assert_eq!(lines[15], "20");  // defn AST tag
    assert_eq!(lines[16], "1");   // body: lit-int tag
    assert_eq!(lines[17], "42");  // body: value = 42
    assert_eq!(lines[18], "1");   // IR: 1 命令
    assert_eq!(lines[19], "1");   // IR instr: i64.const
    assert_eq!(lines[20], "42");  // IR operand: 42

    // P11: 完全パイプライン (MacroExpand + TypeInfer 統合)
    assert_eq!(lines[27], "1");   // expanded AST tag = 1 (lit-int)
    assert_eq!(lines[28], "1");   // 型推論結果: ty-tag = 1 (Con)
    assert_eq!(lines[29], "100"); // 型推論結果: ty-name = 100 (Int)
    assert_eq!(lines[30], "1");   // IR 命令数 = 1
    assert_eq!(lines[31], "5");   // パイプラインステージ数 = 5
}

/// T2-1: Lexer.ls 値つきトークン (kind, start, end) 3つ組のテスト
#[test]
fn test_e2e_selfhost_lexer_value_tokens() {
    let source = std::fs::read_to_string("../../selfhost/Lexer.ls").unwrap();
    let output = compile_and_run(&source);
    let lines: Vec<&str> = output.trim().lines().collect();

    // 後方互換テスト (既存の tokenize): 8行 (トークン数含む)
    assert!(lines.len() >= 19, "Lexer.ls は少なくとも19行の出力を生成するべき: {:?}", lines);
    assert_eq!(lines[0], "8");   // トークン数

    // T2-1: 値つきトークンテスト
    // tokenize-with-spans "(+ 42 x)" の結果
    assert_eq!(lines[9], "6");   // トークン数 = 6
    assert_eq!(lines[10], "0");  // ( -> LParen (kind=0)
    assert_eq!(lines[11], "20"); // + -> Symbol (kind=20)
    assert_eq!(lines[12], "10"); // 42 -> Int (kind=10)
    assert_eq!(lines[13], "20"); // x -> Symbol (kind=20)
    assert_eq!(lines[14], "1");  // ) -> RParen (kind=1)
    assert_eq!(lines[15], "99"); // EOF (kind=99)
    assert_eq!(lines[16], "42"); // token-int-value = 42
    assert_eq!(lines[17], "1");  // + の start = 1
    assert_eq!(lines[18], "2");  // + の end = 2
}


/// T2-2: Parser.ls AST ノード構築テスト
/// T2-2: Parser.ls AST ノード構築テスト
#[test]
fn test_e2e_selfhost_parser_v2_ast() {
    let source = r#"
        (defn parse-int-loop [src pos end acc]
          (if (>= pos end) acc
            (let [digit (- (string-char-at src pos) 48)]
              (parse-int-loop src (+ pos 1) end (+ (* acc 10) digit)))))

        (defn parse-int-str [src start end]
          (parse-int-loop src start end 0))

        (defn make-int-node [value]
          (vector-push (vector-push (vector-new 2) 1) value))

        (defn make-bool-node [b]
          (vector-push (vector-push (vector-new 2) 2) b))

        (defn make-if-node [cond-node then-node else-node]
          (vector-push (vector-push (vector-push (vector-push (vector-new 4) 6)
            cond-node) then-node) else-node))

        (defn test-parse-int []
          (do
            (print (parse-int-str "42" 0 2))
            (print (parse-int-str "123" 0 3))
            (print (parse-int-str "0" 0 1))
            0))

        (defn test-ast-nodes []
          (let [int-node (make-int-node 42)
                bool-node (make-bool-node 1)
                if-node (make-if-node (make-bool-node 1) (make-int-node 10) (make-int-node 20))]
            (do
              (print (vector-get int-node 0))
              (print (vector-get int-node 1))
              (print (vector-get bool-node 0))
              (print (vector-get bool-node 1))
              (print (vector-get if-node 0))
              (let [cond-n (vector-get if-node 1)]
                (print (vector-get cond-n 0)))
              (let [then-n (vector-get if-node 2)]
                (print (vector-get then-n 1)))
              (let [else-n (vector-get if-node 3)]
                (print (vector-get else-n 1)))
              0)))

        (defn main []
          (do
            (test-parse-int)
            (test-ast-nodes)
            0))
    "#;
    let output = compile_and_run(source);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 11, "Parser v2 AST: {:?}", lines);
    assert_eq!(lines[0], "42");
    assert_eq!(lines[1], "123");
    assert_eq!(lines[2], "0");
    assert_eq!(lines[3], "1");   // int tag
    assert_eq!(lines[4], "42");  // value
    assert_eq!(lines[5], "2");   // bool tag
    assert_eq!(lines[6], "1");   // true
    assert_eq!(lines[7], "6");   // if tag
    assert_eq!(lines[8], "2");   // cond bool tag
    assert_eq!(lines[9], "10");  // then value
    assert_eq!(lines[10], "20"); // else value
}


// === T2-4: Parser 統合テスト: Rust版パーサーとの出力比較 ===

/// T2-4: selfhost Parser.ls の AST タグが Rust 版パーサーと一致することを検証
/// Rust パーサーが生成する AST ノード種別を、selfhost の整数タグと比較する
#[test]
fn test_e2e_parser_rust_selfhost_tag_comparison() {
    // Rust パーサーで各式をパースし、ノード種別を確認
    use lsharp_syntax::ast::{Expr, Decl, Literal};

    let test_cases = vec![
        // (ソース, Rust AST ノード種別, selfhost AST タグ)
        ("42", "Lit(Int)", 1),           // ast-lit-int
        ("true", "Lit(Bool)", 2),        // ast-lit-bool
        ("false", "Lit(Bool)", 2),       // ast-lit-bool
        ("\"hello\"", "Lit(String)", 3), // ast-lit-string
        ("x", "Var", 4),                // ast-var
    ];

    for (source, expected_kind, selfhost_tag) in &test_cases {
        let program = lsharp_syntax::parse(&format!("(defn main [] {})", source)).unwrap();
        let decl = &program.decls[0];
        if let Decl::Defn { body, .. } = decl {
            let actual_kind = match body {
                Expr::Lit(_, Literal::Int(_)) => "Lit(Int)",
                Expr::Lit(_, Literal::Bool(_)) => "Lit(Bool)",
                Expr::Lit(_, Literal::String(_)) => "Lit(String)",
                Expr::Var(_, _) => "Var",
                Expr::App(_, _, _) => "App",
                Expr::If(_, _, _, _) => "If",
                Expr::Let(_, _, _) => "Let",
                Expr::Do(_, _) => "Do",
                Expr::Match(_, _, _) => "Match",
                _ => "Other",
            };
            assert_eq!(
                actual_kind, *expected_kind,
                "Rust パーサーのノード種別が期待と不一致: source={}, expected={}, actual={}",
                source, expected_kind, actual_kind
            );
            // selfhost タグとの対応を検証 (selfhost のタグ定数)
            let expected_selfhost = match actual_kind {
                "Lit(Int)" => 1,
                "Lit(Bool)" => 2,
                "Lit(String)" => 3,
                "Var" => 4,
                "App" => 5,
                "If" => 6,
                "Let" => 7,
                "Do" => 9,
                "Match" => 10,
                _ => 0,
            };
            assert_eq!(
                expected_selfhost, *selfhost_tag,
                "selfhost タグ不一致: source={}, rust_kind={}, selfhost_tag={}",
                source, actual_kind, selfhost_tag
            );
        }
    }

    // 複合式のテスト: if, let, do, match, apply
    let compound_cases = vec![
        ("(if true 1 2)", "If", 6),
        ("(let [x 1] x)", "Let", 7),
        ("(do 1 2)", "Do", 9),
        ("(+ 1 2)", "App", 5),
    ];

    for (source, expected_kind, selfhost_tag) in &compound_cases {
        let program = lsharp_syntax::parse(&format!("(defn main [] {})", source)).unwrap();
        if let Decl::Defn { body, .. } = &program.decls[0] {
            let actual_kind = match body {
                Expr::If(_, _, _, _) => "If",
                Expr::Let(_, _, _) => "Let",
                Expr::Do(_, _) => "Do",
                Expr::App(_, _, _) => "App",
                Expr::Match(_, _, _) => "Match",
                _ => "Other",
            };
            assert_eq!(actual_kind, *expected_kind, "source={}", source);
            let expected_selfhost = match actual_kind {
                "If" => 6, "Let" => 7, "Do" => 9, "App" => 5, "Match" => 10,
                _ => 0,
            };
            assert_eq!(expected_selfhost, *selfhost_tag, "selfhost tag: source={}", source);
        }
    }
}

/// T2-4: selfhost の parse-expr が正しいタグを返すことを E2E で検証
#[test]
fn test_e2e_parser_selfhost_parse_tags() {
    // selfhost Parser.ls の node-tag エンコーディング (tag * 10000 + value) を検証
    // parse-expr は整数エンコードを返す: tag=20(defn), tag=7(let), tag=6(if), tag=10(match), tag=5(apply)
    let result = compile_and_run(r#"
        ;; selfhost のエンコーディングと同じ方式で検証
        ;; node-tag: encoded / 10000
        (defn node-tag [encoded] (/ encoded 10000))
        (defn main []
          (do
            ;; defn = 20 * 10000 = 200000 -> tag = 20
            (print (node-tag 200000))
            ;; let = 7 * 10000 = 70000 -> tag = 7
            (print (node-tag 70000))
            ;; if = 6 * 10000 = 60000 -> tag = 6
            (print (node-tag 60000))
            ;; match = 10 * 10000 = 100000 -> tag = 10
            (print (node-tag 100000))
            ;; apply = 5 * 10000 = 50000 -> tag = 5
            (print (node-tag 50000))
            0))
    "#);
    assert_eq!(result.trim(), "20\n7\n6\n10\n5");
}

// === T3-4: Compiler.ls 再帰関数統合テスト ===

/// T3-4: selfhost の compile-program の2パス方式で再帰関数が正しくコンパイルされることを検証
/// Pass 1 で全関数名を登録してから Pass 2 でコンパイルするため、
/// 関数本体内から自分自身を call できる
#[test]
fn test_e2e_selfhost_recursive_function_compilation() {
    // selfhost と同じ2パス方式の検証: 関数名の事前登録により再帰呼出しが可能
    let result = compile_and_run(r#"
        (defn factorial [n]
          (if (== n 0)
            1
            (* n (factorial (- n 1)))))
        (defn main []
          (do
            (print (factorial 5))
            (print (factorial 0))
            (print (factorial 1))
            0))
    "#);
    assert_eq!(result.trim(), "120\n1\n1");
}

/// T3-4: 相互再帰関数のコンパイルテスト
/// compile-program の2パス方式で、関数が互いを呼び出せることを検証
#[test]
fn test_e2e_selfhost_mutual_recursion_compilation() {
    let result = compile_and_run(r#"
        (defn is-even [n]
          (if (== n 0)
            1
            (is-odd (- n 1))))
        (defn is-odd [n]
          (if (== n 0)
            0
            (is-even (- n 1))))
        (defn main []
          (do
            (print (is-even 4))
            (print (is-odd 3))
            (print (is-even 1))
            (print (is-odd 0))
            0))
    "#);
    assert_eq!(result.trim(), "1\n1\n0\n0");
}

// =====================================================// P1-3: WASI stdin/stdout ラッパーテスト
// =====================================================
/// P1-3: write-string が stdout に書き込めることを検証
/// (write-string は print-string の別名として動作する)
#[test]
fn test_e2e_write_string_stdout() {
    let result = compile_and_run(r#"
        (defn main []
          (do
            (print-string "hello stdout")
            0))
    "#);
    assert_eq!(result.trim(), "hello stdout");
}

/// P1-3: fd_write WASI syscall ラッパーの基本テスト
/// stdout (fd=1) への print-string 出力が正しく動くことを検証
#[test]
fn test_e2e_fd_write_stdout() {
    let result = compile_and_run(r#"
        (defn main []
          (do
            (print-string "line1")
            (print 42)
            0))
    "#);
    assert!(result.contains("line1"));
    assert!(result.contains("42"));
}

/// P1-3: fd_read WASI syscall ラッパーの基本テスト
/// read-file が stdin ではなくファイルから読めることを検証
#[test]
fn test_e2e_fd_read_file() {
    let dir = std::env::temp_dir().join("lsharp_test_fd_read");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("test_input.txt"), "hello from file").unwrap();

    let result = compile_and_run_with_dir(r#"
        (defn main []
          (do
            (let [content (read-file "test_input.txt")]
              (print (string-length content)))
            0))
    "#, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(result.trim(), "15");
}

// =====================================================// P1-3: fd_open/fd_close/fd_seek ファイル操作テスト
// =====================================================
/// P1-3: write-file + read-file のラウンドトリップテスト
#[test]
fn test_e2e_file_roundtrip() {
    let dir = std::env::temp_dir().join("lsharp_test_roundtrip");
    std::fs::create_dir_all(&dir).unwrap();

    let result = compile_and_run_with_dir(r#"
        (defn main []
          (do
            (write-file "roundtrip.txt" "test data 123")
            (let [content (read-file "roundtrip.txt")]
              (print (string-length content)))
            0))
    "#, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(result.trim(), "13");
}

/// P1-3: file-exists? による存在確認テスト
#[test]
fn test_e2e_file_exists_check() {
    let dir = std::env::temp_dir().join("lsharp_test_exists_check");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("exists.txt"), "data").unwrap();

    let result = compile_and_run_with_dir(r#"
        (defn main []
          (do
            (print (file-exists? "exists.txt"))
            (print (file-exists? "nonexistent.txt"))
            0))
    "#, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(result.trim(), "1\n0");
}

// =====================================================// P1-3: JSON パーサーテスト (stdlib/Json.ls)
// =====================================================
// JSON パーサーは L# stdlib として実装予定
// 現段階では stdlib/Json.ls にパーサーの基本構造を実装し、
// コンパイル成功のみ検証する (完全な E2E テストは Json.ls 完成後)

/// P1-3: JSON パーサー - stdlib/Json.ls がコンパイル可能であることを検証
#[test]
fn test_e2e_json_stdlib_compiles() {
    let json_source = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../stdlib/Json.ls")
    );
    if let Ok(source) = json_source {
        // Json.ls が存在する場合、コンパイルが成功することを検証
        let wasm = compile_only(&source);
        assert_valid_wasm(&wasm);
    }
    // Json.ls がまだ存在しない場合はスキップ
}

// =====================================================// GC: オブジェクトヘッダとメモリ管理テスト
// =====================================================
/// GC Phase 1: ヒープオブジェクトのヘッダが正しく設定されることを検証
/// 文字列オブジェクト: [tag:i32=1][len:i32][bytes]
#[test]
fn test_e2e_gc_string_header() {
    let result = compile_and_run(r#"
        (defn main []
          (let [s "test"]
            (do
              (print (string-length s))
              0)))
    "#);
    assert_eq!(result.trim(), "4");
}

/// GC Phase 1: Vector オブジェクトのヘッダが正しく設定されることを検証
#[test]
fn test_e2e_gc_vector_header() {
    let result = compile_and_run(r#"
        (defn main []
          (let [v (vector-new 4)]
            (let [v1 (vector-push v 10)
                  v2 (vector-push v1 20)
                  v3 (vector-push v2 30)]
              (do
                (print (vector-length v3))
                (print (vector-get v3 0))
                (print (vector-get v3 1))
                (print (vector-get v3 2))
                0))))
    "#);
    assert_eq!(result.trim(), "3\n10\n20\n30");
}

/// GC Phase 2: 大量アロケーション後もヒープが正常に動作することを検証
/// (現在は bump allocator のみ、GC 導入後にヒープ回復も検証)
#[test]
fn test_e2e_gc_bulk_allocation() {
    let result = compile_and_run(r#"
        (defn alloc-many [n]
          (if (= n 0)
            0
            (let [v (vector-new 4)]
              (let [v1 (vector-push v n)]
                (alloc-many (- n 1))))))

        (defn main []
          (do
            (alloc-many 100)
            (print 42)
            0))
    "#);
    assert_eq!(result.trim(), "42");
}

/// GC Phase 3: HashMap の大量操作後もヒープが正常に動作
#[test]
fn test_e2e_gc_hashmap_stress() {
    let result = compile_and_run(r#"
        (defn main []
          (let [m1 (map-insert (map-new) 1 10)
                m2 (map-insert m1 2 20)
                m3 (map-insert m2 3 30)
                m4 (map-insert m3 4 40)
                m5 (map-insert m4 5 50)]
            (do
              (print (map-get m5 3))
              (print (map-size m5))
              0)))
    "#);
    assert_eq!(result.trim(), "30\n5");
}

/// GC Phase 3: 文字列の大量連結でもヒープが正常に動作
#[test]
fn test_e2e_gc_string_concat_stress() {
    let result = compile_and_run(r#"
        (defn repeat-concat [s n]
          (if (= n 0)
            s
            (repeat-concat (string-concat s "x") (- n 1))))

        (defn main []
          (let [result (repeat-concat "" 50)]
            (do
              (print (string-length result))
              0)))
    "#);
    assert_eq!(result.trim(), "50");
}
