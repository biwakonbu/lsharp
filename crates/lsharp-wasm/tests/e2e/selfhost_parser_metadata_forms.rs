use super::support::*;


/// TEST-SYNTAX-02l: parametric type / type-alias head を decl tag にパースできる
#[test]
fn test_e2e_selfhost_parser_parametric_type_heads() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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
    assert_eq!(lines[0], "1", "parametric type record は ast-recorddef であるべき");
    assert_eq!(lines[1], "1", "parametric type 名 hash は Pair であるべき");
    assert_eq!(lines[2], "1", "parametric type-alias は ast-typealias であるべき");
    assert_eq!(lines[3], "1", "parametric alias 名 hash は Callback であるべき");
}

/// TEST-SYNTAX-02m: annotation form を AST ノードにパースできる
#[test]
fn test_e2e_selfhost_parser_ann_form() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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

    assert!(lines.len() >= 3, "annotation parser 出力が不足: {:?}", lines);
    assert_eq!(lines[0], "1", "annotation は ast-ann であるべき");
    assert_eq!(lines[1], "1", "annotation inner は int literal であるべき");
    assert_eq!(lines[2], "42", "annotation inner の値が保持されるべき");
}

/// TEST-SYNTAX-02n: float literal を lexer/parser で扱える
#[test]
fn test_e2e_selfhost_parser_float_literal() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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

    assert!(lines.len() >= 13, "computation parser 出力が不足: {:?}", lines);
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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

    assert!(lines.len() >= 6, "typed defn parser 出力が不足: {:?}", lines);
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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

    assert!(lines.len() >= 9, "where defn parser 出力が不足: {:?}", lines);
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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

    assert!(lines.len() >= 6, "metadata defn parser 出力が不足: {:?}", lines);
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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

    assert!(lines.len() >= 7, "string metadata parser 出力が不足: {:?}", lines);
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
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(project_root.join("selfhost/Lexer.ls"))
        .expect("selfhost/Lexer.ls が読み込めない");
    let parser_ls =
        std::fs::read_to_string(project_root.join("selfhost/Parser.ls"))
            .expect("selfhost/Parser.ls が読み込めない");

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

    assert!(lines.len() >= 7, "params metadata parser 出力が不足: {:?}", lines);
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
/// selfhost/Hygiene.ls が存在し、gensym, scope-id, expansion-trace 関数を公開していることを検証。
/// 現状: Hygiene.ls 未作成 → FAIL
#[test]
fn test_e2e_selfhost_hygiene_gensym() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // Hygiene.ls が存在することを検証
    let hygiene_ls_path = project_root.join("selfhost/Hygiene.ls");
    assert!(
        hygiene_ls_path.exists(),
        "selfhost/Hygiene.ls が存在しない -- 衛生的マクロモジュール未作成"
    );

    let hygiene_content = std::fs::read_to_string(&hygiene_ls_path)
        .expect("selfhost/Hygiene.ls の読み込みに失敗");

    // 必須関数が定義されていることを検証
    assert!(
        hygiene_content.contains("(module Hygiene)"),
        "selfhost/Hygiene.ls に (module Hygiene) 宣言がない"
    );
    assert!(
        hygiene_content.contains("(defn gensym"),
        "selfhost/Hygiene.ls に gensym 関数が未定義"
    );
    assert!(
        hygiene_content.contains("(defn scope-id")
            || hygiene_content.contains("(defn make-scope-id"),
        "selfhost/Hygiene.ls に scope-id 関数が未定義"
    );
    assert!(
        hygiene_content.contains("(defn expansion-trace")
            || hygiene_content.contains("(defn make-expansion-trace"),
        "selfhost/Hygiene.ls に expansion-trace 関数が未定義"
    );
}

/// TEST-SYNTAX-05: Derive.ls expand-derives
///
/// selfhost/Derive.ls が存在し、expand-derives 関数がヘルパー decl を生成できることを検証。
/// 現状: Derive.ls 未作成 → FAIL
#[test]
fn test_e2e_selfhost_derive_expansion() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // Derive.ls が存在することを検証
    let derive_ls_path = project_root.join("selfhost/Derive.ls");
    assert!(
        derive_ls_path.exists(),
        "selfhost/Derive.ls が存在しない -- derive マクロモジュール未作成"
    );

    let derive_content = std::fs::read_to_string(&derive_ls_path)
        .expect("selfhost/Derive.ls の読み込みに失敗");

    // 必須関数が定義されていることを検証
    assert!(
        derive_content.contains("(module Derive)"),
        "selfhost/Derive.ls に (module Derive) 宣言がない"
    );
    assert!(
        derive_content.contains("(defn expand-derives")
            || derive_content.contains("(defn expand-derive"),
        "selfhost/Derive.ls に expand-derives 関数が未定義"
    );
}

