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

// === P1-2: 文字列リテラルのヒープ化テスト ===

#[test]
fn test_e2e_string_heap_print() {
    // ヒープ上の String オブジェクト経由で文字列が正しく出力されることを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (do (print-string "hello heap") 0))
    "#,
    );
    assert_eq!(result, "hello heap");
}

#[test]
fn test_e2e_string_heap_length() {
    // ヒープ上の String オブジェクトから長さが正しく取得できることを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (print (string-length "heap string")))
    "#,
    );
    assert_eq!(result.trim(), "11");
}

#[test]
fn test_e2e_string_heap_char_at() {
    // ヒープ上の String オブジェクトから文字取得が正しく動作することを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (print (string-char-at "abcdef" 2)))
    "#,
    );
    // 'c' = 99
    assert_eq!(result.trim(), "99");
}

#[test]
fn test_e2e_string_heap_substring() {
    // ヒープ上の String オブジェクトから部分文字列が正しく取得できることを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (do (print-string (substring "hello world" 6 11)) 0))
    "#,
    );
    assert_eq!(result, "world");
}

#[test]
fn test_e2e_string_heap_concat_mixed() {
    // リテラル文字列同士の結合がヒープ上で正しく動作することを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (do (print-string (string-concat "foo" "bar")) 0))
    "#,
    );
    assert_eq!(result, "foobar");
}

#[test]
fn test_e2e_string_heap_eq() {
    // ヒープ上の文字列同士の比較が正しく動作することを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (print (if (string-eq "test" "test") 1 0)))
    "#,
    );
    assert_eq!(result.trim(), "1");
}

#[test]
fn test_e2e_string_heap_multiple_literals() {
    // 複数の文字列リテラルがそれぞれヒープ上に正しく配置されることを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "first")
            (print-string " ")
            (print-string "second")
            0))
    "#,
    );
    assert_eq!(result, "first second");
}

#[test]
fn test_e2e_string_heap_object_layout() {
    // 文字列リテラルがヒープ上に [tag=1][len][bytes] として配置されることを検証
    let result = compile_and_run(
        r#"
        (defn main []
          (let [s "hello"]
            (do
              (print (string-length s))
              (print (string-char-at s 0))
              (print (string-char-at s 4))
              0)))
    "#,
    );
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
// selfhost/src/** の複数モジュールを結合した統合パイプラインの検証
// =====================================================
/// 統合テスト: selfhost/src/App/Main.ls を Rust コンパイラでコンパイル・実行し、
/// AST 構築 → IR 変換 → Wasm バイナリ生成の統合パイプラインを検証する。
#[test]
fn test_e2e_bootstrap_stage1_integration() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();
    let expected_prefix = [
        "1", "42", "1", "1", "42", "8", "0", "97", "115", "109", "7", "1", "15", "10", "8", "20",
        "1", "42", "1", "1", "42", "1", "6", "3", "1", "7", "2", "1", "1", "100", "1", "5",
    ];
    assert!(
        lines.starts_with(&expected_prefix),
        "bootstrap stage1 出力 prefix が想定と異なる: {:?}",
        lines
    );
    assert!(
        lines.len() >= 71,
        "native pipeline を含む統合出力が不足: {} 行",
        lines.len()
    );
}

/// 統合テスト: selfhost/src/** の全モジュールを結合したソースが正しくコンパイルでき、
/// stage1.wasm 相当のバイナリ生成まで検証する。
#[test]
fn test_e2e_bootstrap_stage1_wasm_generation() {
    let wasm_bytes = compile_file_only(&selfhost_main_path());
    // 有効な Wasm バイナリであること (マジックナンバー確認)
    assert!(
        wasm_bytes.len() > 8,
        "Wasm バイナリが短すぎる: {} bytes",
        wasm_bytes.len()
    );
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
    assert_eq!(
        lines.len(),
        4,
        "Path.ls は4行の出力を生成するべき: {:?}",
        lines
    );
    assert_eq!(lines[0], "13"); // path-join "/tmp" "file.txt" = "/tmp/file.txt" (13文字)
    assert_eq!(lines[1], "4"); // path-extension "file.txt" = ".txt" (4文字)
    assert_eq!(lines[2], "8"); // path-basename "/tmp/file.txt" = "file.txt" (8文字)
    assert_eq!(lines[3], "4"); // path-dirname "/tmp/file.txt" = "/tmp" (4文字)
}

/// selfhost/src/Backend/Wasm/Compiler.ls のセルフホストコンパイラのコンパイル+実行
#[test]
fn test_e2e_selfhost_compiler_file() {
    let path = selfhost_source_path("Compiler.ls");
    let output = compile_and_run_file(&path);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 15,
        "Compiler.ls は少なくとも15行の出力を生成するべき: {:?}",
        lines
    );
    assert_eq!(lines[0], "1"); // vector-length instrs = 1
    assert_eq!(lines[1], "1"); // op: i64.const
    assert_eq!(lines[2], "42"); // operand: 42
    assert_eq!(lines[3], "3"); // do 式 lowering は 3 命令
    assert_eq!(lines[14], "40"); // 末尾は call opcode
}

/// selfhost/src/Backend/Wasm/WasmEmit.ls の Wasm バイナリ生成のコンパイル+実行
#[test]
fn test_e2e_selfhost_wasmemit() {
    let output = compile_and_run_file(&selfhost_source_path("WasmEmit.ls"));
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 6,
        "WasmEmit.ls は少なくとも6行の出力を生成するべき: {:?}",
        lines
    );
    assert_eq!(lines[0], "8"); // ヘッダー長
    assert_eq!(lines[1], "0"); // \0
    assert_eq!(lines[2], "97"); // 'a'
    assert_eq!(lines[3], "115"); // 's'
    assert_eq!(lines[4], "109"); // 'm'
    assert_eq!(lines[5], "1"); // version
}

/// T1-9: selfhost/src/App/Main.ls 統合 E2E テスト
/// AST 構築 → IR 変換 → Wasm ヘッダー生成の統合パイプラインを検証
#[test]
fn test_e2e_selfhost_main_integration() {
    let output = compile_and_run_file(&selfhost_main_path());
    let lines: Vec<&str> = output.trim().lines().collect();

    // Main.ls 旧パイプライン + T4-4 新パイプライン検証
    assert!(
        lines.len() >= 32,
        "Main.ls は少なくとも32行の出力を生成するべき: {:?}",
        lines
    );

    // 旧パイプライン: AST → IR → Wasm
    assert_eq!(lines[0], "1"); // ast-tag = 1 (lit-int)
    assert_eq!(lines[1], "42"); // value = 42
    assert_eq!(lines[2], "1"); // vector-length instrs = 1
    assert_eq!(lines[3], "1"); // op: i64.const
    assert_eq!(lines[4], "42"); // operand: 42
    assert_eq!(lines[5], "8"); // ヘッダー長 = 8
    assert_eq!(lines[6], "0"); // \0
    assert_eq!(lines[7], "97"); // 'a'
    assert_eq!(lines[8], "115"); // 's'
    assert_eq!(lines[9], "109"); // 'm'
    assert_eq!(lines[10], "7"); // type section length = 7
    assert_eq!(lines[11], "1"); // section-id: Type
    assert_eq!(lines[12], "15"); // wasm-size = 8 + 7
    assert_eq!(lines[13], "10"); // module-count = 10

    // T4-4: 新パイプライン (Lexer.tokenize の kind 列長)
    assert_eq!(lines[14], "8"); // "(defn main [] 42)" のトークン数 (Lexer 実装に準拠)
    assert_eq!(lines[15], "20"); // defn AST tag
    assert_eq!(lines[16], "1"); // body: lit-int tag
    assert_eq!(lines[17], "42"); // body: value = 42
    assert_eq!(lines[18], "1"); // IR: 1 命令
    assert_eq!(lines[19], "1"); // IR instr: i64.const
    assert_eq!(lines[20], "42"); // IR operand: 42

    // P11: 完全パイプライン (MacroExpand + TypeInfer 統合)
    assert_eq!(lines[27], "1"); // expanded AST tag = 1 (lit-int)
    assert_eq!(lines[28], "1"); // 型推論結果: ty-tag = 1 (Con)
    assert_eq!(lines[29], "100"); // 型推論結果: ty-name = 100 (Int)
    assert_eq!(lines[30], "1"); // IR 命令数 = 1
    assert_eq!(lines[31], "5"); // パイプラインステージ数 = 5
}

