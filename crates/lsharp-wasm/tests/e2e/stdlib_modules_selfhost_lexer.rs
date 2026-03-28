use super::support::*;

#[test]
fn test_e2e_selfhost_codegen_comparison() {
    // セルフホスト Codegen vs Rust Codegen の比較テスト
    // L# の IR.ls/Compiler.ls/WasmEmit.ls パターンで命令・LEB128 を構築し、
    // Rust の Instruction/leb128 エンコードと同等の結果を検証
    //
    // 対応関係:
    //   L# make-instr(1, 42)  ↔  Rust Instruction::I64Const(42)
    //   L# make-instr(10, 0)  ↔  Rust Instruction::LocalGet(0)
    //   L# make-instr(40, 5)  ↔  Rust Instruction::Call(5)
    //   L# leb128-unsigned    ↔  Rust wasm-encoder の LEB128
    let result = compile_and_run(
        r#"
        ;; IR 命令構築 (IR.ls パターン)
        (defn make-instr [opcode operand]
          (vector-push (vector-push (vector-new 2) opcode) operand))

        ;; LEB128 符号なしエンコード (Compiler.ls/WasmEmit.ls パターン)
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

        ;; Wasm ヘッダー生成 (WasmEmit.ls パターン)
        (defn emit-header []
          (let [h (vector-new 8)]
            (vector-push
              (vector-push
                (vector-push
                  (vector-push
                    (vector-push
                      (vector-push
                        (vector-push
                          (vector-push h 0)
                          97)
                        115)
                      109)
                    1)
                  0)
                0)
              0)))

        (defn main []
          (let [;; IR 命令構築比較 (Rust: Instruction 列挙型との対応)
                const-instr (make-instr 1 42)
                get-instr (make-instr 10 0)
                call-instr (make-instr 40 5)

                ;; LEB128 比較 (Rust: wasm-encoder の LEB128 と同等)
                leb5 (leb128-unsigned 5)
                leb300 (leb128-unsigned 300)
                leb16384 (leb128-unsigned 16384)

                ;; Wasm ヘッダー比較
                header (emit-header)]
            (do
              ;; IR 命令: i64.const 42 (Rust: Instruction::I64Const(42))
              (print (vector-get const-instr 0))
              (print (vector-get const-instr 1))

              ;; IR 命令: local.get 0 (Rust: Instruction::LocalGet(0))
              (print (vector-get get-instr 0))
              (print (vector-get get-instr 1))

              ;; IR 命令: call 5 (Rust: Instruction::Call(5))
              (print (vector-get call-instr 0))
              (print (vector-get call-instr 1))

              ;; LEB128(5) = [5] (1バイト)
              (print (vector-length leb5))
              (print (vector-get leb5 0))

              ;; LEB128(300) = [172, 2] (2バイト: 300 = 0b100101100)
              (print (vector-length leb300))
              (print (vector-get leb300 0))
              (print (vector-get leb300 1))

              ;; LEB128(16384) = [128, 128, 1] (3バイト: 16384 = 0x4000)
              (print (vector-length leb16384))
              (print (vector-get leb16384 0))
              (print (vector-get leb16384 1))
              (print (vector-get leb16384 2))

              ;; Wasm ヘッダー先頭4バイト: \0asm (Rust: wasm マジックナンバー)
              (print (vector-get header 0))
              (print (vector-get header 1))
              (print (vector-get header 2))
              (print (vector-get header 3))

              0)))
    "#,
    );
    // IR: const(1,42), get(10,0), call(40,5)
    // LEB128(5)=[5](1byte), LEB128(300)=[172,2](2bytes), LEB128(16384)=[128,128,1](3bytes)
    // Header: 0,97,115,109
    assert_eq!(
        result.trim(),
        "1\n42\n10\n0\n40\n5\n1\n5\n2\n172\n2\n3\n128\n128\n1\n0\n97\n115\n109"
    );
}

