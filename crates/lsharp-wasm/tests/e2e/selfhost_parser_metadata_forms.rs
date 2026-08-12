use super::support::*;

fn run_parser_runtime(harness: &str) -> String {
    compile_and_run(&format!(
        "{}\n{}",
        selfhost_parser_runtime_bundle(),
        harness
    ))
}

/// TEST-SYNTAX-02l: parametric type / type-alias head を decl tag にパースできる
#[test]
fn test_e2e_selfhost_parser_parametric_type_heads() {
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

    let output = run_parser_runtime(harness);
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

/// TEST-SYNTAX-02l0: record 宣言は field 名と raw type expression を保持する
///
/// selfhost Parser が `(type Point (record (: x Int) (: y (Ref String))))`
/// を record definition node として読み、field payload を後段の型推論へ渡せることを検証する。
#[test]
fn test_e2e_selfhost_parser_record_decl_retains_fields_and_raw_type_exprs() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(type Point (record (: x Int) (: y (Ref String))))") 0)]
    (do
      (print (if (= (vector-get node 0) (ast-recorddef)) 1 0))
      (print (if (= (vector-get node 1) (name-hash "Point" 0 5)) 1 0))
      (if (> (vector-length node) 2)
        (let [fields (vector-get node 2)
              x-type (vector-get fields 2)
              y-type (vector-get fields 5)
              y-arg (vector-get y-type 3)]
          (do
            (print (if (= (vector-length node) 3) 1 0))
            (print (if (= (vector-length fields) 6) 1 0))
            (print (if (= (vector-get fields 0) (name-hash "x" 0 1)) 1 0))
            (print (if (= (vector-get fields 1) (name-hash "Point.x" 0 7)) 1 0))
            (print (if (= (vector-get x-type 0) (ast-type-named)) 1 0))
            (print (if (= (vector-get x-type 1) (name-hash "Int" 0 3)) 1 0))
            (print (if (= (vector-get fields 3) (name-hash "y" 0 1)) 1 0))
            (print (if (= (vector-get fields 4) (name-hash "Point.y" 0 7)) 1 0))
            (print (if (= (vector-get y-type 0) (ast-type-app)) 1 0))
            (print (if (= (vector-get y-type 1) (name-hash "Ref" 0 3)) 1 0))
            (print (if (= (vector-get y-type 2) 1) 1 0))
            (print (if (= (vector-get y-arg 0) (ast-type-named)) 1 0))
            (print (if (= (vector-get y-arg 1) (name-hash "String" 0 6)) 1 0))
            0))
        (do
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          0)))))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1"
        ],
        "record 宣言は field 名と nested raw type expression を保持するべき"
    );
}

/// TEST-SYNTAX-02l0a: parametric record 宣言は parameter 名と field 型式を両方保持する
#[test]
fn test_e2e_selfhost_parser_parametric_record_decl_retains_params_and_fields() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(type (Pair a b) (record (: fst a) (: snd b)))") 0)]
    (do
      (print (if (= (vector-get node 0) (ast-recorddef)) 1 0))
      (print (if (= (vector-get node 1) (name-hash "Pair" 0 4)) 1 0))
      (if (> (vector-length node) 3)
        (let [params (vector-get node 2)
              fields (vector-get node 3)
              fst-type (vector-get fields 2)
              snd-type (vector-get fields 5)]
          (do
            (print (if (= (vector-length node) 4) 1 0))
            (print (if (= (vector-length params) 2) 1 0))
            (print (if (= (vector-get params 0) (name-hash "a" 0 1)) 1 0))
            (print (if (= (vector-get params 1) (name-hash "b" 0 1)) 1 0))
            (print (if (= (vector-length fields) 6) 1 0))
            (print (if (= (vector-get fields 0) (name-hash "fst" 0 3)) 1 0))
            (print (if (= (vector-get fields 1) (name-hash "Pair.fst" 0 8)) 1 0))
            (print (if (= (vector-get fst-type 0) (ast-type-var)) 1 0))
            (print (if (= (vector-get fields 3) (name-hash "snd" 0 3)) 1 0))
            (print (if (= (vector-get fields 4) (name-hash "Pair.snd" 0 8)) 1 0))
            (print (if (= (vector-get snd-type 0) (ast-type-var)) 1 0))
            0))
        (do
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          (print 0)
          0)))))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1", "1"
        ],
        "parametric record 宣言は parameter vector と field 型式を保持するべき"
    );
}