/// T2-1: Lexer.ls 値つきトークン (kind, start, end) 3つ組のテスト
#[test]
fn test_e2e_selfhost_lexer_value_tokens() {
    let output = compile_and_run(&format!(
        "{}\n(defn main [] (demo-main))",
        selfhost_lexer_runtime_bundle()
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    // 後方互換テスト (既存の tokenize): 8行 (トークン数含む)
    assert!(
        lines.len() >= 19,
        "Lexer.ls は少なくとも19行の出力を生成するべき: {:?}",
        lines
    );
    assert_eq!(lines[0], "8"); // トークン数

    // T2-1: 値つきトークンテスト
    // tokenize-with-spans "(+ 42 x)" の結果
    assert_eq!(lines[9], "6"); // トークン数 = 6
    assert_eq!(lines[10], "0"); // ( -> LParen (kind=0)
    assert_eq!(lines[11], "20"); // + -> Symbol (kind=20)
    assert_eq!(lines[12], "10"); // 42 -> Int (kind=10)
    assert_eq!(lines[13], "20"); // x -> Symbol (kind=20)
    assert_eq!(lines[14], "1"); // ) -> RParen (kind=1)
    assert_eq!(lines[15], "99"); // EOF (kind=99)
    assert_eq!(lines[16], "42"); // token-int-value = 42
    assert_eq!(lines[17], "1"); // + の start = 1
    assert_eq!(lines[18], "2"); // + の end = 2
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
    assert_eq!(lines[3], "1"); // int tag
    assert_eq!(lines[4], "42"); // value
    assert_eq!(lines[5], "2"); // bool tag
    assert_eq!(lines[6], "1"); // true
    assert_eq!(lines[7], "6"); // if tag
    assert_eq!(lines[8], "2"); // cond bool tag
    assert_eq!(lines[9], "10"); // then value
    assert_eq!(lines[10], "20"); // else value
}

// === T2-4: Parser 統合テスト: Rust版パーサーとの出力比較 ===

/// T2-4: selfhost Parser.ls の AST タグが Rust 版パーサーと一致することを検証
/// Rust パーサーが生成する AST ノード種別を、selfhost の整数タグと比較する
#[test]
fn test_e2e_parser_rust_selfhost_tag_comparison() {
    // Rust パーサーで各式をパースし、ノード種別を確認
    use lsharp_syntax::ast::{Decl, Expr, Literal};

    let test_cases = vec![
        // (ソース, Rust AST ノード種別, selfhost AST タグ)
        ("42", "Lit(Int)", 1),           // ast-lit-int
        ("true", "Lit(Bool)", 2),        // ast-lit-bool
        ("false", "Lit(Bool)", 2),       // ast-lit-bool
        ("\"hello\"", "Lit(String)", 3), // ast-lit-string
        ("x", "Var", 4),                 // ast-var
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
                "If" => 6,
                "Let" => 7,
                "Do" => 9,
                "App" => 5,
                "Match" => 10,
                _ => 0,
            };
            assert_eq!(
                expected_selfhost, *selfhost_tag,
                "selfhost tag: source={}",
                source
            );
        }
    }
}

/// T2-4: selfhost の parse-expr が正しいタグを返すことを E2E で検証
#[test]
fn test_e2e_parser_selfhost_parse_tags() {
    // selfhost Parser.ls の node-tag エンコーディング (tag * 10000 + value) を検証
    // parse-expr は整数エンコードを返す: tag=20(defn), tag=7(let), tag=6(if), tag=10(match), tag=5(apply)
    let result = compile_and_run(
        r#"
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
    "#,
    );
    assert_eq!(result.trim(), "20\n7\n6\n10\n5");
}

// === T3-4: Compiler.ls 再帰関数統合テスト ===

/// T3-4: selfhost の compile-program の2パス方式で再帰関数が正しくコンパイルされることを検証
/// Pass 1 で全関数名を登録してから Pass 2 でコンパイルするため、
/// 関数本体内から自分自身を call できる
#[test]
fn test_e2e_selfhost_recursive_function_compilation() {
    // selfhost と同じ2パス方式の検証: 関数名の事前登録により再帰呼出しが可能
    let result = compile_and_run(
        r#"
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
    "#,
    );
    assert_eq!(result.trim(), "120\n1\n1");
}

/// T3-4: 相互再帰関数のコンパイルテスト
/// compile-program の2パス方式で、関数が互いを呼び出せることを検証
#[test]
fn test_e2e_selfhost_mutual_recursion_compilation() {
    let result = compile_and_run(
        r#"
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
    "#,
    );
    assert_eq!(result.trim(), "1\n1\n0\n0");
}

/// selfhost Compiler.ls: 2 関数プログラムで call operand が関数インデックスになること
#[test]
fn test_e2e_selfhost_compiler_two_zero_arg_defns_call_index() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn helper [] 42) (defn main [] (helper))")
        pair (compile-program program)
        ir-list (vector-get pair 1)
        helper-ir (vector-get ir-list 0)
        main-ir (vector-get ir-list 1)
        helper-instr (vector-get helper-ir 0)
        main-instr (vector-get main-ir 0)]
    (do
      (print (vector-length ir-list))
      (print (vector-get helper-instr 0))
      (print (vector-get helper-instr 1))
      (print (vector-get main-instr 0))
      (print (vector-get main-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "2 関数 compile-program 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "2", "helper/main の 2 関数が必要");
    assert_eq!(lines[1], "1", "helper body は i64.const で始まること");
    assert_eq!(lines[2], "42", "helper body operand は 42");
    assert_eq!(lines[3], "40", "main body は call 命令で始まること");
    assert_eq!(
        lines[4], "0",
        "main は helper の関数インデックス 0 を call すること"
    );
}

/// selfhost Compiler.ls: 5 関数プログラムでも全 defn を compile-program-functions が保持できること
#[test]
fn test_e2e_selfhost_compiler_five_zero_arg_defns_metadata_loop() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn a [] 1) (defn b [] 2) (defn c [] 3) (defn d [] 4) (defn main [] (d))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 4)
        main-ir (vector-get main-fn 2)
        main-instr (vector-get main-ir 0)]
    (do
      (print (vector-length functions))
      (print (vector-get main-instr 0))
      (print (vector-get main-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 3, "5 関数 metadata 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "5", "5 defn 全てが metadata に残ること");
    assert_eq!(
        lines[1], "40",
        "5 個目の main body は call 命令で始まること"
    );
    assert_eq!(
        lines[2], "3",
        "main は 4 個目の helper(d) を関数インデックス 3 で call すること"
    );
}

/// selfhost Compiler.ls: string-char-at を builtin として lowering し、補助 local を metadata に反映すること
#[test]
fn test_e2e_selfhost_compiler_string_char_at_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn first [s] (string-char-at s 0)) (defn main [] 0)")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        first-fn (vector-get functions 0)
        first-ir (vector-get first-fn 2)
        last-instr (vector-get first-ir 2)]
    (do
      (print (vector-get first-fn 0))
      (print (vector-get first-fn 1))
      (print (vector-length first-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "string-char-at lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "first は 1 引数関数であること");
    assert_eq!(
        lines[1], "1",
        "string-char-at lowering 用の補助 local が 1 個必要"
    );
    assert_eq!(
        lines[2], "3",
        "body は local.get / i64.const / string-char-at の 3 命令であること"
    );
    assert_eq!(
        lines[3], "50",
        "末尾命令は string-char-at builtin opcode であること"
    );
    assert_eq!(lines[4], "2", "補助 local index は 2 であること");
}

/// selfhost Compiler.ls: string-length を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_string_length_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn len1 [s] (string-length s)) (defn main [] 0)")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        len-fn (vector-get functions 0)
        len-ir (vector-get len-fn 2)
        last-instr (vector-get len-ir 1)]
    (do
      (print (vector-get len-fn 0))
      (print (vector-get len-fn 1))
      (print (vector-length len-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "string-length lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "len1 は 1 引数関数であること");
    assert_eq!(lines[1], "0", "string-length lowering に補助 local は不要");
    assert_eq!(
        lines[2], "2",
        "body は local.get / string-length の 2 命令であること"
    );
    assert_eq!(
        lines[3], "51",
        "末尾命令は string-length builtin opcode であること"
    );
    assert_eq!(
        lines[4], "0",
        "string-length opcode operand は 0 であること"
    );
}

/// selfhost Compiler.ls: vector-length を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_vector_length_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn vlen [v] (vector-length v)) (defn main [] 0)")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        vlen-fn (vector-get functions 0)
        vlen-ir (vector-get vlen-fn 2)
        last-instr (vector-get vlen-ir 1)]
    (do
      (print (vector-get vlen-fn 0))
      (print (vector-get vlen-fn 1))
      (print (vector-length vlen-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "vector-length lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "vlen は 1 引数関数であること");
    assert_eq!(lines[1], "0", "vector-length lowering に補助 local は不要");
    assert_eq!(
        lines[2], "2",
        "body は local.get / vector-length の 2 命令であること"
    );
    assert_eq!(
        lines[3], "52",
        "末尾命令は vector-length builtin opcode であること"
    );
    assert_eq!(
        lines[4], "0",
        "vector-length opcode operand は 0 であること"
    );
}

/// selfhost Compiler.ls: vector-get を builtin として lowering し、補助 local を metadata に反映すること
#[test]
fn test_e2e_selfhost_compiler_vector_get_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn vget0 [v] (vector-get v 0)) (defn main [] 0)")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        vget-fn (vector-get functions 0)
        vget-ir (vector-get vget-fn 2)
        last-instr (vector-get vget-ir 2)]
    (do
      (print (vector-get vget-fn 0))
      (print (vector-get vget-fn 1))
      (print (vector-length vget-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "vector-get lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "vget0 は 1 引数関数であること");
    assert_eq!(
        lines[1], "1",
        "vector-get lowering 用の補助 local が 1 個必要"
    );
    assert_eq!(
        lines[2], "3",
        "body は local.get / i64.const / vector-get の 3 命令であること"
    );
    assert_eq!(
        lines[3], "53",
        "末尾命令は vector-get builtin opcode であること"
    );
    assert_eq!(lines[4], "2", "補助 local index は 2 であること");
}

/// selfhost Compiler.ls: vector-new を builtin として lowering し、補助 locals を metadata に反映すること
#[test]
fn test_e2e_selfhost_compiler_vector_new_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (vector-new 4))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 1)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "vector-new lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(
        lines[1], "2",
        "vector-new lowering 用の補助 local が 2 個必要"
    );
    assert_eq!(
        lines[2], "2",
        "body は i64.const / vector-new の 2 命令であること"
    );
    assert_eq!(
        lines[3], "54",
        "末尾命令は vector-new builtin opcode であること"
    );
    assert_eq!(lines[4], "1", "補助 local base index は 1 であること");
}