#[test]
fn test_e2e_bootstrap_stage1_modules() {
    let mut passed = 0;
    let mut skipped = 0;
    let mut failed = Vec::new();

    // 各モジュールの定義: (ファイル名, 期待出力) — ソースは selfhost/ から読み、(import) はマルチファイル経路
    let modules: Vec<(&str, &str)> = vec![
        // Token.ls: トークン種別定数の出力 (lparen=0, rparen=1, eof=99)
        ("Token.ls", "0\n1\n99"),
        // Lexer.ls: "(defn main [] 42)" をトークナイズ (8トークン + 各トークン種別)
        (
            "Lexer.ls",
            "8\n0\n30\n20\n2\n3\n10\n1\n99\n6\n0\n20\n10\n20\n1\n99\n42\n1\n2",
        ),
        // AST.ls: ノード生成 + 走査基盤 (tag/leaf/count/contains-var)
        ("AST.ls", "1\n42\n10\n1\n0\n1\n4\n1\n0\n1\n3\n4"),
        // Parser.ls: トークン列からパース (tag=20 defn, pos=2)
        ("Parser.ls", "20\n2\n10\n10\n2\n1\n2"),
        // IR.ls: IR命令生成 (i64.const=1/42, local.get=10/0)
        ("IR.ls", "1\n42\n10\n0"),
        // Type.ls: 型操作 (Con tag=1, Var tag=2, name=42, subst lookup→Con tag=1)
        ("Type.ls", "1\n2\n42\n1"),
        // TypeScheme.ls: 型スキーム操作 (mono/poly instantiate, free-vars)
        ("TypeScheme.ls", "1\n100\n3\n2\n1000\n0\n1\n1"),
        // Compiler.ls: コンパイラ操作 (命令数=1, op=1/42, LEB128検証)
        (
            "Compiler.ls",
            "1\n1\n42\n3\n1\n5\n2\n172\n2\n3\n1\n3\n1\n4\n40",
        ),
        // WasmEmit.ls: Wasmバイナリ生成 (header + type section + LEB128)
        (
            "WasmEmit.ls",
            "8\n0\n97\n115\n109\n1\n7\n1\n5\n1\n96\n5\n172\n2\n5\n1\n127",
        ),
    ];

    let selfhost_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../selfhost");

    // コンパイラの既知の制限により一部モジュールが未対応:
    // - Lexer.ls: 深いネストの if 式でパースエラー
    // - Parser.ls: 相互再帰関数 (parse-sexp) の前方参照が未対応
    // - TypeScheme.ls: 相互再帰関数 (instantiate-apply) の前方参照が未対応
    // これらは将来のコンパイラ改善で解消される予定
    // 2パス型推論 + TypeScheme.ls 修正により全モジュールがコンパイル可能
    let known_limitations: &[&str] = &[];

    for (name, expected) in &modules {
        let is_known_limitation = known_limitations.contains(name);
        let path = selfhost_dir.join(name);

        match try_compile_and_run_file(&path) {
            Ok(output) => {
                if output.trim() == *expected {
                    passed += 1;
                } else if is_known_limitation {
                    // 既知の制限: コンパイル成功したが出力不一致 (前方参照解決後の動作検証は別タスク)
                    eprintln!(
                        "  [既知の制限] {}: 出力不一致 (期待: {:?}, 実際: {:?})",
                        name,
                        expected,
                        output.trim()
                    );
                    skipped += 1;
                } else {
                    failed.push(format!(
                        "{}: 出力不一致\n  期待: {:?}\n  実際: {:?}",
                        name,
                        expected,
                        output.trim()
                    ));
                }
            }
            Err(e) => {
                if is_known_limitation {
                    // 既知の制限: エラーを記録するがテスト失敗にはしない
                    eprintln!("  [既知の制限] {}: {}", name, e);
                    skipped += 1;
                } else {
                    failed.push(format!("{}: {}", name, e));
                }
            }
        }
    }

    // 結果サマリーを出力
    eprintln!(
        "\n=== ブートストラップ Stage1 検証結果 ===\n成功: {}/{} (スキップ: {})\n",
        passed,
        modules.len(),
        skipped,
    );
    if !failed.is_empty() {
        eprintln!("失敗モジュール:");
        for msg in &failed {
            eprintln!("  - {}", msg);
        }
    }

    // 既知の制限以外の失敗があればテスト失敗
    assert!(
        failed.is_empty(),
        "ブートストラップ検証: {}/{} モジュールが予期せず失敗\n{}",
        failed.len(),
        modules.len(),
        failed.join("\n")
    );

    // 成功数の最低ラインを検証 (回帰防止)
    assert!(
        passed >= 9,
        "ブートストラップ検証: 成功モジュール数が回帰 ({}/9、全9必要)",
        passed,
    );
}

// === stdlib テスト: IO.ls ===

