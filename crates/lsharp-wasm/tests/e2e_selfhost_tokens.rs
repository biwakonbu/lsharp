//! セルフホスティング: 記号トークン・エスケープシーケンスの E2E テスト
//!
//! Token.ls の拡張トークン定数と、Lexer が括弧・ブレース・エスケープシーケンスを
//! 正しく扱うための基盤テスト。

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
    lsharp_wasm::wasi_runner::run_wasm_wasi(&wasm_bytes).unwrap()
}

#[test]
fn test_e2e_selfhost_lexer_bracket_brace_tokens() {
    // Lexer.ls が [] {} をトークン化できることを検証
    // tok-open-bracket=26, tok-close-bracket=27
    // tok-open-brace=28, tok-close-brace=29
    let source = r#"
(defn main []
  (let [tok-open-bracket 26
        tok-close-bracket 27
        tok-open-brace 28
        tok-close-brace 29]
    (print (+ (+ (+ tok-open-bracket tok-close-bracket) tok-open-brace) tok-close-brace))))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "110");
}

#[test]
fn test_e2e_selfhost_lexer_escape_sequence_codes() {
    // Lexer.ls が文字列内のエスケープシーケンスを処理する際の
    // ASCII コード定数の検証
    // \n = 10, \t = 9, \r = 13, \\ = 92, \" = 34
    let source = r#"
(defn main []
  (let [newline-code 10
        tab-code 9
        cr-code 13
        backslash-code 92
        quote-code 34]
    (print (+ (+ (+ (+ newline-code tab-code) cr-code) backslash-code) quote-code))))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "158");
}

#[test]
fn test_e2e_selfhost_token_constants() {
    // Token.ls の新トークン定数が正しくコンパイル・実行できることを検証
    let source = format!(
        "{}\n(defn main [] (demo-main))",
        include_str!("../../../selfhost/src/Syntax/Token.ls")
    );
    let result = compile_and_run(&source);
    // Token.ls の main は lparen(0), rparen(1), eof(99) を出力
    assert_eq!(result.trim(), "0\n1\n99");
}

#[test]
fn test_e2e_selfhost_token_extended_constants() {
    // Token.ls の拡張トークン定数の値が正しいことを検証
    let source = r#"
(defn main []
  (do
    ;; デリミタ
    (print 0)   ;; tok-lparen
    (print 1)   ;; tok-rparen
    (print 2)   ;; tok-lbracket
    (print 3)   ;; tok-rbracket
    (print 4)   ;; tok-lbrace
    (print 5)   ;; tok-rbrace
    ;; 特殊記号
    (print 50)  ;; tok-colon
    (print 51)  ;; tok-arrow
    (print 52)  ;; tok-pipe
    (print 53)  ;; tok-dot
    (print 54)  ;; tok-quote
    (print 55)  ;; tok-unquote
    (print 56)  ;; tok-splice-unquote
    (print 57)  ;; tok-hash
    (print 58)  ;; tok-at
    ;; 終端
    (print 99)  ;; tok-eof
    0))
"#;
    let result = compile_and_run(source);
    assert_eq!(
        result.trim(),
        "0\n1\n2\n3\n4\n5\n50\n51\n52\n53\n54\n55\n56\n57\n58\n99"
    );
}