/// TEST-SYNTAX-02l1: parametric type-alias は parameter と raw target を保持する
#[test]
fn test_e2e_selfhost_parser_parametric_type_alias_retains_params_and_target() {
    let harness = r#"
(defn main []
  (let [alias-node (vector-get (parse-program "(type-alias (Callback a b) (-> a b))") 0)
        params (vector-get alias-node 2)
        target (vector-get alias-node 3)]
    (do
      (print (vector-length alias-node))
      (print (vector-length params))
      (print (if (= (vector-get params 0) (name-hash "a" 0 1)) 1 0))
      (print (if (= (vector-get params 1) (name-hash "b" 0 1)) 1 0))
      (print (if (= (vector-get target 0) (ast-type-fun)) 1 0))
      (print (vector-get target 1))
      (print (if (= (vector-get (vector-get target 2) 0) (ast-type-var)) 1 0))
      (print (if (= (vector-get (vector-get target 3) 0) (ast-type-var)) 1 0))
      (print (vector-get alias-node 4))
      (print (vector-get alias-node 5))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["6", "2", "1", "1", "1", "1", "1", "1", "0", "36"],
        "parametric type-alias は parameter vector、raw target、宣言 span を保持するべき"
    );
}

/// TEST-SYNTAX-02l2: closed type-alias は target の raw type expression を保持する
#[test]
fn test_e2e_selfhost_parser_type_alias_retains_closed_target_expr() {
    let harness = r#"
(defn type-alias-target-or-zero [node]
  (if (> (vector-length node) 3)
    (let [candidate (vector-get node 2)]
      (if (= (vector-length candidate) 0)
        (vector-get node 3)
        candidate))
    (if (> (vector-length node) 2)
      (vector-get node 2)
      0)))

(defn main []
  (let [alias-node (vector-get (parse-program "(type-alias Str String)") 0)
        target (type-alias-target-or-zero alias-node)]
    (do
      (print (if (= (vector-get alias-node 0) (ast-typealias)) 1 0))
      (print (if (= (vector-get alias-node 1) (name-hash "Str" 0 3)) 1 0))
      (print (if (= target 0) 0 (if (= (vector-get target 0) (ast-type-named)) 1 0)))
      (print (if (= target 0) 0 (if (= (vector-get target 1) (name-hash "String" 0 6)) 1 0)))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "1", "1", "1"],
        "closed type-alias は name と target raw type expression を保持するべき"
    );
}

/// TEST-SYNTAX-02m: annotation form を AST ノードにパースできる
#[test]
fn test_e2e_selfhost_parser_ann_form() {
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

    let output = run_parser_runtime(harness);
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

/// TEST-SYNTAX-02m2: annotation form は raw type expression を保持する
#[test]
fn test_e2e_selfhost_parser_ann_form_retains_type_expr() {
    let harness = r#"
(defn ann-type-tag-or-zero [node]
  (if (> (vector-length node) 2)
    (let [ty (vector-get node 2)]
      (if (= ty 0) 0 (vector-get ty 0)))
    0))

(defn ann-type-name-or-zero [node]
  (if (> (vector-length node) 2)
    (let [ty (vector-get node 2)]
      (if (= ty 0) 0 (vector-get ty 1)))
    0))

(defn main []
  (let [node (vector-get (parse-program "(: 42 Int)") 0)]
    (do
      (print (if (= (vector-get node 0) (ast-ann)) 1 0))
      (print (vector-length node))
      (print (ann-type-tag-or-zero node))
      (print (ann-type-name-or-zero node))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "3", "60", "73679"],
        "annotation parser は TypeNamed の raw payload を保持するべき"
    );
}

/// TEST-SYNTAX-02m3: annotation form は applied / function raw type expression を保持する
#[test]
fn test_e2e_selfhost_parser_ann_form_retains_type_app_and_fun_expr() {
    let harness = r#"
(defn main []
  (let [app-node (vector-get (parse-program "(: 42 (Ref (Vector Int)))") 0)
        app-type (vector-get app-node 2)
        app-arg (vector-get app-type 3)
        app-inner (vector-get app-arg 3)
        fun-node (vector-get (parse-program "(: 42 (-> Int String Bool))") 0)
        fun-type (vector-get fun-node 2)
        fun-param1 (vector-get fun-type 2)
        fun-param2 (vector-get fun-type 3)
        fun-return (vector-get fun-type 4)]
    (do
      (print (vector-get app-type 0))
      (print (vector-get app-type 1))
      (print (vector-get app-type 2))
      (print (vector-get app-arg 0))
      (print (vector-get app-arg 1))
      (print (vector-get app-arg 2))
      (print (vector-get app-inner 0))
      (print (vector-get app-inner 1))
      (print (vector-get fun-type 0))
      (print (vector-get fun-type 1))
      (print (vector-get fun-param1 0))
      (print (vector-get fun-param1 1))
      (print (vector-get fun-param2 0))
      (print (vector-get fun-param2 1))
      (print (vector-get fun-return 0))
      (print (vector-get fun-return 1))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "61",
            "82035",
            "1",
            "61",
            "2558446947",
            "1",
            "60",
            "73679",
            "62",
            "2",
            "60",
            "73679",
            "60",
            "2486848561",
            "60",
            "2076426",
        ],
        "annotation parser は TypeApp / TypeFun の raw payload を保持するべき"
    );
}

/// TEST-SYNTAX-02m4: lower-case type variable を named type と区別して保持する
#[test]
fn test_e2e_selfhost_parser_ann_form_retains_type_var_expr() {
    let harness = r#"
(defn main []
  (let [var-node (vector-get (parse-program "(: 42 a)") 0)
        var-type (vector-get var-node 2)
        app-node (vector-get (parse-program "(: 42 (Ref a))") 0)
        app-type (vector-get app-node 2)
        app-arg (vector-get app-type 3)]
    (do
      (print (vector-get var-type 0))
      (print (vector-get var-type 1))
      (print (vector-get app-type 0))
      (print (vector-get app-type 1))
      (print (vector-get app-type 2))
      (print (vector-get app-arg 0))
      (print (vector-get app-arg 1))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["63", "97", "61", "82035", "1", "63", "97"],
        "lower-case type variable は TypeNamed と区別して raw payload を保持するべき"
    );
}

/// TEST-SYNTAX-02n: float literal を lexer/parser で扱える
#[test]
fn test_e2e_selfhost_parser_float_literal() {
    let harness = r#"
(defn main []
  (let [src "3.14"
        tokens (tokenize-with-spans src)
        node (vector-get (parse-program src) 0)]
    (do
      (print (if (= (vector-get tokens 0) (tok-float)) 1 0))
      (print (if (= (vector-get node 0) (ast-lit-float)) 1 0))
      (print (vector-get node 1))
      (print (vector-get node 2))
      (print (if (string-eq (substring src (vector-get node 1) (vector-get node 2)) "3.14") 1 0))
      0)))
"#;

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

/// TEST-SYNTAX-02p: defn の annotated param / return type を body の後ろに保持できる
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn add [(: x Int) (: y Int)] : Int (+ x y))") 0)
        body (vector-get node 5)
        signature (vector-get node 6)
        param1-type (vector-get signature 2)
        param2-type (vector-get signature 3)
        return-type (vector-get signature 4)]
    (do
      (print (if (= (vector-get node 0) 20) 1 0))
      (print (if (= (vector-get node 1) (name-hash "add" 0 3)) 1 0))
      (print (vector-get node 2))
      (print (if (= (vector-get node 3) (name-hash "x" 0 1)) 1 0))
      (print (if (= (vector-get node 4) (name-hash "y" 0 1)) 1 0))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (if (= (vector-get signature 0) 65) 1 0))
      (print (vector-get signature 1))
      (print (if (= (vector-get param1-type 0) (ast-type-named)) 1 0))
      (print (vector-get param1-type 1))
      (print (if (= (vector-get param2-type 0) (ast-type-named)) 1 0))
      (print (vector-get param2-type 1))
      (print (if (= (vector-get return-type 0) (ast-type-named)) 1 0))
      (print (vector-get return-type 1))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 14,
        "typed defn parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "defn tag が一致すべき");
    assert_eq!(lines[1], "1", "関数名 hash が一致すべき");
    assert_eq!(lines[2], "2", "param count は 2 であるべき");
    assert_eq!(lines[3], "1", "param1 hash は x であるべき");
    assert_eq!(lines[4], "1", "param2 hash は y であるべき");
    assert_eq!(lines[5], "1", "body は apply ノードであるべき");
    assert_eq!(lines[6], "1", "signature tag は 65 であるべき");
    assert_eq!(lines[7], "2", "signature param count は 2 であるべき");
    assert_eq!(lines[8], "1", "param1 type は named type であるべき");
    assert_eq!(lines[9], "73679", "param1 type は Int であるべき");
    assert_eq!(lines[10], "1", "param2 type は named type であるべき");
    assert_eq!(lines[11], "73679", "param2 type は Int であるべき");
    assert_eq!(lines[12], "1", "return type は named type であるべき");
    assert_eq!(lines[13], "73679", "return type は Int であるべき");
}

/// TEST-SYNTAX-02p2: typed defn signature は nested type expression を保持する
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature_retains_nested_type_expr() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn transform [(: ref (Ref (Vector Int)))] : (-> Int String Bool) ref)") 0)
        signature (vector-get node 5)
        param-type (vector-get signature 2)
        param-arg (vector-get param-type 3)
        param-inner (vector-get param-arg 3)
        return-type (vector-get signature 3)
        return-param1 (vector-get return-type 2)
        return-param2 (vector-get return-type 3)
        return-ret (vector-get return-type 4)]
    (do
      (print (vector-get signature 0))
      (print (vector-get signature 1))
      (print (vector-get param-type 0))
      (print (vector-get param-type 1))
      (print (vector-get param-type 2))
      (print (vector-get param-arg 0))
      (print (vector-get param-arg 1))
      (print (vector-get param-arg 2))
      (print (vector-get param-inner 0))
      (print (vector-get param-inner 1))
      (print (vector-get return-type 0))
      (print (vector-get return-type 1))
      (print (vector-get return-param1 0))
      (print (vector-get return-param1 1))
      (print (vector-get return-param2 0))
      (print (vector-get return-param2 1))
      (print (vector-get return-ret 0))
      (print (vector-get return-ret 1))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "65",
            "1",
            "61",
            "82035",
            "1",
            "61",
            "2558446947",
            "1",
            "60",
            "73679",
            "62",
            "2",
            "60",
            "73679",
            "60",
            "2486848561",
            "60",
            "2076426",
        ],
        "typed defn parser は nested TypeApp / TypeFun を signature に保持するべき"
    );
}

/// TEST-SYNTAX-02q: defn の :where clause を最小 payload のままスキップできる
#[test]
fn test_e2e_selfhost_parser_defn_where_clause() {
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

    let output = run_parser_runtime(harness);
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

/// TEST-SYNTAX-02u: defn の :doc / :example metadata を trailing metadata vector として保持できる
#[test]
fn test_e2e_selfhost_parser_defn_preserves_doc_example_metadata() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn add [x y] :doc \"addition\" :example [(add 1 2)] (+ x y))") 0)
        body (vector-get node 5)
        meta (vector-get node 6)
        doc (vector-get meta 0)
        example (vector-get meta 1)
        params (vector-get meta 2)
        returns (vector-get meta 3)]
    (do
      (print (vector-length node))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (vector-length meta))
      (print (vector-length params))
      (print (string-length returns))
      (print-string doc)
      (print-string "\n")
      (print-string example)
      (print-string "\n")
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 7,
        "metadata preserve parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "7",
        "defn node は trailing metadata 付き 7 要素であるべき"
    );
    assert_eq!(lines[1], "1", "body は apply ノードのまま保持されるべき");
    assert_eq!(
        lines[2], "6",
        "metadata entry は legacy 5 スロットと ordered forms の 6 件であるべき"
    );
    assert_eq!(lines[3], "0", ":params なしでは空 vector を保持するべき");
    assert_eq!(lines[4], "0", ":returns なしでは空文字列を保持するべき");
    assert_eq!(lines[5], "addition", ":doc string が保持されるべき");
    assert_eq!(lines[6], "(add 1 2)", ":example string が保持されるべき");
}