/// stdlib/IO.ls の file-exists? テスト (WASI stdout キャプチャ)
#[test]
fn test_e2e_stdlib_io_file_exists() {
    // IO.ls の main 関数相当: file-exists? でファイルが存在しないことを確認
    let result = compile_and_run(
        r#"
        (defn main []
          (do
            (print (file-exists? "nonexistent.txt"))
            0))
    "#,
    );
    // file-exists? は false (0) を返す
    assert_eq!(result.trim(), "0");
}

/// stdlib/IO.ls の read-file-or: ファイルが存在しない場合のデフォルト値
#[test]
fn test_e2e_stdlib_io_read_file_or() {
    let tmpdir = std::env::temp_dir().join("lsharp_test_io_read_file_or");
    std::fs::create_dir_all(&tmpdir).unwrap();
    let result = compile_and_run_with_dir(
        r#"
        (defn read-file-or [path default]
          (if (file-exists? path)
            (read-file path)
            default))
        (defn main []
          (let [content (read-file-or "missing.txt" "fallback")]
            (do
              (print (string-length content))
              0)))
    "#,
        &tmpdir,
    );
    // "fallback" は 8 文字
    assert_eq!(result.trim(), "8");
    let _ = std::fs::remove_dir_all(&tmpdir);
}

// === stdlib テスト: Map.ls ===

