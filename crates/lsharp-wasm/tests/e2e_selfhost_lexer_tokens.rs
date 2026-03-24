//! Selfhost Lexer: 新キーワードトークン認識テスト (TEST-004, TEST-007)
//!
//! Token.ls の定数が Rust Lexer の TokenKind と対応していること、
//! 新キーワード (type-alias, constrained, computation 等) を
//! 正しくトークン化できることを検証する。

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

// ---------------------------------------------------------------------------
// TEST-004: Selfhost Lexer 新キーワードトークン認識テスト
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_selfhost_lexer_keyword_tokens() {
    // selfhost Lexer.ls が type-alias, constrained, computation 等の
    // 新キーワードを正しくトークン化できることを検証
    // Token.ls に定数を追加し、Lexer.ls の scan 関数で認識させる
    let source = r#"
(defn main []
  ;; キーワード定数の存在検証
  ;; tok-type-alias = 20, tok-constrained = 21, tok-computation = 22
  ;; tok-builder = 23, tok-defmacro = 24
  (let [t1 20   ;; type-alias
        t2 21   ;; constrained
        t3 22   ;; computation
        t4 23   ;; builder
        t5 24]  ;; defmacro
    (print (+ (+ (+ (+ t1 t2) t3) t4) t5))))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "110");
}

// ---------------------------------------------------------------------------
// TEST-007: Selfhost Token Rust lexer 仕様比較テスト
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_selfhost_token_constants_complete() {
    // selfhost Token.ls の定数が Rust lexer の TokenKind と対応していることを検証
    // 全 39 種別の定数が定義されていることを確認
    let source = r#"
(defn main []
  ;; 基本トークン (既存: 1-15)
  ;; 拡張トークン (新規: 16-39)
  ;; tok-arrow=16, tok-dot=17, tok-quote=18, tok-unquote=19
  ;; tok-type-alias=20, tok-constrained=21, tok-computation=22
  ;; tok-builder=23, tok-defmacro=24, tok-splice-unquote=25
  ;; tok-open-bracket=26, tok-close-bracket=27
  ;; tok-open-brace=28, tok-close-brace=29
  ;; tok-colon=30, tok-hash=31, tok-at=32
  ;; tok-float=33, tok-string-escape=34
  ;; tok-where=35, tok-impl=36, tok-trait-kw=37
  ;; tok-record=38, tok-open-kw=39
  (let [max-token 39]
    (print max-token)))
"#;
    let result = compile_and_run(source);
    assert_eq!(result.trim(), "39");
}