/// TEST-SYNTAX-02v: defn の :params / :returns metadata を trailing metadata vector として保持できる
#[test]
fn test_e2e_selfhost_parser_defn_preserves_params_returns_metadata() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn add [x y] :params [(x \"left\") (y \"right\")] :returns \"sum\" (+ x y))") 0)
        body (vector-get node 5)
        meta (vector-get node 6)
        params (vector-get meta 2)
        returns (vector-get meta 3)
        param0 (vector-get params 0)
        param1 (vector-get params 1)]
    (do
      (print (vector-length node))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (vector-length meta))
      (print (vector-length params))
      (print (if (= (vector-get param0 0) (name-hash "x" 0 1)) 1 0))
      (print-string (vector-get param0 1))
      (print-string "\n")
      (print (if (= (vector-get param1 0) (name-hash "y" 0 1)) 1 0))
      (print-string (vector-get param1 1))
      (print-string "\n")
      (print-string returns)
      (print-string "\n")
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 9,
        "params/returns metadata parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "7",
        "defn node は trailing metadata 付き 7 要素であるべき"
    );
    assert_eq!(lines[1], "1", "body は apply ノードのまま保持されるべき");
    assert_eq!(
        lines[2], "6",
        "metadata entry は legacy 5 スロットと ordered forms の 6 件であるべき"
    );
    assert_eq!(lines[3], "2", "params metadata は 2 件であるべき");
    assert_eq!(lines[4], "1", "1件目 param 名 hash は x であるべき");
    assert_eq!(lines[5], "left", "1件目 param doc が保持されるべき");
    assert_eq!(lines[6], "1", "2件目 param 名 hash は y であるべき");
    assert_eq!(lines[7], "right", "2件目 param doc が保持されるべき");
    assert_eq!(lines[8], "sum", ":returns string が保持されるべき");
}

