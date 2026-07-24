use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

fn run_parser_macroexpand_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}\n{}",
        selfhost_parser_runtime_bundle(),
        selfhost_module("MacroExpand.ls"),
        harness
    ))
}

/// TEST-SYNTAX-02p: 深い let 連鎖でも parse-program がトラップしない
#[test]
fn test_e2e_selfhost_parser_deep_let_chain() {
    let mut nested_expr = "0".to_string();
    for i in (0..512).rev() {
        nested_expr = format!("(let [v{i:04} {i}] {nested_expr})");
    }
    let stage2_src = format!("(defn main [] {nested_expr})");
    let harness = format!(
        concat!(
            "(defn main []\n",
            "  (let [program (parse-program \"{src}\")]\n",
            "    (do (print (vector-length program)) 0)))\n",
        ),
        src = stage2_src,
    );

    let output = run_parser_runtime(&harness);
    assert_eq!(output.trim(), "1", "深い let 連鎖でも 1 decl を返すべき");
}

/// TEST-SYNTAX-02q: `let` binding list が EOF に達しても parser が無限再帰しない
#[test]
fn test_e2e_selfhost_parser_malformed_let_binding_list_reaches_eof() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (let [x 1 y 2")]
    (do
      (print (vector-length program))
      0)))
"#;

    let output = run_parser_runtime(harness);
    assert_eq!(
        output.trim(),
        "1",
        "malformed let binding list でも EOF で停止して decl を返すべき"
    );
}

/// TEST-SYNTAX-02r: `do` body が EOF に達しても parser が無限再帰しない
#[test]
fn test_e2e_selfhost_parser_malformed_do_body_reaches_eof() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn main [] (do 1 2")]
    (do
      (print (vector-length program))
      0)))
"#;

    let output = run_parser_runtime(harness);
    assert_eq!(
        output.trim(),
        "1",
        "malformed do body でも EOF で停止して decl を返すべき"
    );
}

/// TEST-SYNTAX-02g: defmacro が canonical tag でパースされ collect-macros に拾われる
///
/// selfhost Parser が `(defmacro ...)` を ast-defmacro として返し、
/// MacroExpand.collect-macros がそのノードを収集できることを検証する。
#[test]
fn test_e2e_selfhost_parser_defmacro_collect() {
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

    let output = run_parser_macroexpand_runtime(harness);
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

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 3, "private parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "private は ast-private であるべき");
    assert_eq!(lines[1], "1", "private の内側は ast-defn であるべき");
    assert_eq!(lines[2], "1", "inner defn 名ハッシュが一致すべき");
}

/// TEST-SYNTAX-02i: record update を AST ノードにパースできる
#[test]
fn test_e2e_selfhost_parser_record_update() {
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "()") 0)]
    (do
      (print (if (= (vector-get node 0) (ast-lit-unit)) 1 0))
      (print (vector-length node))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(lines.len() >= 2, "unit parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "unit literal は ast-lit-unit であるべき");
    assert_eq!(lines[1], "1", "unit literal node length は 1 であるべき");
}

/// TEST-SYNTAX-02k: if 式を明示的に ast-if としてパースできる
#[test]
fn test_e2e_selfhost_parser_if_expr() {
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match 1 [rest 3])") 0)
        pat (vector-get node 3)]
    (do
      (print (if (= (vector-get pat 0) (ast-pat-var)) 1 0))
      (print (if (= (vector-get pat 1) (name-hash "rest" 0 4)) 1 0))
      0)))
"#;

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

/// TEST-SYNTAX-02l4a: record pattern は record type 名の hash を保持する
#[test]
fn test_e2e_selfhost_parser_match_record_pattern_retains_type_hash() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match value [{Point x rest} rest])") 0)
        pat (vector-get node 3)
        type-slot (+ 2 (* (vector-get pat 1) 2))]
    (do
      (print (if (= (vector-get pat type-slot) (name-hash "Point" 0 5)) 1 0))
      0)))
"#;

    let output = run_parser_runtime(harness);
    assert_eq!(
        output.trim(),
        "1",
        "record pattern は record type name hash を保持すべき"
    );
}

/// TEST-SYNTAX-02l4c: qualified record pattern は marker 用の raw suffix hash も保持する
#[test]
fn test_e2e_selfhost_parser_match_qualified_record_pattern_retains_raw_type_hash() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(match value [{S.Point x rest} rest])") 0)
        pat (vector-get node 3)
        type-slot (+ 2 (* (vector-get pat 1) 2))
        qualified-hash (ast-qualified-name-hash (name-hash "S" 0 1) (name-hash "Point" 0 5))]
    (do
      (print (if (= (vector-get pat type-slot) qualified-hash) 1 0))
      (print (if (and (> (vector-length pat) (+ type-slot 1)) (= (vector-get pat (+ type-slot 1)) (name-hash "Point" 0 5))) 1 0))
      0)))
"#;

    let output = run_parser_runtime(harness);
    assert_eq!(
        output.trim(),
        "1\n1",
        "qualified record pattern は full hash と raw suffix hash を保持すべき"
    );
}

/// TEST-SYNTAX-02l4d: qualified record literal は marker 用の raw suffix hash も保持する
#[test]
fn test_e2e_selfhost_parser_qualified_record_literal_retains_raw_type_hash() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "{S.Point x 10 y 20}") 0)
        field-count (vector-get node 2)
        qualified-slot (+ 3 (* field-count 2))]
    (do
      (print (if (= (vector-get node 1) (ast-qualified-name-hash (name-hash "S" 0 1) (name-hash "Point" 0 5))) 1 0))
      (print (if (= (vector-get node qualified-slot) 1) 1 0))
      (print (if (and (> (vector-length node) (+ qualified-slot 1)) (= (vector-get node (+ qualified-slot 1)) (name-hash "Point" 0 5))) 1 0))
      0)))
"#;

    let output = run_parser_runtime(harness);
    assert_eq!(
        output.trim(),
        "1\n1\n1",
        "qualified record literal は full hash、qualified flag、raw suffix hash を保持すべき"
    );
}

/// TEST-SYNTAX-02l4b: GADT variant の return type を AST に保持できる
#[test]
fn test_e2e_selfhost_parser_gadt_variant_retains_return_type() {
    let harness = r#"
(defn main []
  (let [decl (vector-get (parse-program "(type (Expr a) (: (IntLit Int) (Expr Int)) (: (BoolLit Bool) (Expr Bool)))") 0)
        variants (vector-get decl 3)
        int-variant (vector-get variants 0)
        return-type (vector-get int-variant 2)]
    (do
      (print (vector-length int-variant))
      (print (if (= (vector-get return-type 0) (ast-type-app)) 1 0))
      (print (if (= (vector-get return-type 1) (name-hash "Expr" 0 4)) 1 0))
      (print (vector-get return-type 2))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["3", "1", "1", "1"],
        "GADT variant の return type は raw TypeExpr として保持されるべき"
    );
}

/// TEST-SYNTAX-02l5: match の int/bool literal pattern を canonical tag としてパースできる
#[test]
fn test_e2e_selfhost_parser_match_literal_pattern_tag() {
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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