/// TEST-SYNTAX-06: Syntax golden fixtures
///
/// tests/golden/syntax/ に tokens.json, ast.json, diagnostics.json の
/// golden fixture が存在し、内容が正しいことを検証。
/// 現状: golden fixture 未作成 → FAIL
#[test]
fn test_e2e_syntax_golden_fixtures() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

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
    let tokens_content = std::fs::read_to_string(&tokens_path)
        .expect("tokens.json の読み込みに失敗");
    let tokens: serde_json::Value = serde_json::from_str(&tokens_content)
        .expect("tokens.json が有効な JSON でない");
    assert!(
        tokens.get("cases").is_some(),
        "tokens.json に cases セクションがない"
    );
    let token_cases = tokens["cases"].as_array()
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
    let ast_content = std::fs::read_to_string(&ast_path)
        .expect("ast.json の読み込みに失敗");
    let ast: serde_json::Value = serde_json::from_str(&ast_content)
        .expect("ast.json が有効な JSON でない");
    assert!(
        ast.get("cases").is_some(),
        "ast.json に cases セクションがない"
    );
    let ast_cases = ast["cases"].as_array()
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
    let diag_content = std::fs::read_to_string(&diag_path)
        .expect("diagnostics.json の読み込みに失敗");
    let diag: serde_json::Value = serde_json::from_str(&diag_content)
        .expect("diagnostics.json が有効な JSON でない");
    assert!(
        diag.get("cases").is_some(),
        "diagnostics.json に cases セクションがない"
    );
    let diag_cases = diag["cases"].as_array()
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
/// selfhost/TypeInfer.ls に infer-pattern 関数があり、
/// match 式の型推論でコンストラクタパターンに対応していることを検証。
/// 現状: infer-pattern 関数未実装 → FAIL
#[test]
fn test_e2e_selfhost_match_inference() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // TypeInfer.ls を読み込み
    let type_infer_path = project_root.join("selfhost/TypeInfer.ls");
    assert!(
        type_infer_path.exists(),
        "selfhost/TypeInfer.ls が存在しない"
    );
    let type_infer_content = std::fs::read_to_string(&type_infer_path)
        .expect("selfhost/TypeInfer.ls の読み込みに失敗");

    // infer-pattern 関数が定義されていることを検証
    assert!(
        type_infer_content.contains("(defn infer-pattern"),
        "selfhost/TypeInfer.ls に infer-pattern 関数が未定義 -- \
         match 式のパターン型推論が未実装"
    );

    // infer-pattern がコンストラクタパターン対応していることを検証
    assert!(
        type_infer_content.contains("constructor-pattern")
            || type_infer_content.contains("ctor-pattern")
            || type_infer_content.contains("tag-pattern"),
        "selfhost/TypeInfer.ls の infer-pattern が \
         コンストラクタパターンに対応していない"
    );
}

/// TEST-TYPE-04: Constraints.ls trait/where/constraint solving
///
/// selfhost/Constraints.ls が存在し、trait registry, impl registry,
/// constraint solver を公開していることを検証。
/// 現状: Constraints.ls 未作成 → FAIL
#[test]
fn test_e2e_selfhost_constraints_trait_where() {
    let project_root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // Constraints.ls が存在することを検証
    let constraints_path = project_root.join("selfhost/Constraints.ls");
    assert!(
        constraints_path.exists(),
        "selfhost/Constraints.ls が存在しない -- 制約解決モジュール未作成"
    );

    let constraints_content = std::fs::read_to_string(&constraints_path)
        .expect("selfhost/Constraints.ls の読み込みに失敗");

    // モジュール宣言を検証
    assert!(
        constraints_content.contains("(module Constraints)"),
        "selfhost/Constraints.ls に (module Constraints) 宣言がない"
    );

    // trait registry 関連の関数が定義されていることを検証
    assert!(
        constraints_content.contains("(defn trait-registry")
            || constraints_content.contains("(defn make-trait-registry")
            || constraints_content.contains("(defn register-trait"),
        "selfhost/Constraints.ls に trait registry 関数が未定義"
    );

    // impl registry 関連の関数が定義されていることを検証
    assert!(
        constraints_content.contains("(defn impl-registry")
            || constraints_content.contains("(defn make-impl-registry")
            || constraints_content.contains("(defn register-impl"),
        "selfhost/Constraints.ls に impl registry 関数が未定義"
    );

    // constraint solver が定義されていることを検証
    assert!(
        constraints_content.contains("(defn solve-constraints")
            || constraints_content.contains("(defn resolve-constraint"),
        "selfhost/Constraints.ls に constraint solver 関数が未定義"
    );
}