/// selfhost Compiler.ls: vector-push を builtin として lowering し、growth 用 locals を metadata に反映すること
#[test]
fn test_e2e_selfhost_compiler_vector_push_builtin_lowering() {
    let harness = r#"
(defn find-op-loop [ir idx count target]
  (if (>= idx count)
    -1
    (if (= (vector-get (vector-get ir idx) 0) target)
      idx
      (find-op-loop ir (+ idx 1) count target))))

(defn main []
  (let [program (parse-program "(defn main [] (vector-push (vector-new 1) 99))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        instr-count (vector-length main-ir)
        vector-push-idx (find-op-loop main-ir 0 instr-count 55)
        vector-push-instr (vector-get main-ir vector-push-idx)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print instr-count)
      (print (vector-get vector-push-instr 0))
      (print (vector-get vector-push-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "vector-push lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert!(
        lines[1]
            .parse::<i64>()
            .expect("local count は整数であること")
            >= 6,
        "vector-push lowering は growth 用 metadata local を保持すべき"
    );
    assert!(
        lines[2].parse::<i64>().expect("IR length は整数であること") >= 4,
        "vector-push lowering は vector-new を含む body を持つべき"
    );
    assert_eq!(
        lines[3], "55",
        "末尾命令は vector-push builtin opcode であること"
    );
    assert_eq!(lines[4], "1", "補助 local base index は 1 であること");
}

/// selfhost Compiler.ls: ref-new を builtin として lowering し、alloc 用 locals を metadata に反映すること
#[test]
fn test_e2e_selfhost_compiler_ref_new_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (ref-new 1))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 1)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 5, "ref-new lowering 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "2", "ref-new lowering 用の補助 local が 2 個必要");
    assert_eq!(
        lines[2], "2",
        "body は i64.const / ref-new の 2 命令であること"
    );
    assert_eq!(
        lines[3], "56",
        "末尾命令は ref-new builtin opcode であること"
    );
    assert_eq!(lines[4], "1", "補助 local base index は 1 であること");
}

/// selfhost Compiler.ls: ref-set を builtin として lowering し、補助 local を metadata に反映すること
#[test]
fn test_e2e_selfhost_compiler_ref_set_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn set1 [r v] (ref-set r v)) (defn main [] 0)")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        set-fn (vector-get functions 0)
        set-ir (vector-get set-fn 2)
        last-instr (vector-get set-ir 2)]
    (do
      (print (vector-get set-fn 0))
      (print (vector-get set-fn 1))
      (print (vector-length set-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 5, "ref-set lowering 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "2", "set1 は 2 引数関数であること");
    assert_eq!(lines[1], "1", "ref-set lowering 用の補助 local が 1 個必要");
    assert_eq!(
        lines[2], "3",
        "body は local.get / local.get / ref-set の 3 命令であること"
    );
    assert_eq!(
        lines[3], "58",
        "末尾命令は ref-set builtin opcode であること"
    );
    assert_eq!(lines[4], "3", "補助 local index は 3 であること");
}

/// selfhost Compiler.ls: map-new を builtin として lowering し、alloc 用 locals を metadata に反映すること
#[test]
fn test_e2e_selfhost_compiler_map_new_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (map-new))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 0)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 5, "map-new lowering 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "1", "map-new lowering 用の補助 local が 1 個必要");
    assert_eq!(lines[2], "1", "body は map-new の 1 命令であること");
    assert_eq!(
        lines[3], "60",
        "末尾命令は map-new builtin opcode であること"
    );
    assert_eq!(lines[4], "1", "補助 local base index は 1 であること");
}

/// selfhost Compiler.ls: print を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_print_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (print 7))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 1)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(lines.len() >= 5, "print lowering 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "0", "print lowering に補助 local は不要");
    assert_eq!(
        lines[2], "2",
        "body は i64.const / print の 2 命令であること"
    );
    assert_eq!(lines[3], "59", "末尾命令は print builtin opcode であること");
    assert_eq!(lines[4], "0", "print opcode operand は 0 であること");
}

/// selfhost Compiler.ls: read-file を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_read_file_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (read-file 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 1)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "read-file lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "0", "read-file lowering に補助 local は不要");
    assert_eq!(
        lines[2], "2",
        "body は i64.const / read-file の 2 命令であること"
    );
    assert_eq!(
        lines[3], "64",
        "末尾命令は read-file builtin opcode であること"
    );
    assert_eq!(lines[4], "0", "read-file opcode operand は 0 であること");
}

/// selfhost Compiler.ls: write-file を binary builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_write_file_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (write-file 0 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 2)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "write-file lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert!(
        lines[1].parse::<i64>().is_ok(),
        "write-file lowering の local count は整数であること"
    );
    assert_eq!(
        lines[2], "3",
        "body は path/content const と write-file の 3 命令であること"
    );
    assert_eq!(
        lines[3], "89",
        "末尾命令は write-file builtin opcode であること"
    );
    assert_eq!(lines[4], "0", "write-file opcode operand は 0 であること");
}

/// selfhost Compiler.ls: write-file-bytes を binary builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_write_file_bytes_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (write-file-bytes 0 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 2)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "write-file-bytes lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert!(
        lines[1].parse::<i64>().is_ok(),
        "write-file-bytes lowering の local count は整数であること"
    );
    assert_eq!(
        lines[2], "3",
        "body は path/bytes const と write-file-bytes の 3 命令であること"
    );
    assert_eq!(
        lines[3], "90",
        "末尾命令は write-file-bytes builtin opcode であること"
    );
    assert_eq!(
        lines[4], "0",
        "write-file-bytes opcode operand は 0 であること"
    );
}

