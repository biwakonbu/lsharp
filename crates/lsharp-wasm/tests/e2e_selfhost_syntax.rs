//! selfhost syntax / parser integration tests extracted from e2e.rs

mod common;
use common::*;

// Phase 6 Group C: selfhost 拡張テスト (TDD Red Phase)
// =============================================================================

/// TEST-SYNTAX-03: Parser recovery + 複数診断収集
///
/// selfhost/src/Syntax/Parser.ls に recovery point が実装されていること、
/// 不正入力で複数の診断 [severity code span message-hash] を収集できることを検証。
/// 現状: Parser.ls に recovery 機構なし → FAIL
#[test]
fn test_e2e_selfhost_parser_recovery_diagnostics() {
    // Parser.ls を読み込み
    let parser_ls_path = selfhost_source_path("Parser.ls");
    assert!(
        parser_ls_path.exists(),
        "selfhost/src/Syntax/Parser.ls が存在しない"
    );
    let parser_content =
        std::fs::read_to_string(&parser_ls_path).expect("selfhost/src/Syntax/Parser.ls の読み込みに失敗");

    // recovery 関連の関数が定義されていることを検証
    assert!(
        parser_content.contains("parse-with-recovery")
            || parser_content.contains("recover-to-next"),
        "selfhost/src/Syntax/Parser.ls に recovery 機構 (parse-with-recovery / recover-to-next) が未実装"
    );

    // 診断収集関数が定義されていることを検証
    assert!(
        parser_content.contains("collect-diagnostics")
            || parser_content.contains("make-diagnostic"),
        "selfhost/src/Syntax/Parser.ls に診断収集 (collect-diagnostics / make-diagnostic) が未実装"
    );
}