/// stdlib/Map.ls の map 基本操作テスト (map-new, map-insert, map-get, map-size)
#[test]
fn test_e2e_stdlib_map_basic() {
    let result = compile_and_run(
        r#"
        (defn main []
          (let [m (map-new)
                m1 (map-insert m 1 100)
                m2 (map-insert m1 2 200)]
            (do
              (print (map-size m2))
              (print (map-get m2 1))
              (print (map-get m2 2))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "2\n100\n200");
}

/// stdlib/Map.ls の map-empty?, map-contains?, map-remove テスト
/// 注意: map-insert/map-remove はインプレース変更のため、元変数も変化する
#[test]
fn test_e2e_stdlib_map_operations() {
    let result = compile_and_run(
        r#"
        (defn map-empty? [m] (== (map-size m) 0))
        (defn main []
          (do
            ;; 空マップのテスト
            (print (map-empty? (map-new)))
            ;; 要素追加後のテスト
            (let [m1 (map-insert (map-new) 10 999)]
              (do
                (print (map-empty? m1))
                (print (map-contains? m1 10))
                (print (map-contains? m1 99))
                ;; remove 後のテスト
                (let [m2 (map-remove m1 10)]
                  (print (map-size m2)))
                0))))
    "#,
    );
    assert_eq!(result.trim(), "1\n0\n1\n0\n0");
}

/// stdlib/Map.ls の map-get-or テスト (キーが存在しない場合のデフォルト値)
#[test]
fn test_e2e_stdlib_map_get_or() {
    // map-contains? は Bool を返すが、map-get は Int を返すため
    // 型推論の互換性のために match + == パターンを使用
    let result = compile_and_run(
        r#"
        (defn map-get-or [m key default]
          (let [has (map-contains? m key)]
            (if (== has 1)
              (map-get m key)
              default)))
        (defn main []
          (let [m (map-insert (map-new) 1 42)]
            (do
              (print (map-get-or m 1 0))
              (print (map-get-or m 999 -1))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "42\n-1");
}

// === stdlib テスト: Vector.ls ===

/// stdlib/Vector.ls の基本操作テスト (vector-new, vector-push, vector-get, vector-length)
#[test]
fn test_e2e_stdlib_vector_basic() {
    let result = compile_and_run(
        r#"
        (defn main []
          (let [v (vector-push (vector-push (vector-push (vector-new 4) 1) 2) 3)]
            (do
              (print (vector-length v))
              (print (vector-get v 0))
              (print (vector-get v 1))
              (print (vector-get v 2))
              0)))
    "#,
    );
    assert_eq!(result.trim(), "3\n1\n2\n3");
}

/// stdlib/Vector.ls の vector-empty?, vector-set テスト
/// 注意: vector-push はインプレース変更のため、元変数も変化する
#[test]
fn test_e2e_stdlib_vector_empty_and_set() {
    let result = compile_and_run(
        r#"
        (defn vector-empty? [v] (== (vector-length v) 0))
        (defn main []
          (do
            ;; 空ベクタのテスト
            (print (vector-empty? (vector-new 4)))
            ;; 要素追加後のテスト
            (let [v1 (vector-push (vector-push (vector-new 4) 10) 20)
                  v2 (vector-set v1 0 99)]
              (do
                (print (vector-empty? v1))
                (print (vector-get v2 0))
                (print (vector-get v2 1))
                0))))
    "#,
    );
    assert_eq!(result.trim(), "1\n0\n99\n20");
}

/// stdlib/Vector.ls の vector-fold (左畳み込み) と vector-sum テスト
#[test]
fn test_e2e_stdlib_vector_fold_sum() {
    let result = compile_and_run(
        r#"
        (defn vector-fold-impl [f acc v i len]
          (if (>= i len)
            acc
            (vector-fold-impl f (f acc (vector-get v i)) v (+ i 1) len)))
        (defn vector-fold [f init v]
          (vector-fold-impl f init v 0 (vector-length v)))
        (defn vector-sum [v]
          (vector-fold (fn [acc x] (+ acc x)) 0 v))
        (defn main []
          (let [v (vector-push (vector-push (vector-push (vector-new 4) 10) 20) 30)]
            (do
              (print (vector-sum v))
              (print (vector-fold (fn [acc x] (+ acc 1)) 0 v))
              0)))
    "#,
    );
    // sum = 10 + 20 + 30 = 60, count = 3
    assert_eq!(result.trim(), "60\n3");
}

// === セルフホスティング: Lexer 比較テスト ===

/// L# Lexer.ls と Rust Lexer の出力を比較するテスト
/// 同一の入力文字列に対して、両方の Lexer が同等のトークン種別を返すことを検証
#[test]
fn test_e2e_selfhost_lexer_comparison() {
    // L# Lexer.ls のトークン種別マッピング:
    //   0=LParen, 1=RParen, 2=LBracket, 3=RBracket, 4=LBrace, 5=RBrace,
    //   10=Int, 12=String, 13=true, 14=false, 20=Symbol,
    //   30=Defn, 31=Let, 32=If, 33=Match, 34=Type, 35=Fn, 36=Do,
    //   50=Colon, 52=Pipe, 99=Eof

    // テスト入力: "(defn main [] 42)"
    let input = "(defn main [] 42)";

    // --- Rust Lexer でトークン化 ---
    let mut rust_lexer = lsharp_syntax::lexer::Lexer::new(input);
    let rust_tokens = rust_lexer.tokenize().unwrap();
    // Rust トークンを L# Lexer.ls の種別コードに変換
    let rust_kinds: Vec<i64> = rust_tokens
        .iter()
        .map(|t| {
            use lsharp_syntax::token::TokenKind;
            match &t.kind {
                TokenKind::LParen => 0,
                TokenKind::RParen => 1,
                TokenKind::LBracket => 2,
                TokenKind::RBracket => 3,
                TokenKind::LBrace => 4,
                TokenKind::RBrace => 5,
                TokenKind::Int(_) => 10,
                TokenKind::String(_) => 12,
                TokenKind::Bool(true) => 13,
                TokenKind::Bool(false) => 14,
                TokenKind::Symbol(_) => 20,
                TokenKind::Defn => 30,
                TokenKind::Let => 31,
                TokenKind::If => 32,
                TokenKind::Match => 33,
                TokenKind::Type => 34,
                TokenKind::Fn => 35,
                TokenKind::Do => 36,
                TokenKind::Module => 37,
                TokenKind::Import => 38,
                TokenKind::Colon => 50,
                TokenKind::Pipe => 52,
                TokenKind::Eof => 99,
                // L# Lexer.ls は以下をサポートしていないため、Symbol 扱い
                _ => 20,
            }
        })
        .collect();

    // --- L# Lexer.ls (Wasm) でトークン化 ---
    // Lexer.ls の関数群をインラインで定義して実行
    let lsharp_result = compile_and_run(
        r#"
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
                (if (string-eq name "match") 33
                  (if (string-eq name "type") 34
                    (if (string-eq name "fn") 35
                      (if (string-eq name "do") 36
                        (if (string-eq name "true") 13
                          (if (string-eq name "false") 14 20))))))))))
        (defn scan-digits [src pos len]
          (if (>= pos len) pos
            (if (is-digit-char (string-char-at src pos)) (scan-digits src (+ pos 1) len) pos)))
        (defn scan-symbol-end [src pos len]
          (if (>= pos len) pos
            (if (is-symbol-char (string-char-at src pos)) (scan-symbol-end src (+ pos 1) len) pos)))
        (defn scan-string-end [src pos len]
          (if (>= pos len) pos
            (let [c (string-char-at src pos)]
              (if (== c 34) (+ pos 1)
                (if (== c 92) (scan-string-end src (+ pos 2) len)
                  (scan-string-end src (+ pos 1) len))))))
        (defn lex-one [src pos len]
          (if (>= pos len) (+ (* 99 1000000) pos)
            (let [c (string-char-at src pos)]
              (if (== c 40) (+ (* 0 1000000) (+ pos 1))
                (if (== c 41) (+ (* 1 1000000) (+ pos 1))
                  (if (== c 91) (+ (* 2 1000000) (+ pos 1))
                    (if (== c 93) (+ (* 3 1000000) (+ pos 1))
                      (if (== c 123) (+ (* 4 1000000) (+ pos 1))
                        (if (== c 125) (+ (* 5 1000000) (+ pos 1))
                          (if (== c 58) (+ (* 50 1000000) (+ pos 1))
                            (if (== c 124) (+ (* 52 1000000) (+ pos 1))
                              (if (== c 34)
                                (let [end (scan-string-end src (+ pos 1) len)]
                                  (+ (* 12 1000000) end))
                                (if (is-digit-char c)
                                  (let [end (scan-digits src (+ pos 1) len)]
                                    (+ (* 10 1000000) end))
                                  (if (is-symbol-start c)
                                    (let [end (scan-symbol-end src (+ pos 1) len)
                                          name (substring src pos end)
                                          kind (classify-symbol name)]
                                      (+ (* kind 1000000) end))
                                    (+ (* 99 1000000) (+ pos 1))))))))))))))))
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
        (defn print-tokens [tokens i len]
          (if (>= i len) 0
            (do (print (vector-get tokens i))
                (print-tokens tokens (+ i 1) len))))
        (defn main []
          (let [tokens (tokenize "(defn main [] 42)")
                len (vector-length tokens)]
            (do
              (print len)
              (print-tokens tokens 0 len)
              0)))
    "#,
    );

    // L# Lexer の出力をパース
    let lsharp_lines: Vec<i64> = lsharp_result
        .trim()
        .lines()
        .map(|l| l.trim().parse::<i64>().unwrap())
        .collect();

    let lsharp_token_count = lsharp_lines[0] as usize;
    let lsharp_kinds: Vec<i64> = lsharp_lines[1..].to_vec();

    assert_eq!(
        lsharp_token_count,
        lsharp_kinds.len(),
        "L# Lexer: トークン数が一致しない"
    );

    // Rust Lexer と L# Lexer の結果を比較
    assert_eq!(
        rust_kinds, lsharp_kinds,
        "Rust Lexer と L# Lexer のトークン種別が一致しない\n\
         Rust: {:?}\nL#:   {:?}\n入力: {:?}",
        rust_kinds, lsharp_kinds, input
    );
}

/// Lexer 比較テスト: キーワード・コメント・文字列を含む入力
/// 注意: Lexer.ls は深いネスト if で classify-symbol の一部キーワード
/// (module, import 等) が未対応の場合があるため、基本キーワードのみテスト
#[test]
fn test_e2e_selfhost_lexer_comparison_keywords() {
    // テスト入力: 基本キーワードと各種リテラル
    let input = "(let [x 10] (if true x 0))";

    // --- Rust Lexer ---
    let mut rust_lexer = lsharp_syntax::lexer::Lexer::new(input);
    let rust_tokens = rust_lexer.tokenize().unwrap();
    let rust_kinds: Vec<i64> = rust_tokens
        .iter()
        .map(|t| {
            use lsharp_syntax::token::TokenKind;
            match &t.kind {
                TokenKind::LParen => 0,
                TokenKind::RParen => 1,
                TokenKind::LBracket => 2,
                TokenKind::RBracket => 3,
                TokenKind::LBrace => 4,
                TokenKind::RBrace => 5,
                TokenKind::Int(_) => 10,
                TokenKind::String(_) => 12,
                TokenKind::Bool(true) => 13,
                TokenKind::Bool(false) => 14,
                TokenKind::Symbol(_) => 20,
                TokenKind::Defn => 30,
                TokenKind::Let => 31,
                TokenKind::If => 32,
                TokenKind::Match => 33,
                TokenKind::Type => 34,
                TokenKind::Fn => 35,
                TokenKind::Do => 36,
                TokenKind::Module => 37,
                TokenKind::Import => 38,
                TokenKind::Colon => 50,
                TokenKind::Pipe => 52,
                TokenKind::Eof => 99,
                _ => 20,
            }
        })
        .collect();

    // --- L# Lexer ---
    let lsharp_result = compile_and_run(
        r#"
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
                (if (string-eq name "match") 33
                  (if (string-eq name "type") 34
                    (if (string-eq name "fn") 35
                      (if (string-eq name "do") 36
                        (if (string-eq name "true") 13
                          (if (string-eq name "false") 14 20))))))))))
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
        (defn print-tokens [tokens i len]
          (if (>= i len) 0
            (do (print (vector-get tokens i))
                (print-tokens tokens (+ i 1) len))))
        (defn main []
          (let [tokens (tokenize "(let [x 10] (if true x 0))")
                len (vector-length tokens)]
            (do
              (print len)
              (print-tokens tokens 0 len)
              0)))
    "#,
    );

    let lsharp_lines: Vec<i64> = lsharp_result
        .trim()
        .lines()
        .map(|l| l.trim().parse::<i64>().unwrap())
        .collect();

    let lsharp_token_count = lsharp_lines[0] as usize;
    let lsharp_kinds: Vec<i64> = lsharp_lines[1..].to_vec();

    assert_eq!(
        lsharp_token_count,
        lsharp_kinds.len(),
        "L# Lexer: トークン数が一致しない"
    );

    // 入力 "(let [x 10] (if true x 0))" の期待トークン:
    // ( let [ x 10 ] ( if true x 0 ) ) EOF
    // 0  31  2 20 10 3  0 32  13  20 10 1  1  99
    assert_eq!(
        rust_kinds, lsharp_kinds,
        "Rust Lexer と L# Lexer のトークン種別が一致しない\n\
         Rust: {:?}\nL#:   {:?}\n入力: {:?}",
        rust_kinds, lsharp_kinds, input
    );
}