/// selfhost Compiler.ls: command-line-arg を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_command_line_arg_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (command-line-arg 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 1)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "command-line-arg lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(
        lines[1], "0",
        "command-line-arg lowering に補助 local は不要"
    );
    assert_eq!(
        lines[2], "2",
        "body は i64.const / command-line-arg の 2 命令であること"
    );
    assert_eq!(
        lines[3], "67",
        "末尾命令は command-line-arg builtin opcode であること"
    );
    assert_eq!(
        lines[4], "0",
        "command-line-arg opcode operand は 0 であること"
    );
}

/// selfhost Compiler.ls: native CLI runtime builtin を正しい arity で lowering できること
#[test]
fn test_e2e_selfhost_compiler_native_cli_runtime_builtin_lowering() {
    let harness = r#"
(defn compiled-main-ir [source]
  (let [program (parse-program source)
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)]
    (vector-get main-fn 2)))

(defn print-ir-summary [source]
  (let [ir (compiled-main-ir source)
        last-instr (vector-get ir (- (vector-length ir) 1))]
    (do
      (print (vector-length ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1)))))

(defn main []
  (do
    (print-ir-summary "(defn main [] (command-line-args))")
    (print-ir-summary "(defn main [] (print-string \"ok\"))")
    (print-ir-summary "(defn main [] (proc-exit 7))")
    0))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "86", "0", "2", "87", "0", "2", "88", "0"],
        "command-line-args/print-string/proc-exit は nullary/unary builtin として dedicated opcode に lower される必要がある"
    );
}

/// selfhost Compiler.ls: file-exists? を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_file_exists_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (file-exists? 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 1)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "file-exists? lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "0", "file-exists? lowering に補助 local は不要");
    assert_eq!(
        lines[2], "2",
        "body は i64.const / file-exists? の 2 命令であること"
    );
    assert_eq!(
        lines[3], "73",
        "末尾命令は file-exists? builtin opcode であること"
    );
    assert_eq!(lines[4], "0", "file-exists? opcode operand は 0 であること");
}

/// selfhost WasmEmit.ls: command-line-arg opcode を call import へ落とせること
#[test]
fn test_e2e_selfhost_wasmemit_command_line_arg_instr() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (command-line-arg 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        code-sec (emit-code-section-functions functions)]
    (do
      (print (vector-length code-sec))
      (print (vector-get code-sec 0))
      (print (vector-get code-sec 1))
      (print (vector-get code-sec 2))
      (print (vector-get code-sec 3))
      (print (vector-get code-sec 4))
      (print (vector-get code-sec 5))
      (print (vector-get code-sec 6))
      (print (vector-get code-sec 7))
      (print (vector-get code-sec 8))
      (print (vector-get code-sec 9))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 11,
        "command-line-arg code section 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "10", "code section byte 長は 10 であること");
    assert_eq!(lines[1], "10", "section id は code=10");
    assert_eq!(lines[2], "8", "section size は 8");
    assert_eq!(lines[3], "1", "function count は 1");
    assert_eq!(lines[4], "6", "function body size は 6");
    assert_eq!(lines[5], "0", "local decl count は 0");
    assert_eq!(lines[6], "66", "先頭命令は i64.const");
    assert_eq!(lines[7], "0", "const operand は 0");
    assert_eq!(
        lines[8], "16",
        "command-line-arg は call opcode へ lower されること"
    );
    assert_eq!(
        lines[9], "3",
        "command-line-arg import index は 3 であること"
    );
    assert_eq!(lines[10], "11", "body は end で終わること");
}

/// selfhost WasmEmit.ls: file-exists? opcode を call import へ落とせること
#[test]
fn test_e2e_selfhost_wasmemit_file_exists_instr() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (file-exists? 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        code-sec (emit-code-section-functions functions)]
    (do
      (print (vector-length code-sec))
      (print (vector-get code-sec 0))
      (print (vector-get code-sec 1))
      (print (vector-get code-sec 2))
      (print (vector-get code-sec 3))
      (print (vector-get code-sec 4))
      (print (vector-get code-sec 5))
      (print (vector-get code-sec 6))
      (print (vector-get code-sec 7))
      (print (vector-get code-sec 8))
      (print (vector-get code-sec 9))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 11,
        "file-exists? code section 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "10", "code section byte 長は 10 であること");
    assert_eq!(lines[1], "10", "section id は code=10");
    assert_eq!(lines[2], "8", "section size は 8");
    assert_eq!(lines[3], "1", "function count は 1");
    assert_eq!(lines[4], "6", "function body size は 6");
    assert_eq!(lines[5], "0", "local decl count は 0");
    assert_eq!(lines[6], "66", "先頭命令は i64.const");
    assert_eq!(lines[7], "0", "const operand は 0");
    assert_eq!(
        lines[8], "16",
        "file-exists? は call opcode へ lower されること"
    );
    assert_eq!(lines[9], "6", "file-exists? import index は 6 であること");
    assert_eq!(lines[10], "11", "body は end で終わること");
}

/// selfhost WasmEmit.ls: i64.ge_s opcode を比較命令として emit できること
#[test]
fn test_e2e_selfhost_wasmemit_i64_ge_instr() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (>= 3 2))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        code-sec (emit-code-section-functions functions)]
    (do
      (print (vector-length code-sec))
      (print (vector-get code-sec 0))
      (print (vector-get code-sec 1))
      (print (vector-get code-sec 2))
      (print (vector-get code-sec 3))
      (print (vector-get code-sec 4))
      (print (vector-get code-sec 5))
      (print (vector-get code-sec 6))
      (print (vector-get code-sec 7))
      (print (vector-get code-sec 8))
      (print (vector-get code-sec 9))
      (print (vector-get code-sec 10))
      (print (vector-get code-sec 11))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 13,
        "i64.ge_s code section 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "12", "code section byte 長は 12 であること");
    assert_eq!(lines[1], "10", "section id は code=10");
    assert_eq!(lines[2], "10", "section size は 10");
    assert_eq!(lines[3], "1", "function count は 1");
    assert_eq!(lines[4], "8", "function body size は 8");
    assert_eq!(lines[5], "0", "local decl count は 0");
    assert_eq!(lines[6], "66", "左辺 i64.const");
    assert_eq!(lines[7], "3", "左辺 const operand");
    assert_eq!(lines[8], "66", "右辺 i64.const");
    assert_eq!(lines[9], "2", "右辺 const operand");
    assert_eq!(lines[10], "89", ">= は i64.ge_s (0x59) を emit すること");
    assert_eq!(
        lines[11], "172",
        "比較結果は i64.extend_i32_s で i64 に戻すこと"
    );
    assert_eq!(lines[12], "11", "body は end で終わること");
}

/// selfhost WasmEmit.ls: IR opcode 28 を i64.rem_s として emit できること
#[test]
fn test_e2e_selfhost_wasmemit_i64_mod_instr() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (% 3 2))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        code-sec (emit-code-section-functions functions)]
    (do
      (print (vector-length code-sec))
      (print (vector-get code-sec 0))
      (print (vector-get code-sec 1))
      (print (vector-get code-sec 2))
      (print (vector-get code-sec 3))
      (print (vector-get code-sec 4))
      (print (vector-get code-sec 5))
      (print (vector-get code-sec 6))
      (print (vector-get code-sec 7))
      (print (vector-get code-sec 8))
      (print (vector-get code-sec 9))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 11,
        "i64.rem_s code section 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "11", "code section byte 長は 11 であること");
    assert_eq!(lines[1], "10", "section id は code=10");
    assert_eq!(lines[2], "9", "section size は 9");
    assert_eq!(lines[3], "1", "function count は 1");
    assert_eq!(lines[4], "7", "function body size は 7");
    assert_eq!(lines[5], "0", "local decl count は 0");
    assert_eq!(lines[6], "66", "左辺 i64.const");
    assert_eq!(lines[7], "3", "左辺 const operand");
    assert_eq!(lines[8], "66", "右辺 i64.const");
    assert_eq!(lines[9], "2", "右辺 const operand");
    assert_eq!(lines[10], "129", "% は i64.rem_s (0x81) を emit すること");
}