/// TEST-SYNTAX-02b: module/import/type 宣言が AST 正本タグを使う
///
/// selfhost Parser が module/import/type を独自ダミータグではなく
/// AST.ls の canonical tag (`ast-module-decl`, `ast-import-decl`, `ast-type-decl`)
/// で返すことを、selfhost 実装自身を実行して検証する。
#[test]
fn test_e2e_selfhost_parser_decl_ast_tags() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [src1 "(module Foo)"
        program1 (parse-program src1)
        node1 (vector-get program1 0)
        src2 "(import Bar)"
        program2 (parse-program src2)
        node2 (vector-get program2 0)
        src3 "(type Baz)"
        program3 (parse-program src3)
        node3 (vector-get program3 0)]
    (do
      (print (vector-get node1 0))
      (print (if (= (vector-get node1 1) (name-hash src1 8 11)) 1 0))
      (print (vector-get node2 0))
      (print (if (= (vector-get node2 1) (name-hash src2 8 11)) 1 0))
      (print (vector-get node3 0))
      (print (if (= (vector-get node3 1) (name-hash src3 6 9)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "module/import/type の parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "25", "module は ast-module-decl (=25) を返すべき");
    assert_eq!(
        lines[1], "1",
        "module 名ハッシュが Parser.name-hash と一致すべき"
    );
    assert_eq!(lines[2], "26", "import は ast-import-decl (=26) を返すべき");
    assert_eq!(
        lines[3], "1",
        "import 名ハッシュが Parser.name-hash と一致すべき"
    );
    assert_eq!(lines[4], "21", "type は ast-type-decl (=21) を返すべき");
    assert_eq!(
        lines[5], "1",
        "type 名ハッシュが Parser.name-hash と一致すべき"
    );
}

/// TEST-SYNTAX-02c: 可変長ノードの count フィールドが実要素数を持つ
///
/// selfhost Parser が lambda/do/apply/match/defn の可変長ノードで、
/// count フィールドを 0 のまま残さず、実際の引数数・式数・腕数・param 数へ
/// 正しく更新して返すことを検証する。
#[test]
fn test_e2e_selfhost_parser_count_fields() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [lambda-node (vector-get (parse-program "(fn [x y] x)") 0)
        do-node (vector-get (parse-program "(do 1 2 3)") 0)
        apply-node (vector-get (parse-program "(f 1 2)") 0)
        match-node (vector-get (parse-program "(match x [1 10] [2 20])") 0)
        defn-node (vector-get (parse-program "(defn foo [x y] x)") 0)]
    (do
      (print (vector-get lambda-node 1))
      (print (vector-get do-node 1))
      (print (vector-get apply-node 2))
      (print (vector-get match-node 2))
      (print (vector-get defn-node 2))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 5, "count field 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "2", "lambda の param-count は 2 であるべき");
    assert_eq!(lines[1], "3", "do の expr-count は 3 であるべき");
    assert_eq!(lines[2], "2", "apply の arg-count は 2 であるべき");
    assert_eq!(lines[3], "2", "match の arm-count は 2 であるべき");
    assert_eq!(lines[4], "2", "defn の param-count は 2 であるべき");
}

/// TEST-SYNTAX-02c2: nested module を body 付きでパースできる
#[test]
fn test_e2e_selfhost_parser_nested_module_decl() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(module App (module Sub (defn inner [] 42)))") 0)
        inner (vector-get node 3)
        inner-defn (vector-get inner 3)]
    (do
      (print (if (= (vector-get node 0) (ast-module-decl)) 1 0))
      (print (if (= (vector-get node 1) (name-hash "App" 0 3)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get inner 0) (ast-module-decl)) 1 0))
      (print (if (= (vector-get inner 1) (name-hash "Sub" 0 3)) 1 0))
      (print (vector-get inner 2))
      (print (if (= (vector-get inner-defn 0) (ast-defn)) 1 0))
      (print (if (= (vector-get inner-defn 1) (name-hash "inner" 0 5)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 8,
        "nested module parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "outer node は module decl であるべき");
    assert_eq!(lines[1], "1", "outer module 名 hash が一致すべき");
    assert_eq!(lines[2], "1", "outer module body-count は 1 であるべき");
    assert_eq!(lines[3], "1", "inner node も module decl であるべき");
    assert_eq!(lines[4], "1", "inner module 名 hash が一致すべき");
    assert_eq!(lines[5], "1", "inner module body-count は 1 であるべき");
    assert_eq!(lines[6], "1", "inner body は defn であるべき");
    assert_eq!(lines[7], "1", "inner defn 名 hash が一致すべき");
}

/// TEST-SYNTAX-02d: quote/unquote 系トークンを AST ノードへパースできる
///
/// selfhost Parser が `'expr`, `~expr`, `~@expr` を
/// ast-quote / ast-unquote / ast-unquote-splice として返し、
/// 内側の式もそのまま保持することを検証する。
#[test]
fn test_e2e_selfhost_parser_quote_forms() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [quote-node (vector-get (parse-program "'foo") 0)
        unquote-node (vector-get (parse-program "~bar") 0)
        splice-node (vector-get (parse-program "~@baz") 0)]
    (do
      (print (if (= (vector-get quote-node 0) (ast-quote)) 1 0))
      (print (if (= (vector-get (vector-get quote-node 1) 0) (ast-var)) 1 0))
      (print (if (= (vector-get (vector-get quote-node 1) 1) (name-hash "foo" 0 3)) 1 0))
      (print (if (= (vector-get unquote-node 0) (ast-unquote)) 1 0))
      (print (if (= (vector-get (vector-get unquote-node 1) 0) (ast-var)) 1 0))
      (print (if (= (vector-get (vector-get unquote-node 1) 1) (name-hash "bar" 0 3)) 1 0))
      (print (if (= (vector-get splice-node 0) (ast-unquote-splice)) 1 0))
      (print (if (= (vector-get (vector-get splice-node 1) 0) (ast-var)) 1 0))
      (print (if (= (vector-get (vector-get splice-node 1) 1) (name-hash "baz" 0 3)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 9, "quote parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "quote ノードは ast-quote であるべき");
    assert_eq!(lines[1], "1", "quote 内側は var ノードであるべき");
    assert_eq!(lines[2], "1", "quote 内側の name-hash が一致すべき");
    assert_eq!(lines[3], "1", "unquote ノードは ast-unquote であるべき");
    assert_eq!(lines[4], "1", "unquote 内側は var ノードであるべき");
    assert_eq!(lines[5], "1", "unquote 内側の name-hash が一致すべき");
    assert_eq!(
        lines[6], "1",
        "splice-unquote ノードは ast-unquote-splice であるべき"
    );
    assert_eq!(lines[7], "1", "splice-unquote 内側は var ノードであるべき");
    assert_eq!(
        lines[8], "1",
        "splice-unquote 内側の name-hash が一致すべき"
    );
}

/// TEST-SYNTAX-02e: record / trait 宣言も canonical decl tag を返す
///
/// selfhost Parser が `(type Name (record ...))` を ast-recorddef、
/// `(trait (Name a) ...)` を ast-traitdef として返し、
/// 先頭名の hash も保持することを検証する。
#[test]
fn test_e2e_selfhost_parser_record_and_trait_decl_tags() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [record-node (vector-get (parse-program "(type Point (record (: x Int) (: y Int)))") 0)
        trait-node (vector-get (parse-program "(trait (Show a) (defn show [self] : String))") 0)]
    (do
      (print (vector-get record-node 0))
      (print (if (= (vector-get record-node 1) (name-hash "Point" 0 5)) 1 0))
      (print (vector-get trait-node 0))
      (print (if (= (vector-get trait-node 1) (name-hash "Show" 0 4)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "record/trait parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "22",
        "record type は ast-recorddef (=22) を返すべき"
    );
    assert_eq!(lines[1], "1", "record type 名ハッシュが一致すべき");
    assert_eq!(lines[2], "27", "trait は ast-traitdef (=27) を返すべき");
    assert_eq!(lines[3], "1", "trait 名ハッシュが一致すべき");
}

/// TEST-SYNTAX-02f: record literal を AST ノードにパースできる
///
/// selfhost Parser が `{Point x 10 y 20}` を ast-recordlit として返し、
/// type 名と field-count / field 名 / 値ノードを保持することを検証する。
#[test]
fn test_e2e_selfhost_parser_record_literal() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [record-node (vector-get (parse-program "{Point x 10 y 20}") 0)]
    (do
      (print (if (= (vector-get record-node 0) (ast-recordlit)) 1 0))
      (print (if (= (vector-get record-node 1) (name-hash "Point" 0 5)) 1 0))
      (print (vector-get record-node 2))
      (print (if (= (vector-get record-node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get (vector-get record-node 4) 0) (ast-lit-int)) 1 0))
      (print (if (= (vector-get record-node 5) (name-hash "y" 0 1)) 1 0))
      (print (if (= (vector-get (vector-get record-node 6) 0) (ast-lit-int)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 7,
        "record literal parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "record literal は ast-recordlit であるべき");
    assert_eq!(lines[1], "1", "record literal type 名ハッシュが一致すべき");
    assert_eq!(lines[2], "2", "record literal field-count は 2 であるべき");
    assert_eq!(lines[3], "1", "field x の name-hash が一致すべき");
    assert_eq!(lines[4], "1", "field x の値は int literal であるべき");
    assert_eq!(lines[5], "1", "field y の name-hash が一致すべき");
    assert_eq!(lines[6], "1", "field y の値は int literal であるべき");
}

/// TEST-SYNTAX-02g: defmacro が canonical tag でパースされ collect-macros に拾われる
///
/// selfhost Parser が `(defmacro ...)` を ast-defmacro として返し、
/// MacroExpand.collect-macros がそのノードを収集できることを検証する。
#[test]
fn test_e2e_selfhost_parser_defmacro_collect() {
    let (token_ls, ast_ls, lexer_ls, parser_ls, macroexpand_ls) =
        parser_macroexpand_runtime_modules();

    let harness = r#"
(defn main []
  (let [src "(defmacro double [x] '(+ ~x ~x))"
        program (parse-program src)
        node (vector-get program 0)
        table (collect-macros program)
        name-h (name-hash "double" 0 6)
        param-h (name-hash "x" 0 1)
        entry (macro-table-get table name-h)]
    (do
      (print (if (= (vector-get node 0) (ast-defmacro)) 1 0))
      (print (if (= (vector-get node 1) name-h) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get (vector-get node 4) 0) (ast-quote)) 1 0))
      (print (if (= entry 0) 0 1))
      (print (if (= entry 0) 0 (if (= (entry-param-count entry) 1) 1 0)))
      (print (if (= entry 0) 0 (if (= (entry-param-hash entry 0) param-h) 1 0)))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, macroexpand_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 7, "defmacro parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "defmacro は ast-defmacro であるべき");
    assert_eq!(lines[1], "1", "defmacro 名ハッシュが一致すべき");
    assert_eq!(lines[2], "1", "defmacro の param-count は 1 であるべき");
    assert_eq!(lines[3], "1", "defmacro body は quote ノードであるべき");
    assert_eq!(lines[4], "1", "collect-macros が defmacro を拾うべき");
    assert_eq!(lines[5], "1", "macro entry の param-count は 1 であるべき");
    assert_eq!(lines[6], "1", "macro entry の param hash が一致すべき");
}

/// TEST-SYNTAX-02h: private 宣言が canonical tag でパースされる
#[test]
fn test_e2e_selfhost_parser_private_decl() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(private (defn foo [] 1))") 0)
        inner (vector-get node 1)]
    (do
      (print (if (= (vector-get node 0) (ast-private)) 1 0))
      (print (if (= (vector-get inner 0) (ast-defn)) 1 0))
      (print (if (= (vector-get inner 1) (name-hash "foo" 0 3)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "private parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "private は ast-private であるべき");
    assert_eq!(lines[1], "1", "private の内側は ast-defn であるべき");
    assert_eq!(lines[2], "1", "inner defn 名ハッシュが一致すべき");
}

/// TEST-SYNTAX-02i: record update を AST ノードにパースできる
#[test]
fn test_e2e_selfhost_parser_record_update() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "{p | x 10 y 20}") 0)
        base (vector-get node 1)]
    (do
      (print (if (= (vector-get node 0) (ast-recordupdate)) 1 0))
      (print (if (= (vector-get base 0) (ast-var)) 1 0))
      (print (if (= (vector-get base 1) (name-hash "p" 0 1)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get (vector-get node 4) 0) (ast-lit-int)) 1 0))
      (print (if (= (vector-get node 5) (name-hash "y" 0 1)) 1 0))
      (print (if (= (vector-get (vector-get node 6) 0) (ast-lit-int)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 8,
        "record update parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "record update は ast-recordupdate であるべき"
    );
    assert_eq!(lines[1], "1", "record update base は var であるべき");
    assert_eq!(lines[2], "1", "record update base 名ハッシュが一致すべき");
    assert_eq!(lines[3], "2", "record update field-count は 2 であるべき");
    assert_eq!(lines[4], "1", "field x の name-hash が一致すべき");
    assert_eq!(lines[5], "1", "field x の値は int literal であるべき");
    assert_eq!(lines[6], "1", "field y の name-hash が一致すべき");
    assert_eq!(lines[7], "1", "field y の値は int literal であるべき");
}

/// TEST-SYNTAX-02j: type-alias / type-constrained / computation-builder / impl を decl tag にパースできる
#[test]
fn test_e2e_selfhost_parser_extended_decl_forms() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [alias-node (vector-get (parse-program "(type-alias Str String)") 0)
        constrained-node (vector-get (parse-program "(type-constrained Natural Int :constraints [(>= 0)])") 0)
        builder-node (vector-get (parse-program "(computation-builder maybe-builder mb identity)") 0)
        impl-node (vector-get (parse-program "(impl (Show Int) (defn show [self] self))") 0)]
    (do
      (print (if (= (vector-get alias-node 0) (ast-typealias)) 1 0))
      (print (if (= (vector-get alias-node 1) (name-hash "Str" 0 3)) 1 0))
      (print (if (= (vector-get constrained-node 0) (ast-typeconstrained)) 1 0))
      (print (if (= (vector-get constrained-node 1) (name-hash "Natural" 0 7)) 1 0))
      (print (if (= (vector-get builder-node 0) (ast-computationbuilder)) 1 0))
      (print (if (= (vector-get builder-node 1) (name-hash "maybe-builder" 0 13)) 1 0))
      (print (if (= (vector-get builder-node 2) (name-hash "mb" 0 2)) 1 0))
      (print (if (= (vector-get builder-node 3) (name-hash "identity" 0 8)) 1 0))
      (print (if (= (vector-get impl-node 0) (ast-impldef)) 1 0))
      (print (if (= (vector-get impl-node 1) (name-hash "Show" 0 4)) 1 0))
      (print (if (= (vector-get impl-node 2) (name-hash "Int" 0 3)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 11,
        "extended decl parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "type-alias は ast-typealias であるべき");
    assert_eq!(lines[1], "1", "type-alias 名ハッシュが一致すべき");
    assert_eq!(
        lines[2], "1",
        "type-constrained は ast-typeconstrained であるべき"
    );
    assert_eq!(lines[3], "1", "type-constrained 名ハッシュが一致すべき");
    assert_eq!(
        lines[4], "1",
        "computation-builder は ast-computationbuilder であるべき"
    );
    assert_eq!(lines[5], "1", "builder 名ハッシュが一致すべき");
    assert_eq!(lines[6], "1", "bind 関数名ハッシュが一致すべき");
    assert_eq!(lines[7], "1", "return 関数名ハッシュが一致すべき");
    assert_eq!(lines[8], "1", "impl は ast-impldef であるべき");
    assert_eq!(lines[9], "1", "impl trait 名ハッシュが一致すべき");
    assert_eq!(lines[10], "1", "impl type 名ハッシュが一致すべき");
}

/// TEST-SYNTAX-02j2: trait / impl の body decl を最小 payload で保持できる
#[test]
fn test_e2e_selfhost_parser_trait_impl_bodies() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [trait-node (vector-get (parse-program "(trait (Show a) (defn show [self] : String))") 0)
        trait-defn (vector-get trait-node 3)
        impl-node (vector-get (parse-program "(impl (Show Int) (defn show [self] (str self)))") 0)
        impl-defn (vector-get impl-node 4)]
    (do
      (print (if (= (vector-get trait-node 0) (ast-traitdef)) 1 0))
      (print (if (= (vector-get trait-node 1) (name-hash "Show" 0 4)) 1 0))
      (print (vector-get trait-node 2))
      (print (if (= (vector-get trait-defn 0) (ast-defn)) 1 0))
      (print (if (= (vector-get trait-defn 1) (name-hash "show" 0 4)) 1 0))
      (print (if (= (vector-get impl-node 0) (ast-impldef)) 1 0))
      (print (if (= (vector-get impl-node 1) (name-hash "Show" 0 4)) 1 0))
      (print (if (= (vector-get impl-node 2) (name-hash "Int" 0 3)) 1 0))
      (print (vector-get impl-node 3))
      (print (if (= (vector-get impl-defn 0) (ast-defn)) 1 0))
      (print (if (= (vector-get impl-defn 1) (name-hash "show" 0 4)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 11,
        "trait/impl body parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "trait は ast-traitdef であるべき");
    assert_eq!(lines[1], "1", "trait 名 hash は Show であるべき");
    assert_eq!(lines[2], "1", "trait body-count は 1 であるべき");
    assert_eq!(lines[3], "1", "trait body は defn であるべき");
    assert_eq!(lines[4], "1", "trait method 名 hash は show であるべき");
    assert_eq!(lines[5], "1", "impl は ast-impldef であるべき");
    assert_eq!(lines[6], "1", "impl trait hash は Show であるべき");
    assert_eq!(lines[7], "1", "impl type hash は Int であるべき");
    assert_eq!(lines[8], "1", "impl body-count は 1 であるべき");
    assert_eq!(lines[9], "1", "impl body は defn であるべき");
    assert_eq!(lines[10], "1", "impl method 名 hash は show であるべき");
}

/// TEST-SYNTAX-02j3: type-constrained の主要 constraint 形式をスキップできる
#[test]
fn test_e2e_selfhost_parser_type_constrained_constraint_forms() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [range-node (vector-get (parse-program "(type-constrained Percentage Int :constraints [(>= 0) (<= 100)])") 0)
        matches-node (vector-get (parse-program "(type-constrained Email String :constraints [(matches \"^[^@]+@[^@]+$\")])") 0)
        satisfies-node (vector-get (parse-program "(type-constrained EvenInt Int :constraints [(satisfies is-even)])") 0)]
    (do
      (print (if (= (vector-get range-node 0) (ast-typeconstrained)) 1 0))
      (print (if (= (vector-get range-node 1) (name-hash "Percentage" 0 10)) 1 0))
      (print (if (= (vector-get matches-node 0) (ast-typeconstrained)) 1 0))
      (print (if (= (vector-get matches-node 1) (name-hash "Email" 0 5)) 1 0))
      (print (if (= (vector-get satisfies-node 0) (ast-typeconstrained)) 1 0))
      (print (if (= (vector-get satisfies-node 1) (name-hash "EvenInt" 0 7)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "type-constrained parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "range constraint は ast-typeconstrained であるべき"
    );
    assert_eq!(lines[1], "1", "range constraint 名 hash が一致すべき");
    assert_eq!(
        lines[2], "1",
        "matches constraint は ast-typeconstrained であるべき"
    );
    assert_eq!(lines[3], "1", "matches constraint 名 hash が一致すべき");
    assert_eq!(
        lines[4], "1",
        "satisfies constraint は ast-typeconstrained であるべき"
    );
    assert_eq!(lines[5], "1", "satisfies constraint 名 hash が一致すべき");
}

/// TEST-SYNTAX-02j4: 空 S 式 `()` を unit literal としてパースできる
#[test]
fn test_e2e_selfhost_parser_unit_literal() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "()") 0)]
    (do
      (print (if (= (vector-get node 0) (ast-lit-unit)) 1 0))
      (print (vector-length node))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "unit parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "unit literal は ast-lit-unit であるべき");
    assert_eq!(lines[1], "1", "unit literal node length は 1 であるべき");
}

/// TEST-SYNTAX-02k: if 式を明示的に ast-if としてパースできる
#[test]
fn test_e2e_selfhost_parser_if_expr() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(if true 1 0)") 0)
        cond-node (vector-get node 1)
        then-node (vector-get node 2)
        else-node (vector-get node 3)]
    (do
      (print (if (= (vector-get node 0) (ast-if)) 1 0))
      (print (if (= (vector-get cond-node 0) (ast-lit-bool)) 1 0))
      (print (if (= (vector-get then-node 0) (ast-lit-int)) 1 0))
      (print (if (= (vector-get else-node 0) (ast-lit-int)) 1 0))
      (print (vector-get then-node 1))
      (print (vector-get else-node 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 6, "if expr parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "if は ast-if であるべき");
    assert_eq!(lines[1], "1", "cond は bool literal であるべき");
    assert_eq!(lines[2], "1", "then は int literal であるべき");
    assert_eq!(lines[3], "1", "else は int literal であるべき");
    assert_eq!(lines[4], "1", "then value は 1 であるべき");
    assert_eq!(lines[5], "0", "else value は 0 であるべき");
}

/// TEST-SYNTAX-02l1: match の `_` パターンを wildcard としてパースできる
#[test]
fn test_e2e_selfhost_parser_match_wildcard_pattern() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match 1 [_ 2] [rest 3])") 0)
        pat1 (vector-get node 3)
        body1 (vector-get node 4)
        pat2 (vector-get node 5)]
    (do
      (print (if (= (vector-get node 0) (ast-match)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get pat1 0) (ast-pat-wildcard)) 1 0))
      (print (if (= (vector-get body1 0) (ast-lit-int)) 1 0))
      (print (if (= (vector-get body1 1) 2) 1 0))
      (print (if (= (vector-get pat2 0) (ast-pat-var)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "match wildcard parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "match ノードは ast-match であるべき");
    assert_eq!(lines[1], "2", "arm-count は 2 であるべき");
    assert_eq!(
        lines[2], "1",
        "先頭 arm の `_` は wildcard パターンであるべき"
    );
    assert_eq!(lines[3], "1", "先頭 arm の body は整数リテラルであるべき");
    assert_eq!(lines[4], "1", "先頭 arm の body 値は 2 であるべき");
    assert_eq!(
        lines[5], "1",
        "通常の symbol pattern は ast-pat-var であるべき"
    );
}

/// TEST-SYNTAX-02l2: match の symbol pattern を pattern tag としてパースできる
#[test]
fn test_e2e_selfhost_parser_match_var_pattern_tag() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match 1 [rest 3])") 0)
        pat (vector-get node 3)]
    (do
      (print (if (= (vector-get pat 0) (ast-pat-var)) 1 0))
      (print (if (= (vector-get pat 1) (name-hash "rest" 0 4)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "match var parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "symbol pattern は ast-pat-var であるべき");
    assert_eq!(
        lines[1], "1",
        "pattern name-hash は source slice と一致すべき"
    );
}

/// TEST-SYNTAX-02l3: match の constructor pattern を canonical tag としてパースできる
#[test]
fn test_e2e_selfhost_parser_match_constructor_pattern_tag() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match value [(Some rest) rest])") 0)
        pat (vector-get node 3)
        child (vector-get pat 3)]
    (do
      (print (if (= (vector-get pat 0) (ast-pat-constructor)) 1 0))
      (print (if (= (vector-get pat 1) (name-hash "Some" 0 4)) 1 0))
      (print (vector-get pat 2))
      (print (if (= (vector-get child 0) (ast-pat-var)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "match constructor parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "constructor pattern は ast-pat-constructor であるべき"
    );
    assert_eq!(
        lines[1], "1",
        "constructor name-hash は source slice と一致すべき"
    );
    assert_eq!(
        lines[2], "1",
        "constructor sub-pattern count は 1 であるべき"
    );
    assert_eq!(lines[3], "1", "constructor child は ast-pat-var であるべき");
}

/// TEST-SYNTAX-02l4: match の record pattern を canonical tag としてパースできる
#[test]
fn test_e2e_selfhost_parser_match_record_pattern_tag() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match value [{Point x rest} rest])") 0)
        pat (vector-get node 3)
        child (vector-get pat 3)]
    (do
      (print (if (= (vector-get pat 0) (ast-pat-recordpat)) 1 0))
      (print (vector-get pat 1))
      (print (if (= (vector-get pat 2) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get child 0) (ast-pat-var)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "match record parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "record pattern は ast-pat-recordpat であるべき"
    );
    assert_eq!(lines[1], "1", "record field count は 1 であるべき");
    assert_eq!(
        lines[2], "1",
        "record field hash は source slice と一致すべき"
    );
    assert_eq!(lines[3], "1", "record child は ast-pat-var であるべき");
}

/// TEST-SYNTAX-02l5: match の int/bool literal pattern を canonical tag としてパースできる
#[test]
fn test_e2e_selfhost_parser_match_literal_pattern_tag() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match value [1 2] [true 3] [rest 4])") 0)
        int-pat (vector-get node 3)
        bool-pat (vector-get node 5)
        int-lit (vector-get int-pat 1)
        bool-lit (vector-get bool-pat 1)]
    (do
      (print (if (= (vector-get int-pat 0) (ast-pat-lit)) 1 0))
      (print (if (= (vector-get int-lit 0) (ast-lit-int)) 1 0))
      (print (if (= (vector-get int-lit 1) 1) 1 0))
      (print (if (= (vector-get bool-pat 0) (ast-pat-lit)) 1 0))
      (print (if (= (vector-get bool-lit 0) (ast-lit-bool)) 1 0))
      (print (if (= (vector-get bool-lit 1) 1) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "match literal parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "int literal pattern は ast-pat-lit であるべき"
    );
    assert_eq!(
        lines[1], "1",
        "int literal payload は ast-lit-int であるべき"
    );
    assert_eq!(lines[2], "1", "int literal payload value は 1 であるべき");
    assert_eq!(
        lines[3], "1",
        "bool literal pattern は ast-pat-lit であるべき"
    );
    assert_eq!(
        lines[4], "1",
        "bool literal payload は ast-lit-bool であるべき"
    );
    assert_eq!(
        lines[5], "1",
        "bool literal payload value は true(1) であるべき"
    );
}

/// TEST-SYNTAX-02l5b: match の unit literal pattern を canonical tag としてパースできる
#[test]
fn test_e2e_selfhost_parser_match_unit_literal_pattern_tag() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match value [() 2] [rest 4])") 0)
        unit-pat (vector-get node 3)
        unit-lit (vector-get unit-pat 1)]
    (do
      (print (if (= (vector-get unit-pat 0) (ast-pat-lit)) 1 0))
      (print (if (= (vector-get unit-lit 0) (ast-lit-unit)) 1 0))
      (print (vector-length unit-lit))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "match unit literal parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "unit literal pattern は ast-pat-lit であるべき"
    );
    assert_eq!(
        lines[1], "1",
        "unit literal payload は ast-lit-unit であるべき"
    );
    assert_eq!(lines[2], "1", "unit literal payload の長さは 1 であるべき");
}

/// TEST-SYNTAX-02l6: nested constructor/record child でも literal pattern を canonicalize できる
#[test]
fn test_e2e_selfhost_parser_match_nested_literal_pattern_tag() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match value [(Some 1) 2] [{Point x true} 3])") 0)
        ctor-pat (vector-get node 3)
        record-pat (vector-get node 5)
        ctor-child (vector-get ctor-pat 3)
        record-child (vector-get record-pat 3)
        ctor-lit (vector-get ctor-child 1)
        record-lit (vector-get record-child 1)]
    (do
      (print (if (= (vector-get ctor-child 0) (ast-pat-lit)) 1 0))
      (print (if (= (vector-get ctor-lit 0) (ast-lit-int)) 1 0))
      (print (if (= (vector-get ctor-lit 1) 1) 1 0))
      (print (if (= (vector-get record-child 0) (ast-pat-lit)) 1 0))
      (print (if (= (vector-get record-lit 0) (ast-lit-bool)) 1 0))
      (print (if (= (vector-get record-lit 1) 1) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "match nested literal parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "constructor child literal pattern は ast-pat-lit であるべき"
    );
    assert_eq!(
        lines[1], "1",
        "constructor child payload は ast-lit-int であるべき"
    );
    assert_eq!(
        lines[2], "1",
        "constructor child payload value は 1 であるべき"
    );
    assert_eq!(
        lines[3], "1",
        "record child literal pattern は ast-pat-lit であるべき"
    );
    assert_eq!(
        lines[4], "1",
        "record child payload は ast-lit-bool であるべき"
    );
    assert_eq!(
        lines[5], "1",
        "record child payload value は true(1) であるべき"
    );
}

/// TEST-SYNTAX-02l: parametric type / type-alias head を decl tag にパースできる
#[test]
fn test_e2e_selfhost_parser_parametric_type_heads() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [type-node (vector-get (parse-program "(type (Pair a b) (record (: fst a) (: snd b)))") 0)
        alias-node (vector-get (parse-program "(type-alias (Callback a b) (-> a b))") 0)]
    (do
      (print (if (= (vector-get type-node 0) (ast-recorddef)) 1 0))
      (print (if (= (vector-get type-node 1) (name-hash "Pair" 0 4)) 1 0))
      (print (if (= (vector-get alias-node 0) (ast-typealias)) 1 0))
      (print (if (= (vector-get alias-node 1) (name-hash "Callback" 0 8)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "parametric type parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1",
        "parametric type record は ast-recorddef であるべき"
    );
    assert_eq!(lines[1], "1", "parametric type 名 hash は Pair であるべき");
    assert_eq!(
        lines[2], "1",
        "parametric type-alias は ast-typealias であるべき"
    );
    assert_eq!(
        lines[3], "1",
        "parametric alias 名 hash は Callback であるべき"
    );
}

/// TEST-SYNTAX-02m: annotation form を AST ノードにパースできる
#[test]
fn test_e2e_selfhost_parser_ann_form() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(: 42 Int)") 0)
        inner (vector-get node 1)]
    (do
      (print (if (= (vector-get node 0) (ast-ann)) 1 0))
      (print (if (= (vector-get inner 0) (ast-lit-int)) 1 0))
      (print (vector-get inner 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "annotation parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "annotation は ast-ann であるべき");
    assert_eq!(lines[1], "1", "annotation inner は int literal であるべき");
    assert_eq!(lines[2], "42", "annotation inner の値が保持されるべき");
}

/// TEST-SYNTAX-02n: float literal を lexer/parser で扱える
#[test]
fn test_e2e_selfhost_parser_float_literal() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [src "3.14"
        tokens (tokenize-with-spans src)
        node (vector-get (parse-program src) 0)]
    (do
      (print (if (= (token-kind tokens 0) (tok-float)) 1 0))
      (print (if (= (vector-get node 0) (ast-lit-float)) 1 0))
      (print (vector-get node 1))
      (print (vector-get node 2))
      (print (if (string-eq (substring src (vector-get node 1) (vector-get node 2)) "3.14") 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 5, "float parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "3.14 は tok-float であるべき");
    assert_eq!(lines[1], "1", "float literal は ast-lit-float であるべき");
    assert_eq!(lines[2], "0", "float literal の start は 0 であるべき");
    assert_eq!(lines[3], "4", "float literal の end は 4 であるべき");
    assert_eq!(lines[4], "1", "float literal の lexeme が保持されるべき");
}

/// TEST-SYNTAX-02o: computation expression を最小 payload でパースできる
#[test]
fn test_e2e_selfhost_parser_computation_expr() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(computation maybe-builder (let! x m) (do! side) value (return x))") 0)
        step1-expr (vector-get node 5)
        step2-expr (vector-get node 8)
        step3-expr (vector-get node 11)
        step4-expr (vector-get node 14)]
    (do
      (print (if (= (vector-get node 0) (ast-computation)) 1 0))
      (print (if (= (vector-get node 1) (name-hash "maybe-builder" 0 13)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (computation-step-let-bang)) 1 0))
      (print (if (= (vector-get node 4) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get step1-expr 0) (ast-var)) 1 0))
      (print (if (= (vector-get step1-expr 1) (name-hash "m" 0 1)) 1 0))
      (print (if (= (vector-get node 6) (computation-step-do-bang)) 1 0))
      (print (if (= (vector-get step2-expr 1) (name-hash "side" 0 4)) 1 0))
      (print (if (= (vector-get node 9) (computation-step-expr)) 1 0))
      (print (if (= (vector-get step3-expr 1) (name-hash "value" 0 5)) 1 0))
      (print (if (= (vector-get node 12) (computation-step-return)) 1 0))
      (print (if (= (vector-get step4-expr 1) (name-hash "x" 0 1)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 13,
        "computation parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "computation は ast-computation であるべき");
    assert_eq!(lines[1], "1", "builder 名ハッシュが一致すべき");
    assert_eq!(lines[2], "4", "step count は 4 であるべき");
    assert_eq!(lines[3], "1", "step1 は let! であるべき");
    assert_eq!(lines[4], "1", "step1 pattern hash が一致すべき");
    assert_eq!(lines[5], "1", "step1 expr は var であるべき");
    assert_eq!(lines[6], "1", "step1 expr の hash が一致すべき");
    assert_eq!(lines[7], "1", "step2 は do! であるべき");
    assert_eq!(lines[8], "1", "step2 expr の hash が一致すべき");
    assert_eq!(lines[9], "1", "step3 は plain expr であるべき");
    assert_eq!(lines[10], "1", "step3 expr の hash が一致すべき");
    assert_eq!(lines[11], "1", "step4 は return であるべき");
    assert_eq!(lines[12], "1", "step4 expr の hash が一致すべき");
}

/// TEST-SYNTAX-02p: defn の annotated param / return type を最小 payload でスキップできる
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn add [(: x Int) (: y Int)] : Int (+ x y))") 0)
        body (vector-get node 5)]
    (do
      (print (if (= (vector-get node 0) 20) 1 0))
      (print (if (= (vector-get node 1) (name-hash "add" 0 3)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get node 4) (name-hash "y" 0 1)) 1 0))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "typed defn parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "defn tag が一致すべき");
    assert_eq!(lines[1], "1", "関数名 hash が一致すべき");
    assert_eq!(lines[2], "2", "param count は 2 であるべき");
    assert_eq!(lines[3], "1", "param1 hash は x であるべき");
    assert_eq!(lines[4], "1", "param2 hash は y であるべき");
    assert_eq!(lines[5], "1", "body は apply ノードであるべき");
}

/// TEST-SYNTAX-02q: defn の :where clause を最小 payload のままスキップできる
#[test]
fn test_e2e_selfhost_parser_defn_where_clause() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn show-it [x] :where [(Show a)] (show x))") 0)
        body (vector-get node 4)
        callee (vector-get body 1)
        arg1 (vector-get body 3)]
    (do
      (print (if (= (vector-get node 0) 20) 1 0))
      (print (if (= (vector-get node 1) (name-hash "show-it" 0 7)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (vector-get body 2))
      (print (if (= (vector-get callee 0) (ast-var)) 1 0))
      (print (if (= (vector-get callee 1) (name-hash "show" 0 4)) 1 0))
      (print (if (= (vector-get arg1 1) (name-hash "x" 0 1)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 9,
        "where defn parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "defn tag が一致すべき");
    assert_eq!(lines[1], "1", "関数名 hash が一致すべき");
    assert_eq!(lines[2], "1", "param count は 1 であるべき");
    assert_eq!(lines[3], "1", "param1 hash は x であるべき");
    assert_eq!(lines[4], "1", "body は apply ノードであるべき");
    assert_eq!(lines[5], "1", "apply arg count は 1 であるべき");
    assert_eq!(lines[6], "1", "callee は var ノードであるべき");
    assert_eq!(lines[7], "1", "callee hash は show であるべき");
    assert_eq!(lines[8], "1", "arg hash は x であるべき");
}

/// TEST-SYNTAX-02q2: defn の複数 :where clause をスキップして body を保てる
#[test]
fn test_e2e_selfhost_parser_defn_multiple_where_clauses() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn show-eq [x y] :where [(Show a) (Eq a)] (do (show x) (== x y)))") 0)
        body (vector-get node 5)]
    (do
      (print (if (= (vector-get node 0) (ast-defn)) 1 0))
      (print (if (= (vector-get node 1) (name-hash "show-eq" 0 7)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get node 4) (name-hash "y" 0 1)) 1 0))
      (print (if (= (vector-get body 0) (ast-do)) 1 0))
      (print (vector-get body 1))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 7,
        "multiple where parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "defn tag が一致すべき");
    assert_eq!(lines[1], "1", "関数名 hash が一致すべき");
    assert_eq!(lines[2], "2", "param count は 2 であるべき");
    assert_eq!(lines[3], "1", "param1 hash は x であるべき");
    assert_eq!(lines[4], "1", "param2 hash は y であるべき");
    assert_eq!(lines[5], "1", "body は do ノードであるべき");
    assert_eq!(lines[6], "2", "do expr-count は 2 であるべき");
}

/// TEST-SYNTAX-02r: defn の metadata directives を最小 payload のままスキップできる
#[test]
fn test_e2e_selfhost_parser_defn_metadata_directives() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn toggle [state] :invariant state :transitions [(Open -> Closed) (Closed -> Open)] (toggle-next state))") 0)
        body (vector-get node 4)]
    (do
      (print (if (= (vector-get node 0) 20) 1 0))
      (print (if (= (vector-get node 1) (name-hash "toggle" 0 6)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "state" 0 5)) 1 0))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (vector-get body 2))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 6,
        "metadata defn parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "defn tag が一致すべき");
    assert_eq!(lines[1], "1", "関数名 hash が一致すべき");
    assert_eq!(lines[2], "1", "param count は 1 であるべき");
    assert_eq!(lines[3], "1", "param1 hash は state であるべき");
    assert_eq!(lines[4], "1", "body は apply ノードであるべき");
    assert_eq!(lines[5], "1", "apply arg count は 1 であるべき");
}

/// TEST-SYNTAX-02s: defn の string metadata directives を最小 payload のままスキップできる
#[test]
fn test_e2e_selfhost_parser_defn_string_metadata() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn add [x y] :doc \"addition\" :returns \"sum\" (+ x y))") 0)
        body (vector-get node 5)]
    (do
      (print (if (= (vector-get node 0) 20) 1 0))
      (print (if (= (vector-get node 1) (name-hash "add" 0 3)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get node 4) (name-hash "y" 0 1)) 1 0))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (vector-get body 2))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 7,
        "string metadata parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "defn tag が一致すべき");
    assert_eq!(lines[1], "1", "関数名 hash が一致すべき");
    assert_eq!(lines[2], "2", "param count は 2 であるべき");
    assert_eq!(lines[3], "1", "param1 hash は x であるべき");
    assert_eq!(lines[4], "1", "param2 hash は y であるべき");
    assert_eq!(lines[5], "1", "body は apply ノードであるべき");
    assert_eq!(lines[6], "2", "apply arg count は 2 であるべき");
}

/// TEST-SYNTAX-02t: defn の params metadata を最小 payload のままスキップできる
#[test]
fn test_e2e_selfhost_parser_defn_params_metadata() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn add [x y] :doc \"addition\" :params [(x \"left\") (y \"right\")] :returns \"sum\" (+ x y))") 0)
        body (vector-get node 5)]
    (do
      (print (if (= (vector-get node 0) 20) 1 0))
      (print (if (= (vector-get node 1) (name-hash "add" 0 3)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get node 4) (name-hash "y" 0 1)) 1 0))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (vector-get body 2))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 7,
        "params metadata parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "defn tag が一致すべき");
    assert_eq!(lines[1], "1", "関数名 hash が一致すべき");
    assert_eq!(lines[2], "2", "param count は 2 であるべき");
    assert_eq!(lines[3], "1", "param1 hash は x であるべき");
    assert_eq!(lines[4], "1", "param2 hash は y であるべき");
    assert_eq!(lines[5], "1", "body は apply ノードであるべき");
    assert_eq!(lines[6], "2", "apply arg count は 2 であるべき");
}

/// TEST-SYNTAX-04: Hygiene.ls gensym/scope-id/expansion trace
///
/// selfhost/src/Syntax/Hygiene.ls が存在し、gensym, scope-id, expansion-trace 関数を公開していることを検証。
/// 現状: Hygiene.ls 未作成 → FAIL
#[test]
fn test_e2e_selfhost_hygiene_gensym() {
    // Hygiene.ls が存在することを検証
    let hygiene_ls_path = selfhost_source_path("Hygiene.ls");
    assert!(
        hygiene_ls_path.exists(),
        "selfhost/src/Syntax/Hygiene.ls が存在しない -- 衛生的マクロモジュール未作成"
    );

    let hygiene_content =
        std::fs::read_to_string(&hygiene_ls_path).expect("selfhost/src/Syntax/Hygiene.ls の読み込みに失敗");

    // 必須関数が定義されていることを検証
    assert!(
        hygiene_content.contains("(module Syntax.Hygiene)"),
        "selfhost/src/Syntax/Hygiene.ls に namespaced module 宣言がない"
    );
    assert!(
        hygiene_content.contains("(defn gensym"),
        "selfhost/src/Syntax/Hygiene.ls に gensym 関数が未定義"
    );
    assert!(
        hygiene_content.contains("(defn scope-id")
            || hygiene_content.contains("(defn make-scope-id"),
        "selfhost/src/Syntax/Hygiene.ls に scope-id 関数が未定義"
    );
    assert!(
        hygiene_content.contains("(defn expansion-trace")
            || hygiene_content.contains("(defn make-expansion-trace"),
        "selfhost/src/Syntax/Hygiene.ls に expansion-trace 関数が未定義"
    );
}

/// TEST-SYNTAX-05: Derive.ls expand-derives
///
/// selfhost/src/Syntax/Derive.ls が存在し、expand-derives 関数がヘルパー decl を生成できることを検証。
/// 現状: Derive.ls 未作成 → FAIL
#[test]
fn test_e2e_selfhost_derive_expansion() {
    // Derive.ls が存在することを検証
    let derive_ls_path = selfhost_source_path("Derive.ls");
    assert!(
        derive_ls_path.exists(),
        "selfhost/src/Syntax/Derive.ls が存在しない -- derive マクロモジュール未作成"
    );

    let derive_content =
        std::fs::read_to_string(&derive_ls_path).expect("selfhost/src/Syntax/Derive.ls の読み込みに失敗");

    // 必須関数が定義されていることを検証
    assert!(
        derive_content.contains("(module Syntax.Derive)"),
        "selfhost/src/Syntax/Derive.ls に namespaced module 宣言がない"
    );
    assert!(
        derive_content.contains("(defn expand-derives")
            || derive_content.contains("(defn expand-derive"),
        "selfhost/src/Syntax/Derive.ls に expand-derives 関数が未定義"
    );
}

/// TEST-SYNTAX-06: Syntax golden fixtures
///
/// tests/golden/syntax/ に tokens.json, ast.json, diagnostics.json の
/// golden fixture が存在し、内容が正しいことを検証。
/// 現状: golden fixture 未作成 → FAIL
#[test]
fn test_e2e_syntax_golden_fixtures() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let golden_dir = project_root.join("tests/golden/syntax");

    // ディレクトリが存在すること
    assert!(
        golden_dir.exists() && golden_dir.is_dir(),
        "tests/golden/syntax/ ディレクトリが存在しない"
    );

    // tokens.json が存在し、有効な JSON であること
    let tokens_path = golden_dir.join("tokens.json");
    assert!(
        tokens_path.exists(),
        "tests/golden/syntax/tokens.json が存在しない"
    );
    let tokens_content =
        std::fs::read_to_string(&tokens_path).expect("tokens.json の読み込みに失敗");
    let tokens: serde_json::Value =
        serde_json::from_str(&tokens_content).expect("tokens.json が有効な JSON でない");
    assert!(
        tokens.get("cases").is_some(),
        "tokens.json に cases セクションがない"
    );
    let token_cases = tokens["cases"]
        .as_array()
        .expect("tokens.json の cases が配列でない");
    assert!(
        token_cases.len() >= 3,
        "tokens.json のテストケースが 3 件未満: {}",
        token_cases.len()
    );

    // ast.json が存在し、有効な JSON であること
    let ast_path = golden_dir.join("ast.json");
    assert!(
        ast_path.exists(),
        "tests/golden/syntax/ast.json が存在しない"
    );
    let ast_content = std::fs::read_to_string(&ast_path).expect("ast.json の読み込みに失敗");
    let ast: serde_json::Value =
        serde_json::from_str(&ast_content).expect("ast.json が有効な JSON でない");
    assert!(
        ast.get("cases").is_some(),
        "ast.json に cases セクションがない"
    );
    let ast_cases = ast["cases"]
        .as_array()
        .expect("ast.json の cases が配列でない");
    assert!(
        ast_cases.len() >= 3,
        "ast.json のテストケースが 3 件未満: {}",
        ast_cases.len()
    );

    // diagnostics.json が存在し、有効な JSON であること
    let diag_path = golden_dir.join("diagnostics.json");
    assert!(
        diag_path.exists(),
        "tests/golden/syntax/diagnostics.json が存在しない"
    );
    let diag_content =
        std::fs::read_to_string(&diag_path).expect("diagnostics.json の読み込みに失敗");
    let diag: serde_json::Value =
        serde_json::from_str(&diag_content).expect("diagnostics.json が有効な JSON でない");
    assert!(
        diag.get("cases").is_some(),
        "diagnostics.json に cases セクションがない"
    );
    let diag_cases = diag["cases"]
        .as_array()
        .expect("diagnostics.json の cases が配列でない");
    assert!(
        diag_cases.len() >= 2,
        "diagnostics.json のテストケースが 2 件未満: {}",
        diag_cases.len()
    );

    // 各 fixture のケースが必須フィールドを持つこと
    for case in token_cases {
        assert!(
            case.get("input").is_some() && case.get("expected_tokens").is_some(),
            "tokens.json のケースに input / expected_tokens フィールドがない: {:?}",
            case
        );
    }
    for case in ast_cases {
        assert!(
            case.get("input").is_some() && case.get("expected_ast").is_some(),
            "ast.json のケースに input / expected_ast フィールドがない: {:?}",
            case
        );
    }
    for case in diag_cases {
        assert!(
            case.get("input").is_some() && case.get("expected_diagnostics").is_some(),
            "diagnostics.json のケースに input / expected_diagnostics フィールドがない: {:?}",
            case
        );
    }
}

/// TEST-TYPE-03: match 型推論 + infer-pattern
///
/// selfhost/src/Types/TypeInferPattern.ls に infer-pattern 関数があり、
/// match 式の型推論でコンストラクタパターンに対応していることを検証。
#[test]
fn test_e2e_selfhost_match_inference() {
    // TypeInferPattern.ls を読み込み (infer-pattern は TypeInferPattern.ls へ分離済み)
    let type_infer_pattern_path = selfhost_source_path("TypeInferPattern.ls");
    assert!(
        type_infer_pattern_path.exists(),
        "selfhost/src/Types/TypeInferPattern.ls が存在しない"
    );
    let type_infer_pattern_content =
        std::fs::read_to_string(&type_infer_pattern_path).expect("selfhost/src/Types/TypeInferPattern.ls の読み込みに失敗");

    // infer-pattern 関数が定義されていることを検証
    assert!(
        type_infer_pattern_content.contains("(defn infer-pattern"),
        "selfhost/src/Types/TypeInferPattern.ls に infer-pattern 関数が未定義 -- \
         match 式のパターン型推論が未実装"
    );

    // infer-pattern がコンストラクタパターン対応していることを検証
    assert!(
        type_infer_pattern_content.contains("constructor-pattern")
            || type_infer_pattern_content.contains("ctor-pattern")
            || type_infer_pattern_content.contains("tag-pattern"),
        "selfhost/src/Types/TypeInferPattern.ls の infer-pattern が \
         コンストラクタパターンに対応していない"
    );
}

/// TEST-TYPE-04: Constraints.ls trait/where/constraint solving
///
/// selfhost/src/Types/Constraints.ls が存在し、trait registry, impl registry,
/// constraint solver を公開していることを検証。
/// 現状: Constraints.ls 未作成 → FAIL
#[test]
fn test_e2e_selfhost_constraints_trait_where() {
    // Constraints.ls が存在することを検証
    let constraints_path = selfhost_source_path("Constraints.ls");
    assert!(
        constraints_path.exists(),
        "selfhost/src/Types/Constraints.ls が存在しない -- 制約解決モジュール未作成"
    );

    let constraints_content = std::fs::read_to_string(&constraints_path)
        .expect("selfhost/src/Types/Constraints.ls の読み込みに失敗");

    // モジュール宣言を検証
    assert!(
        constraints_content.contains("(module Types.Constraints)"),
        "selfhost/src/Types/Constraints.ls に namespaced module 宣言がない"
    );

    // trait registry 関連の関数が定義されていることを検証
    assert!(
        constraints_content.contains("(defn trait-registry")
            || constraints_content.contains("(defn make-trait-registry")
            || constraints_content.contains("(defn register-trait"),
        "selfhost/src/Types/Constraints.ls に trait registry 関数が未定義"
    );

    // impl registry 関連の関数が定義されていることを検証
    assert!(
        constraints_content.contains("(defn impl-registry")
            || constraints_content.contains("(defn make-impl-registry")
            || constraints_content.contains("(defn register-impl"),
        "selfhost/src/Types/Constraints.ls に impl registry 関数が未定義"
    );

    // constraint solver が定義されていることを検証
    assert!(
        constraints_content.contains("(defn solve-constraints")
            || constraints_content.contains("(defn resolve-constraint"),
        "selfhost/src/Types/Constraints.ls に constraint solver 関数が未定義"
    );
}