#[test]
fn test_e2e_metadata_example_pass() {
    // :example アノテーション付き関数の自動テスト (成功ケース)
    let results = run_metadata_tests(r#"(defn add [x y] :example [(= (add 1 2) 3)] (+ x y))"#);
    assert_eq!(results.len(), 1);
    assert!(results[0].passed, ":example テストが成功するはず");
    assert_eq!(
        results[0].kind,
        lsharp_types::metadata_check::TestKind::Example
    );
    assert!(results[0].error.is_none());
}

#[test]
fn test_e2e_metadata_example_fail() {
    // :example アノテーション付き関数の自動テスト (失敗ケース)
    let results = run_metadata_tests(r#"(defn add [x y] :example [(= (add 1 2) 999)] (+ x y))"#);
    assert_eq!(results.len(), 1);
    assert!(!results[0].passed, ":example テストが失敗するはず");
    assert!(results[0].error.is_some());
}

#[test]
fn test_e2e_metadata_invariant_pass() {
    // :invariant アノテーション付き関数の不変条件検証 (成功ケース)
    let results =
        run_metadata_tests(r#"(defn abs [x] :invariant (>= result 0) (if (< x 0) (- 0 x) x))"#);
    assert_eq!(results.len(), 1);
    assert!(
        results[0].passed,
        ":invariant テストが成功するはず: {:?}",
        results[0].error
    );
    assert_eq!(
        results[0].kind,
        lsharp_types::metadata_check::TestKind::Invariant
    );
}

#[test]
fn test_e2e_metadata_example_and_invariant() {
    // :example と :invariant の両方を持つ関数のフルパイプラインテスト
    let results = run_metadata_tests(
        r#"(defn abs [x] :invariant (>= result 0) :example [(= (abs 5) 5)] (if (< x 0) (- 0 x) x))"#,
    );
    assert_eq!(results.len(), 2);
    let invariant_result = results
        .iter()
        .find(|r| r.kind == lsharp_types::metadata_check::TestKind::Invariant)
        .unwrap();
    assert!(
        invariant_result.passed,
        ":invariant テストが成功するはず: {:?}",
        invariant_result.error
    );
    let example_result = results
        .iter()
        .find(|r| r.kind == lsharp_types::metadata_check::TestKind::Example)
        .unwrap();
    assert!(
        example_result.passed,
        ":example テストが成功するはず: {:?}",
        example_result.error
    );
}