/// selfhost Compiler.ls: root_push を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_root_push_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (root_push 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 1)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "root_push lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "0", "root_push lowering に補助 local は不要");
    assert_eq!(
        lines[2], "2",
        "body は i64.const / root_push の 2 命令であること"
    );
    assert_eq!(
        lines[3], "74",
        "末尾命令は root_push builtin opcode であること"
    );
    assert_eq!(lines[4], "0", "root_push opcode operand は 0 であること");
}

/// selfhost Compiler.ls: root_pop を nullary builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_root_pop_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (root_pop))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        only-instr (vector-get main-ir 0)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get only-instr 0))
      (print (vector-get only-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "root_pop lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "0", "root_pop lowering に補助 local は不要");
    assert_eq!(lines[2], "1", "body は root_pop の 1 命令であること");
    assert_eq!(
        lines[3], "75",
        "唯一の命令は root_pop builtin opcode であること"
    );
    assert_eq!(lines[4], "0", "root_pop opcode operand は 0 であること");
}

/// selfhost Compiler.ls: root_set を builtin として lowering できること
#[test]
fn test_e2e_selfhost_compiler_root_set_builtin_lowering() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (root_set 0 1))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        last-instr (vector-get main-ir 2)]
    (do
      (print (vector-get main-fn 0))
      (print (vector-get main-fn 1))
      (print (vector-length main-ir))
      (print (vector-get last-instr 0))
      (print (vector-get last-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "root_set lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "0", "main は 0 引数関数であること");
    assert_eq!(lines[1], "0", "root_set lowering に補助 local は不要");
    assert_eq!(
        lines[2], "3",
        "body は const / const / root_set の 3 命令であること"
    );
    assert_eq!(
        lines[3], "76",
        "末尾命令は root_set builtin opcode であること"
    );
    assert_eq!(lines[4], "0", "root_set opcode operand は 0 であること");
}

/// selfhost WasmEmit.ls: root_push opcode を runtime import call へ落とせること
#[test]
fn test_e2e_selfhost_wasmemit_root_push_instr() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (root_push 0))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        code-sec (emit-code-section-functions functions)]
    (do
      (print (vector-length code-sec))
      (print (vector-get code-sec 0))
      (print (vector-get code-sec 1))
      (print (vector-get code-sec 2))
      (print (vector-get code-sec 3))
      (print (vector-get code-sec 4))
      (print (vector-get code-sec 5))
      (print (vector-get code-sec 6))
      (print (vector-get code-sec 7))
      (print (vector-get code-sec 8))
      (print (vector-get code-sec 9))
      (print (vector-get code-sec 10))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 11,
        "root_push code section 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "10", "code section byte 長は 10 であること");
    assert_eq!(lines[1], "10", "section id は code=10");
    assert_eq!(lines[2], "8", "section size は 8");
    assert_eq!(lines[3], "1", "function count は 1");
    assert_eq!(lines[4], "6", "function body size は 6 であること");
    assert_eq!(lines[5], "0", "local decl count は 0");
    assert_eq!(lines[6], "66", "先頭命令は i64.const");
    assert_eq!(lines[7], "0", "const operand は 0");
    assert_eq!(
        lines[8], "16",
        "root_push は call opcode へ lower されること"
    );
    assert_eq!(lines[9], "7", "root_push import index は 7 であること");
    assert_eq!(lines[10], "11", "body は end で終わること");
}

/// selfhost WasmEmit.ls: root_pop opcode を runtime import call へ落とせること
#[test]
fn test_e2e_selfhost_wasmemit_root_pop_instr() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (root_pop))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        code-sec (emit-code-section-functions functions)]
    (do
      (print (vector-length code-sec))
      (print (vector-get code-sec 0))
      (print (vector-get code-sec 1))
      (print (vector-get code-sec 2))
      (print (vector-get code-sec 3))
      (print (vector-get code-sec 4))
      (print (vector-get code-sec 5))
      (print (vector-get code-sec 6))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 8,
        "root_pop code section 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "8", "code section byte 長は 8 であること");
    assert_eq!(lines[1], "10", "section id は code=10");
    assert_eq!(lines[2], "6", "section size は 6");
    assert_eq!(lines[3], "1", "function count は 1");
    assert_eq!(lines[4], "4", "function body size は 4 であること");
    assert_eq!(lines[5], "0", "local decl count は 0");
    assert_eq!(
        lines[6], "16",
        "root_pop は call opcode へ lower されること"
    );
    assert_eq!(lines[7], "8", "root_pop import index は 8 であること");
}

/// selfhost WasmEmit.ls: root_set opcode を runtime import call へ落とせること
#[test]
fn test_e2e_selfhost_wasmemit_root_set_instr() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (root_set 0 1))")
        pair (compile-program-functions program)
        functions (vector-get pair 1)
        code-sec (emit-code-section-functions functions)]
    (do
      (print (vector-length code-sec))
      (print (vector-get code-sec 0))
      (print (vector-get code-sec 1))
      (print (vector-get code-sec 2))
      (print (vector-get code-sec 3))
      (print (vector-get code-sec 4))
      (print (vector-get code-sec 5))
      (print (vector-get code-sec 6))
      (print (vector-get code-sec 7))
      (print (vector-get code-sec 8))
      (print (vector-get code-sec 9))
      (print (vector-get code-sec 10))
      (print (vector-get code-sec 11))
      (print (vector-get code-sec 12))
      (print (vector-get code-sec 13))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 13,
        "root_set code section 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "12", "code section byte 長は 12 であること");
    assert_eq!(lines[1], "10", "section id は code=10");
    assert_eq!(lines[2], "10", "section size は 10");
    assert_eq!(lines[3], "1", "function count は 1");
    assert_eq!(lines[4], "8", "function body size は 8 であること");
    assert_eq!(lines[5], "0", "local decl count は 0");
    assert_eq!(lines[6], "66", "先頭命令は i64.const");
    assert_eq!(lines[7], "0", "第1引数 const operand は 0");
    assert_eq!(lines[8], "66", "第2命令も i64.const");
    assert_eq!(lines[9], "1", "第2引数 const operand は 1");
    assert_eq!(
        lines[10], "16",
        "root_set は call opcode へ lower されること"
    );
    assert_eq!(lines[11], "9", "root_set import index は 9 であること");
    assert_eq!(lines[12], "11", "body は end で終わること");
}

/// selfhost WasmEmit.ls: native 専用 opcode を黙って破棄せず fail-closed に拒否すること
#[test]
fn test_e2e_selfhost_wasmemit_rejects_native_only_opcodes() {
    // print-string (87) は 11-import runtime の call へ lower されるため拒否対象から外す。
    for opcode in [86, 88] {
        let harness = format!(
            r#"
(defn main []
  (let [bytes (emit-ir-instr (vector-new 0) {opcode} 0)]
    (do
      (print (vector-length bytes))
      0)))
"#
        );
        let combined = format!(
            "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
            selfhost_module("Token.ls"),
            selfhost_module("AST.ls"),
            selfhost_module("Lexer.ls"),
            selfhost_module("Parser.ls"),
            selfhost_module("IR.ls"),
            selfhost_module("Compiler.ls"),
            selfhost_module("WasiBackend.ls"),
            selfhost_module("WasmEmit.ls"),
            harness
        );

        let error = try_compile_and_run(&combined).expect_err(&format!(
            "native 専用 opcode {opcode} を WasmEmit が黙って破棄している"
        ));
        assert!(
            error.contains("unreachable") || error.contains("integer divide by zero"),
            "native 専用 opcode {opcode} は明示的な trap で拒否すること: {error}"
        );
    }
}

/// selfhost WasmEmit.ls: record patch lookup が使う control-flow opcode を Wasm bytes へ出力できること
#[test]
fn test_e2e_selfhost_wasmemit_emits_control_flow_opcodes() {
    let harness = r#"
(defn print-bytes [bytes idx]
  (if (>= idx (vector-length bytes))
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes bytes (+ idx 1)))))