/// EC-M1-02: defn の legacy :invariant metadata を AST として保持できる
#[test]
fn test_e2e_selfhost_parser_defn_preserves_invariant_metadata() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn succ [x] :invariant (= result (+ x 1)) (+ x 1))") 0)
        body (vector-get node 4)
        meta (vector-get node 5)
        invariant (vector-get meta 4)]
    (do
      (print (vector-length node))
      (print (vector-length meta))
      (print (if (= (vector-get body 0) (ast-apply)) 1 0))
      (print (if (= (vector-get invariant 0) (ast-apply)) 1 0))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["6", "6", "1", "1"],
        "selfhost parser は legacy :invariant と defn body を同時に保持するべき"
    );
}

/// EC-M1-02: parser-owned ordered metadata forms が directive 順を保持する
#[test]
fn test_e2e_selfhost_parser_defn_preserves_ordered_metadata_forms() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn succ [x] :example [(succ 0)] :invariant (= result (+ x 1)) :example [(succ 1)] (+ x 1))") 0)
        meta (vector-get node 5)
        forms (vector-get meta 5)
        form0 (vector-get forms 0)
        form1 (vector-get forms 1)
        form2 (vector-get forms 2)
        invariant (vector-get form1 1)]
    (do
      (print (vector-length meta))
      (print (vector-length forms))
      (print (vector-get form0 0))
      (print (vector-get form1 0))
      (print (vector-get form2 0))
      (print-string (vector-get form0 1))
      (print-string "\n")
      (print (if (= (vector-get invariant 0) (ast-apply)) 1 0))
      (print-string (vector-get form2 1))
      (print-string "\n")
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["6", "3", "1", "2", "1", "(succ 0)", "1", "(succ 1)"],
        "parser-owned ordered metadata は directive 順序と payload を保持するべき"
    );
}