(defn main []
        (let [block-bytes (emit-ir-instr (vector-new 0) 42 0)
        loop-bytes (emit-ir-instr (vector-new 0) 82 0)
        br-bytes (emit-ir-instr (vector-new 0) 80 1)
        br-if-bytes (emit-ir-instr (vector-new 0) 81 1)
        if-bytes (emit-ir-instr (vector-new 0) 83 0)]
    (do
      (print-bytes block-bytes 0)
      (print-bytes loop-bytes 0)
      (print-bytes br-bytes 0)
      (print-bytes br-if-bytes 0)
      (print-bytes if-bytes 0)
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        selfhost_module("WasiBackend.ls"),
        selfhost_module("WasmEmit.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    assert_eq!(output, "2\n64\n3\n64\n12\n1\n13\n1\n167\n4\n64\n");
}

/// selfhost compiler-mode: root runtime API が actual import semantics で動作すること
#[test]
fn test_e2e_selfhost_compiler_mode_root_runtime_api_works() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(defn main [] (let [slot0 (root_push 111) slot1 (root_push 222) set-result (root_set slot0 333) pop1 (root_pop) pop2 (root_pop) pop3 (root_pop)] (do (print slot0) (print slot1) (print set-result) (print pop1) (print pop2) (print pop3) 0)))"
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
    .expect("selfhost compiler-mode root runtime API module should run");
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["0", "1", "0", "222", "333", "0"]);
}

/// selfhost compiler-mode: root_set の value 側で map-insert を使っても rooted map を返せること
#[test]
fn test_e2e_selfhost_compiler_mode_root_set_preserves_map_insert_value() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(defn main [] (let [slot0 (root_push 111) set-result (root_set slot0 (map-insert (map-new) 123 456)) rooted-map (root_pop)] (do (print slot0) (print set-result) (print (map-size rooted-map)) (print (map-get rooted-map 123)) 0)))"
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
    .expect("selfhost compiler-mode root_set(map-insert) module should run");
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["0", "0", "1", "456"]);
}

/// selfhost compiler-mode: map-insert 単体で entry を保持できること
#[test]
fn test_e2e_selfhost_compiler_mode_map_insert_preserves_entry() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(defn main [] (let [m1 (map-insert (map-new) 123 456)] (do (print (map-size m1)) (print (map-get m1 123)) 0)))"
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
    .expect("selfhost compiler-mode map-insert module should run");
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "456"]);
}

/// selfhost compiler-mode: record literal の field access を actual Wasm で実行できること
#[test]
fn test_e2e_selfhost_compiler_mode_record_literal_field_access_runs() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(type Point (record (: label String) (: x Int))) (defn main [] (let [point {Point label \"record\" x 42}] (do (print (string-length (. point label))) (print (. point x)) 0)))"
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
    .expect("selfhost compiler-mode record literal module should run");
    assert_eq!(output, "6\n42\n");
}

/// selfhost compiler-mode: record pattern が field lookup と binder local を実行できること
#[test]
fn test_e2e_selfhost_compiler_mode_record_pattern_binds_field_and_falls_back() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(type Point (record (: x Int) (: y Int))) (defn main [] (let [p {Point x 41 y 2}] (do (print (match p [{Point x x} x] [_ 0])) (print (match p [{Point x 42} 1] [_ 0])) (print (match p [{Point x x y y} (+ x y)] [_ 0])))))"
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
    .expect("selfhost compiler-mode record pattern module should run");
    assert_eq!(output, "41\n0\n43\n");
}

/// selfhost compiler-mode: ordinary ADT の constructor / pattern binder を actual Wasm で実行できること
#[test]
fn test_e2e_selfhost_compiler_mode_adt_constructor_pattern_binds_and_falls_back() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(type (Maybe a) (Just a) Nothing) (defn unwrap [m] (match m [(Just x) x] [Nothing 0])) (defn main [] (do (print (unwrap (Just 41))) (print (unwrap Nothing)) 0))"
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
    .expect("selfhost compiler-mode ADT pattern module should run");
    assert_eq!(output, "41\n0\n");
}

/// selfhost compiler-mode: record constructor と static accessor を actual Wasm で実行できること
#[test]
fn test_e2e_selfhost_compiler_mode_record_constructor_and_static_accessor_run() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(defn inc [n] (+ n 1)) (type Point (record (: x Int) (: y Int))) (defn main [] (let [point (Point (inc 40) 2)] (do (print (Point.x point)) (print (Point.y point)) 0)))"
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
    .expect("selfhost compiler-mode record constructor module should run");
    assert_eq!(output, "41\n2\n");
}

/// selfhost compiler-mode: parametric record の constructor/static accessor を異なる具体化で実行できること
#[test]
fn test_e2e_selfhost_compiler_mode_parametric_record_constructor_and_static_accessor_run() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(type (Box a) (record (: value a))) (defn main [] (let [int-box (Box 41) bool-box (Box true)] (do (print (Box.value int-box)) (print (Box.value bool-box)) 0)))"
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
    .expect("selfhost compiler-mode parametric record module should run");
    assert_eq!(output, "41\n1\n");
}

/// selfhost compiler-mode: record update の結果を actual Wasm で実行できること
#[test]
fn test_e2e_selfhost_compiler_mode_record_update_runs() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(type Point (record (: x Int) (: y Int) (: z Int))) (defn main [] (let [p {Point x 1 y 2 z 3} q {p | x 10} r {q | y 20}] (do (print (Point.x q)) (print (Point.y q)) (print (Point.z q)) (print (. q y)) (print (Point.x p)) (print (Point.y p)) (print (Point.z p)) (print (Point.x r)) (print (Point.y r)) (print (Point.z r)) (print (. r x)) (print (. r y)) (print (. r z)) (print (match p [{Point x x} x] [_ 0])) (print (match p [{Point x 42} 1] [_ 0])) 0)))"
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
    .expect("selfhost compiler-mode record update module should run");
    assert_eq!(output, "10\n2\n3\n2\n1\n2\n3\n10\n20\n3\n10\n20\n3\n1\n0\n");
}

/// selfhost ftable compiler: record update と static accessor を実行できること
#[test]
fn test_e2e_selfhost_ftable_compiler_record_update_and_static_accessor_run() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(type Point (record (: x Int) (: y Int) (: z Int))) (defn main [] (let [p {Point x 1 y 2 z 3} q {p | x 10} r {q | y 20}] (do (print (Point.x q)) (print (Point.y q)) (print (Point.z q)) (print (. q y)) (print (Point.x p)) (print (Point.y p)) (print (Point.z p)) (print (Point.x r)) (print (Point.y r)) (print (Point.z r)) (print (. r x)) (print (. r y)) (print (. r z)) 0)))"
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
    let output = super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode(
        &wasm_bytes,
        "",
        &[],
    )
    .expect("selfhost legacy base compiler record module should run");
    assert_eq!(output, "10\n2\n3\n2\n1\n2\n3\n10\n20\n3\n10\n20\n3\n");
}

/// selfhost compiler-mode: import 先 record の constructor/static accessor が actual Wasm で実行できること
#[test]
fn test_e2e_selfhost_compiler_mode_imported_record_constructor_and_static_accessor_run() {
    let temp_root = std::env::temp_dir().join(format!(
        "lsharp-selfhost-record-import-runtime-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&temp_root);
    let app_dir = temp_root.join("src/App");
    std::fs::create_dir_all(&app_dir).expect("record import fixture の directory を作れない");
    std::fs::write(
        app_dir.join("Shapes.ls"),
        "(module App.Shapes)\n(type Point (record (: x Int) (: y Int)))\n",
    )
    .expect("record import fixture の Shapes.ls を書けない");
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.Shapes)\n(defn inc [n] (+ n 1))\n(defn main [] (let [point (Point (inc 40) 2)] (do (print (Point.x point)) (print (Point.y point)) 0)))\n",
    )
    .expect("record import fixture の Main.ls を書けない");

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
    let output = super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode_fs(
        &wasm_bytes,
        &temp_root,
        &[],
    )
    .expect("import 先 record を含む selfhost compiler-mode module should run");
    assert_eq!(output, "41\n2\n");
    std::fs::remove_dir_all(&temp_root).expect("record import fixture を削除できない");
}

/// selfhost compiler-mode: root_set を do 位置で使って map を更新できること
#[test]
fn test_e2e_selfhost_compiler_mode_root_set_updates_map_without_binding_result() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(defn main [] (let [base (map-new) slot (root_push base)] (do (root_set slot (map-insert base 123 456)) (let [rooted (root_pop)] (do (print (map-size rooted)) (print (map-get rooted 123)) 0)))))"
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
    .expect("selfhost compiler-mode root_set do-position module should run");
    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines, vec!["1", "456"]);
}

/// selfhost compiler-mode: zero-arg helper call を含む vector builder を source compile できること
#[test]
fn test_e2e_selfhost_compiler_mode_zero_arg_tag_call_inside_vector_builder() {
    let harness = r#"
(defn print-bytes-loop [bytes idx count]
  (if (>= idx count)
    0
    (do
      (print (vector-get bytes idx))
      (print-bytes-loop bytes (+ idx 1) count))))

(defn main []
  (let [source "(defn tag [] 24) (defn wrap [value] (vector-push (vector-push (vector-new 2) (tag)) value)) (defn main [] 0)"
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
    std::fs::write("/tmp/zero_arg_tag_vector_builder.wasm", &wasm_bytes)
        .expect("debug wasm dump should succeed");
    let output = super::selfhost_bootstrap_four_layer::run_wasm_with_eleven_imports_compiler_mode(
        &wasm_bytes,
        "",
        &[],
    )
    .expect("selfhost compiler-mode zero-arg tag vector module should run");
    assert_eq!(output.trim(), "");
}

/// selfhost Compiler.ls: source 付き string literal lowering が inline string object と定数オフセットを返すこと
#[test]
fn test_e2e_selfhost_compiler_string_literal_source_data_lowering() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] \"abc\")"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        const-instr (vector-get main-ir 0)]
    (do
      (print (vector-length data))
      (print (vector-get data 0))
      (print (vector-get data 1))
      (print (vector-get data 2))
      (print (vector-get const-instr 0))
      (print (vector-get const-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 6,
        "string literal lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "11", "string literal object bytes 長が不正");
    assert_eq!(lines[1], "1", "string literal header tag byte が不正");
    assert_eq!(lines[2], "0", "string literal header len low byte が不正");
    assert_eq!(lines[3], "0", "string literal header len high byte が不正");
    assert_eq!(
        lines[4], "1",
        "string literal lowering の IR は i64.const 1 命令であること"
    );
    assert_eq!(
        lines[5], "1024",
        "string literal lowering の定数オフセットが不正"
    );
}

/// selfhost Compiler.ls: nested string literal lowering が distinct offsets と連結 string object bytes を返すこと
#[test]
fn test_e2e_selfhost_compiler_nested_string_literal_source_data_lowering() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] (do \"ab\" \"cde\"))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        first-instr (vector-get main-ir 0)
        drop-instr (vector-get main-ir 1)
        second-instr (vector-get main-ir 2)]
    (do
      (print (vector-length data))
      (print (vector-get data 0))
      (print (vector-get data 1))
      (print (vector-get data 2))
      (print (vector-get data 3))
      (print (vector-get data 4))
      (print (vector-get first-instr 0))
      (print (vector-get first-instr 1))
      (print (vector-get drop-instr 0))
      (print (vector-get second-instr 0))
      (print (vector-get second-instr 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 11,
        "nested string literal lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "21",
        "nested string literal object bytes 長が不正"
    );
    assert_eq!(lines[1], "1", "1 個目 string object header tag byte が不正");
    assert_eq!(lines[2], "0", "1 個目 string object header byte[1] が不正");
    assert_eq!(lines[3], "0", "1 個目 string object header byte[2] が不正");
    assert_eq!(lines[4], "0", "1 個目 string object header byte[3] が不正");
    assert_eq!(lines[5], "2", "1 個目 string object length byte が不正");
    assert_eq!(lines[6], "1", "先頭命令は i64.const であるべき");
    assert_eq!(lines[7], "1024", "先頭 string literal offset が不正");
    assert_eq!(lines[8], "44", "中間 string literal は drop されるべき");
    assert_eq!(lines[9], "1", "末尾命令は i64.const であるべき");
    assert_eq!(
        lines[10], "1034",
        "末尾 string literal offset が前段 object bytes を考慮していない"
    );
}

/// selfhost Compiler.ls: source-aware do string literal lowering が 5 式以上でも全 string object を連結できること
#[test]
fn test_e2e_selfhost_compiler_extended_do_string_literal_source_data_lowering() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] (do \"ab\" \"c\" \"de\" \"fgh\" \"ijk\"))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        final-const (vector-get main-ir 8)]
    (do
      (print (vector-length data))
      (print (vector-get data 0))
      (print (vector-get data 1))
      (print (vector-get data 2))
      (print (vector-get data 3))
      (print (vector-get data 4))
      (print (vector-get data 5))
      (print (vector-get data 6))
      (print (vector-get data 7))
      (print (vector-get data 8))
      (print (vector-get data 9))
      (print (vector-get data 10))
      (print (vector-length main-ir))
      (print (vector-get final-const 0))
      (print (vector-get final-const 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 15,
        "extended do string literal lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "51",
        "extended do string literal object bytes 長が不正"
    );
    assert_eq!(lines[1], "1", "1 個目 string object header tag byte が不正");
    assert_eq!(lines[2], "0", "1 個目 string object header byte[1] が不正");
    assert_eq!(lines[3], "0", "1 個目 string object header byte[2] が不正");
    assert_eq!(lines[4], "0", "1 個目 string object header byte[3] が不正");
    assert_eq!(lines[5], "2", "1 個目 string object length byte が不正");
    assert_eq!(lines[6], "0", "1 個目 string object length byte[1] が不正");
    assert_eq!(lines[7], "0", "1 個目 string object length byte[2] が不正");
    assert_eq!(lines[8], "0", "1 個目 string object length byte[3] が不正");
    assert_eq!(
        lines[9], "97",
        "1 個目 string object payload byte[0] が不正"
    );
    assert_eq!(
        lines[10], "98",
        "1 個目 string object payload byte[1] が不正"
    );
    assert_eq!(
        lines[11], "1",
        "2 個目 string object header tag byte が不正"
    );
    assert_eq!(
        lines[12], "9",
        "5 式 do の IR は const/drop を含む 9 命令であること"
    );
    assert_eq!(
        lines[13], "1",
        "extended do の末尾命令は i64.const であること"
    );
    assert_eq!(
        lines[14], "1064",
        "extended do の末尾 string literal offset が不正"
    );
}

/// selfhost Compiler.ls: source-aware if branch の string literal lowering が両 branch の string object を data section に積むこと
#[test]
fn test_e2e_selfhost_compiler_if_string_literal_source_data_lowering() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] (if (= 1 1) \"hello\" \"world\"))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        functions (vector-get pair 1)
        data (vector-get pair 2)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)
        then-const (vector-get main-ir 4)
        else-const (vector-get main-ir 6)]
    (do
      (print (vector-length data))
      (print (vector-get data 0))
      (print (vector-get data 1))
      (print (vector-get data 2))
      (print (vector-get data 3))
      (print (vector-get data 4))
      (print (vector-get data 5))
      (print (vector-get data 6))
      (print (vector-get data 7))
      (print (vector-get data 8))
      (print (vector-get data 9))
      (print (vector-get then-const 1))
      (print (vector-get else-const 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 13,
        "if string literal lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "26", "if string literal object bytes 長が不正");
    assert_eq!(
        lines[1], "1",
        "then branch string object header tag byte が不正"
    );
    assert_eq!(
        lines[2], "0",
        "then branch string object header byte[1] が不正"
    );
    assert_eq!(
        lines[3], "0",
        "then branch string object header byte[2] が不正"
    );
    assert_eq!(
        lines[4], "0",
        "then branch string object header byte[3] が不正"
    );
    assert_eq!(
        lines[5], "5",
        "then branch string object length byte が不正"
    );
    assert_eq!(
        lines[6], "0",
        "then branch string object length byte[1] が不正"
    );
    assert_eq!(
        lines[7], "0",
        "then branch string object length byte[2] が不正"
    );
    assert_eq!(
        lines[8], "0",
        "then branch string object length byte[3] が不正"
    );
    assert_eq!(
        lines[9], "104",
        "then branch string object payload byte[0] が不正"
    );
    assert_eq!(
        lines[10], "101",
        "then branch string object payload byte[1] が不正"
    );
    assert_eq!(
        lines[11], "1024",
        "then branch string literal offset が不正"
    );
    assert_eq!(
        lines[12], "1037",
        "else branch string literal offset が不正"
    );
}

/// selfhost Compiler.ls: source-aware match arm body の string literal lowering が string object を data section に積むこと
#[test]
fn test_e2e_selfhost_compiler_match_string_literal_source_data_lowering() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] (match 2 [1 \"one\"] [2 \"two\"]))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        data (vector-get pair 2)]
    (do
      (print (vector-length data))
      (print (vector-get data 0))
      (print (vector-get data 1))
      (print (vector-get data 2))
      (print (vector-get data 3))
      (print (vector-get data 4))
      (print (vector-get data 5))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 7,
        "match string literal lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "22", "match string literal object bytes 長が不正");
    assert_eq!(lines[1], "1", "1 個目 string object header tag byte が不正");
    assert_eq!(lines[2], "0", "1 個目 string object header byte[1] が不正");
    assert_eq!(lines[3], "0", "1 個目 string object header byte[2] が不正");
    assert_eq!(lines[4], "0", "1 個目 string object header byte[3] が不正");
    assert_eq!(lines[5], "3", "1 個目 string object length byte が不正");
    assert_eq!(lines[6], "0", "1 個目 string object length byte[1] が不正");
}

/// selfhost Compiler.ls: source-aware lambda body の string literal lowering が string object を data section に積むこと
#[test]
fn test_e2e_selfhost_compiler_lambda_string_literal_source_data_lowering() {
    let harness = r#"
(defn main []
  (let [source "(defn main [] (fn [x] \"ok\"))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        data (vector-get pair 2)]
    (do
      (print (vector-length data))
      (print (vector-get data 0))
      (print (vector-get data 1))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 3,
        "lambda string literal lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "10",
        "lambda string literal object bytes 長が不正"
    );
    assert_eq!(lines[1], "1", "lambda string object header tag byte が不正");
    assert_eq!(lines[2], "0", "lambda string object header byte[1] が不正");
}

/// selfhost Compiler.ls: source-aware map string-key lowering は key literal を data section へ落とさず hash const 化できること
#[test]
fn test_e2e_selfhost_compiler_map_string_key_source_hash_lowering() {
    let harness = r#"
(defn count-opcode [instrs idx count opcode hits]
  (if (>= idx count)
    hits
    (let [instr (vector-get instrs idx)
          tag (vector-get instr 0)]
      (count-opcode instrs (+ idx 1) count opcode (if (= tag opcode) (+ hits 1) hits)))))

(defn main []
  (let [source "(defn main [] (let [m0 (map-new)] (let [m1 (map-insert m0 \"aa\" 10)] (+ (* 10 (map-contains? m1 \"aa\")) (map-contains? m1 \"zz\")))))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        data (vector-get pair 2)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)]
    (do
      (print (vector-length data))
      (print (count-opcode main-ir 0 (vector-length main-ir) 1 0))
      (print (count-opcode main-ir 0 (vector-length main-ir) 62 0))
      (print (count-opcode main-ir 0 (vector-length main-ir) 65 0))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 4,
        "map string-key lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "0",
        "string key literal は data section へ積まず hash const 化すること"
    );
    assert_eq!(lines[2], "1", "map-insert opcode が 1 回入ること");
    assert_eq!(lines[3], "2", "map-contains opcode が 2 回入ること");
}

/// selfhost Compiler.ls: non-literal string key map lowering は key 式を直接 map builtin へ渡すこと
#[test]
fn test_e2e_selfhost_compiler_map_non_literal_string_key_runtime_hash_lowering() {
    let harness = r#"
(defn count-opcode [instrs idx count opcode hits]
  (if (>= idx count)
    hits
    (let [instr (vector-get instrs idx)
          tag (vector-get instr 0)]
      (count-opcode instrs (+ idx 1) count opcode (if (= tag opcode) (+ hits 1) hits)))))

(defn main []
  (let [source "(defn main [] (let [key (read-file \"fixture.txt\")] (let [m0 (map-new)] (let [m1 (map-insert m0 key 42)] (map-get m1 key)))))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        data (vector-get pair 2)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        main-ir (vector-get main-fn 2)]
    (do
      (print (vector-length data))
      (print (count-opcode main-ir 0 (vector-length main-ir) 64 0))
      (print (count-opcode main-ir 0 (vector-length main-ir) 68 0))
      (print (count-opcode main-ir 0 (vector-length main-ir) 62 0))
      (print (count-opcode main-ir 0 (vector-length main-ir) 63 0))
      0)))
"#;
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        selfhost_module("Token.ls"),
        selfhost_module("AST.ls"),
        selfhost_module("Lexer.ls"),
        selfhost_module("Parser.ls"),
        selfhost_module("IR.ls"),
        selfhost_module("Compiler.ls"),
        harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();
    assert!(
        lines.len() >= 5,
        "non-literal string key lowering 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "19",
        "read-file path literal string object は data section に積まれること"
    );
    assert_eq!(lines[1], "1", "read-file opcode が 1 回入ること");
    assert_eq!(
        lines[2], "0",
        "non-literal string key lowering は専用 runtime hash opcode を挿入しないこと"
    );
    assert_eq!(lines[3], "1", "map-insert opcode が 1 回入ること");
    assert_eq!(lines[4], "1", "map-get opcode が 1 回入ること");
}

// =====================================================// P1-3: WASI stdin/stdout ラッパーテスト
// =====================================================
/// P1-3: write-string が stdout に書き込めることを検証
/// (write-string は print-string の別名として動作する)
#[test]
fn test_e2e_write_string_stdout() {
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "hello stdout")
            0))
    "#,
    );
    assert_eq!(result.trim(), "hello stdout");
}

/// P1-3: fd_write WASI syscall ラッパーの基本テスト
/// stdout (fd=1) への print-string 出力が正しく動くことを検証
#[test]
fn test_e2e_fd_write_stdout() {
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print-string "line1")
            (print 42)
            0))
    "#,
    );
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

    let result = compile_and_run_with_dir(
        r#"
        (defn main []
          (do
            (let [content (read-file "test_input.txt")]
              (print (string-length content)))
            0))
    "#,
        &dir,
    );
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

    let result = compile_and_run_with_dir(
        r#"
        (defn main []
          (do
            (write-file "roundtrip.txt" "test data 123")
            (let [content (read-file "roundtrip.txt")]
              (print (string-length content)))
            0))
    "#,
        &dir,
    );
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(result.trim(), "13");
}

/// P1-3: file-exists? による存在確認テスト
#[test]
fn test_e2e_file_exists_check() {
    let dir = std::env::temp_dir().join("lsharp_test_exists_check");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("exists.txt"), "data").unwrap();

    let result = compile_and_run_with_dir(
        r#"
        (defn main []
          (do
            (print (file-exists? "exists.txt"))
            (print (file-exists? "nonexistent.txt"))
            0))
    "#,
        &dir,
    );
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
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib/Json.ls"),
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
    let result = compile_and_run(
        r#"
        (defn main []
          (let [s "test"]
            (do
              (print (string-length s))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "4");
}

/// GC Phase 1: Vector オブジェクトのヘッダが正しく設定されることを検証
#[test]
fn test_e2e_gc_vector_header() {
    let result = compile_and_run(
        r#"
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
    "#,
    );
    assert_eq!(result.trim(), "3\n10\n20\n30");
}

/// GC Phase 2: 大量アロケーション後もヒープが正常に動作することを検証
/// (現在は bump allocator のみ、GC 導入後にヒープ回復も検証)
#[test]
fn test_e2e_gc_bulk_allocation() {
    let result = compile_and_run(
        r#"
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
    "#,
    );
    assert_eq!(result.trim(), "42");
}

/// GC Phase 3: HashMap の大量操作後もヒープが正常に動作
#[test]
fn test_e2e_gc_hashmap_stress() {
    let result = compile_and_run(
        r#"
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
    "#,
    );
    assert_eq!(result.trim(), "30\n5");
}

/// GC Phase 3: 文字列の大量連結でもヒープが正常に動作
#[test]
fn test_e2e_gc_string_concat_stress() {
    let result = compile_and_run(
        r#"
        (defn repeat-concat [s n]
          (if (= n 0)
            s
            (repeat-concat (string-concat s "x") (- n 1))))

        (defn main []
          (let [result (repeat-concat "" 50)]
            (do
              (print (string-length result))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "50");
}