/// EC-M2-01: source intent node / edge metadata を directive 順と wire payload のまま保持する
#[test]
fn test_e2e_selfhost_parser_preserves_source_intent_metadata_forms() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn cancel [] :intent \"intent:checkout/safe-cancel\" \"Users can cancel an order\" :claim \"claim:checkout/cancel-rejects-shipped\" \"The API rejects shipped orders\" :motivates \"intent:checkout/safe-cancel\" \"claim:checkout/cancel-rejects-shipped\" true)") 0)
        node-len (vector-length node)
        last (vector-get node (- node-len 1))]
    (do
      (print node-len)
      (if (= (vector-length last) 6)
        (let [forms (vector-get last 5)
              intent (vector-get forms 0)
              claim (vector-get forms 1)
              motivates (vector-get forms 2)]
          (do
            (print (vector-length forms))
            (print (vector-get intent 0))
            (print-string (vector-get (vector-get intent 1) 0))
            (print-string "\n")
            (print-string (vector-get (vector-get intent 1) 1))
            (print-string "\n")
            (print (vector-get claim 0))
            (print-string (vector-get (vector-get claim 1) 0))
            (print-string "\n")
            (print (vector-get motivates 0))
            (print-string (vector-get (vector-get motivates 1) 0))
            (print-string "\n")
            (print-string (vector-get (vector-get motivates 1) 1))
            (print-string "\n")
            0))
        (do
          (print 0)
          0)))))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "5",
            "3",
            "6",
            "intent:checkout/safe-cancel",
            "Users can cancel an order",
            "7",
            "claim:checkout/cancel-rejects-shipped",
            "10",
            "intent:checkout/safe-cancel",
            "claim:checkout/cancel-rejects-shipped",
        ],
        "selfhost parser は M2 source metadata を directive 順と payload のまま保持するべき"
    );
}

/// EC-M3-04: review attestation は positional payload ではなく named fields を保持する
/// parser-owned form として Rust source syntax と同じ field order/value を渡す。
#[test]
fn test_e2e_selfhost_parser_preserves_review_attestation_named_fields() {
    let harness = r#"
(defn main []
  (let [node (vector-get (parse-program "(defn review [] :review-attestation :review-id \"review:checkout/reviewer-001\" :subject-digest \"sha256:subject-001\" :source-commit \"0123456789abcdef\" :provenance-digest \"sha256:review-001\" :provider \"github\" :key-id \"org/reviews-2026\" :algorithm \"ed25519\" :signature \"AAECAw\" :issued-at \"2026-08-01T00:00:00Z\" :expires-at \"2026-09-01T00:00:00Z\" :sequence 3 true)") 0)
        meta (vector-get node (- (vector-length node) 1))
        forms (vector-get meta 5)
        form (vector-get forms 0)
        payload (vector-get form 1)]
    (do
      (print (vector-length forms))
      (print (vector-get form 0))
      (print (vector-length payload))
      (print-string (vector-get payload 0))
      (print-string "\n")
      (print-string (vector-get payload 1))
      (print-string "\n")
      (print-string (vector-get payload 2))
      (print-string "\n")
      (print-string (vector-get payload 3))
      (print-string "\n")
      (print-string (vector-get payload 4))
      (print-string "\n")
      (print-string (vector-get payload 5))
      (print-string "\n")
      (print-string (vector-get payload 6))
      (print-string "\n")
      (print-string (vector-get payload 7))
      (print-string "\n")
      (print-string (vector-get payload 8))
      (print-string "\n")
      (print-string (vector-get payload 9))
      (print-string "\n")
      (print (vector-get payload 10))
      (print (if (< (vector-get form 2) (vector-get form 3)) 1 0))
      0)))
"#;

    let output = run_parser_runtime(harness);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "20",
            "11",
            "review:checkout/reviewer-001",
            "sha256:subject-001",
            "0123456789abcdef",
            "sha256:review-001",
            "github",
            "org/reviews-2026",
            "ed25519",
            "AAECAw",
            "2026-08-01T00:00:00Z",
            "2026-09-01T00:00:00Z",
            "3",
            "1",
        ],
        "selfhost parser は review attestation の named fields と span を保持するべき"
    );
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
        "canonical Hygiene.ls が存在しない -- 衛生的マクロモジュール未作成"
    );

    let hygiene_content =
        std::fs::read_to_string(&hygiene_ls_path).expect("canonical Hygiene.ls の読み込みに失敗");

    // 必須関数が定義されていることを検証
    assert!(
        hygiene_content.contains("(module Syntax.Hygiene)"),
        "canonical Hygiene.ls に (module Syntax.Hygiene) 宣言がない"
    );
    assert!(
        hygiene_content.contains("(defn gensym"),
        "canonical Hygiene.ls に gensym 関数が未定義"
    );
    assert!(
        hygiene_content.contains("(defn scope-id")
            || hygiene_content.contains("(defn make-scope-id"),
        "canonical Hygiene.ls に scope-id 関数が未定義"
    );
    assert!(
        hygiene_content.contains("(defn expansion-trace")
            || hygiene_content.contains("(defn make-expansion-trace"),
        "canonical Hygiene.ls に expansion-trace 関数が未定義"
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
        "canonical Derive.ls が存在しない -- derive マクロモジュール未作成"
    );

    let derive_content =
        std::fs::read_to_string(&derive_ls_path).expect("canonical Derive.ls の読み込みに失敗");

    // 必須関数が定義されていることを検証
    assert!(
        derive_content.contains("(module Syntax.Derive)"),
        "canonical Derive.ls に (module Syntax.Derive) 宣言がない"
    );
    assert!(
        derive_content.contains("(defn expand-derives")
            || derive_content.contains("(defn expand-derive"),
        "canonical Derive.ls に expand-derives 関数が未定義"
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
/// selfhost/src/Types/TypeInfer.ls に infer-pattern 関数があり、
/// match 式の型推論でコンストラクタパターンに対応していることを検証。
/// 現状: infer-pattern 関数未実装 → FAIL
#[test]
fn test_e2e_selfhost_match_inference() {
    // TypeInfer.ls を読み込み
    let type_infer_path = selfhost_source_path("TypeInfer.ls");
    assert!(
        type_infer_path.exists(),
        "canonical TypeInfer.ls が存在しない"
    );
    let type_infer_content =
        std::fs::read_to_string(&type_infer_path).expect("canonical TypeInfer.ls の読み込みに失敗");

    // infer-pattern 関数が定義されていることを検証
    assert!(
        type_infer_content.contains("(defn infer-pattern"),
        "canonical TypeInfer.ls に infer-pattern 関数が未定義 -- \
         match 式のパターン型推論が未実装"
    );

    // infer-pattern がコンストラクタパターン対応していることを検証
    assert!(
        type_infer_content.contains("constructor-pattern")
            || type_infer_content.contains("ctor-pattern")
            || type_infer_content.contains("tag-pattern"),
        "canonical TypeInfer.ls の infer-pattern が \
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
        "canonical Constraints.ls が存在しない -- 制約解決モジュール未作成"
    );

    let constraints_content = std::fs::read_to_string(&constraints_path)
        .expect("canonical Constraints.ls の読み込みに失敗");

    // モジュール宣言を検証
    assert!(
        constraints_content.contains("(module Types.Constraints)"),
        "canonical Constraints.ls に (module Types.Constraints) 宣言がない"
    );

    // trait registry 関連の関数が定義されていることを検証
    assert!(
        constraints_content.contains("(defn trait-registry")
            || constraints_content.contains("(defn make-trait-registry")
            || constraints_content.contains("(defn register-trait"),
        "canonical Constraints.ls に trait registry 関数が未定義"
    );

    // impl registry 関連の関数が定義されていることを検証
    assert!(
        constraints_content.contains("(defn impl-registry")
            || constraints_content.contains("(defn make-impl-registry")
            || constraints_content.contains("(defn register-impl"),
        "canonical Constraints.ls に impl registry 関数が未定義"
    );

    // constraint solver が定義されていることを検証
    assert!(
        constraints_content.contains("(defn solve-constraints")
            || constraints_content.contains("(defn resolve-constraint"),
        "canonical Constraints.ls に constraint solver 関数が未定義"
    );
}
