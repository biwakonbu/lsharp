/// parser-to-compiler bundle: source compile は if の control IR と outer add を保持する
#[test]
fn test_e2e_selfhost_source_compile_preserves_if_add_ir_order() {
    let harness = r#"
(defn print-ir-loop [ir idx len]
  (if (>= idx len)
    0
    (let [instr (vector-get ir idx)]
      (do
        (print (vector-get instr 0))
        (print (vector-get instr 1))
        (print-ir-loop ir (+ idx 1) len)))))

(defn main []
  (let [source "(defn main [] (+ 7 (if (= 0 0) 42 9)))"
        program (parse-program source)
        pair (compile-program-functions-with-source source program)
        functions (vector-get pair 1)
        main-fn (vector-get functions 0)
        ir (vector-get main-fn 2)]
    (do
      (print (vector-length ir))
      (print-ir-loop ir 0 (vector-length ir))
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

    assert_eq!(
        lines,
        [
            "10", "1", "7", "1", "0", "1", "0", "30", "0", "41", "0", "1", "42", "79", "0", "1",
            "9", "43", "0", "20", "0",
        ],
        "source compiler は if の branch/end と outer add の IR 順序を保持すべき"
    );
}

/// parser-to-inference bundle: closed type-alias は defn の引数・戻り値注釈で透過展開する
#[test]
fn test_e2e_selfhost_parser_closed_type_alias_unifies_defn_signature() {
    let harness = r#"
(defn main []
  (let [valid-program
          (parse-program "(type-alias Text String) (type-alias RefText (Ref Text)) (type-alias TextFn (-> Text Text)) (defn echo [(: value Text)] : String value) (defn label [] : Text \"ok\") (defn ref-echo [(: value RefText)] : (Ref String) value) (defn fn-echo [(: f (-> Text Text))] : TextFn f)")
        valid-analysis (infer-program-analysis valid-program)
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type-alias Text String) (defn invalid [] : Text 1)"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "closed type-alias は defn signature で String と同じ型として検査されるべき"
    );
}

/// parser-to-inference bundle: forward type-alias chain は宣言順に依存せず透過展開する
#[test]
fn test_e2e_selfhost_parser_forward_type_alias_unifies_signature() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type-alias Later LaterTarget) (type-alias LaterTarget String) (defn echo [(: value Later)] : String value)"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type-alias Later LaterTarget) (type-alias LaterTarget String) (defn invalid [] : Later 42)"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "forward type-alias chain は後続 alias の target まで展開して型検査するべき"
    );
}

/// parser-to-inference bundle: recursive type-alias は Rust implementation と同じく拒否する
#[test]
fn test_e2e_selfhost_parser_recursive_type_alias_is_rejected() {
    let source = "(type-alias Rec Rec) (defn ok [] : Int 42)\n";
    let rust_program = lsharp_syntax::parse(source).expect("recursive alias fixture は parse できるべき");
    let mut oracle = Infer::new();
    let error = oracle
        .infer_program(&rust_program)
        .expect_err("Rust oracle は recursive alias を拒否するべき");
    assert_eq!(error.code(), "LS1008");
    assert!(
        error.to_string().contains("再帰的な型エイリアス"),
        "Rust oracle の recursive alias message が不足: {error}"
    );
    let span = error.span().expect("Rust oracle の recursive alias は span を持つべき");
    assert_eq!((span.start, span.end), (0, 20));

    let harness = r#"
(defn main []
  (let [program (parse-program "(type-alias Rec Rec) (defn ok [] : Int 42)")
        analysis (infer-program-analysis program)
        recursive-decl (vector-get program 0)
        kinds (infer-program-analysis-failure-kinds analysis)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      (print (infer-program-analysis-first-error-index analysis))
      (print (if (= (infer-program-analysis-first-error-name-hash analysis)
                    (vector-get recursive-decl 1))
                  1
                  0))
      (print (infer-program-analysis-first-error-start analysis))
      (print (infer-program-analysis-first-error-end analysis))
      (print (vector-length kinds))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "8", "-1", "1", "0", "20", "0"],
        "recursive type-alias は code、alias provenance、宣言 spanを保持し、defn failure-kindsへ混入させないべき"
    );
}

/// parser-to-inference bundle: closed type-alias は式内 annotation でも透過展開する
#[test]
fn test_e2e_selfhost_parser_closed_type_alias_unifies_annotation_expr() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type-alias Str String) (defn hello [] (: \"world\" Str))"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type-alias Str String) (defn invalid [] (: 42 Str))"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "closed type-alias は式内 annotation で String と同じ型として検査されるべき"
    );
}

/// parser-to-inference bundle: parametric type-alias は適用された型引数で target を置換する
#[test]
fn test_e2e_selfhost_parser_parametric_type_alias_unifies_signature() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type-alias (Zero) String) (type-alias (Id a) a) (type-alias (Wrapped a) (Id a)) (type-alias (Callback a b) (-> a b)) (type-alias (Box a) (Ref a)) (defn zero [] : Zero \"zero\") (defn identity [(: value (Id Int))] : Int value) (defn wrapped [(: value (Wrapped Int))] : Int value) (defn callback [(: f (Callback Int String))] : (-> Int String) f) (defn box [(: value (Box String))] : (Ref String) value) (defn annotated [] (: \"text\" (Id String)))"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type-alias (Id a) a) (defn invalid [] (: \"text\" (Id Int)))"))
        arity-analysis
          (infer-program-analysis
            (parse-program "(type-alias (Id a) a) (defn arity [(: value (Id Int String))] : Int value)"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      (print (infer-program-analysis-diagnostic-count arity-analysis))
      (print (infer-program-analysis-first-error-code arity-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6", "1", "6"],
        "parametric type-alias は arity 一致時だけ target 型へ展開されるべき"
    );
}

/// parser-to-inference bundle: nonparametric record 宣言は constructor と literal を型検査する
#[test]
fn test_e2e_selfhost_record_decl_registers_constructor_and_literal_fields() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type Point (record (: x Int) (: y Int))) (defn from-constructor [] (Point 1 2)) (defn from-literal [] {Point x 1 y 2})"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type Point (record (: x Int) (: y Int))) (defn invalid [] {Point x true y 2})"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "record 宣言は constructor/literal を登録し、field 型不一致を診断するべき"
    );
}

/// parser-to-inference bundle: parametric record は constructor/literal ごとに型変数を具体化する
#[test]
fn test_e2e_selfhost_parametric_record_registers_fresh_constructor_and_literal_schemas() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type (Box a) (record (: value a))) (defn int-constructor [] (Box 1)) (defn bool-constructor [] (Box true)) (defn int-literal [] {Box value 1}) (defn bool-literal [] {Box value true})"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a) (record (: left a) (: right a))) (defn invalid [] {Pair left 1 right true})"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "parametric record は使用箇所ごとに fresh で、同一 literal 内では field 型を共有するべき"
    );
}

/// parser-to-inference bundle: parametric record の field access は let 束縛後も schema の field 型を使う
#[test]
fn test_e2e_selfhost_parametric_record_field_access_uses_instantiated_schema() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn int-first [] (let [pair {Pair fst 1 snd true}] (: (. pair fst) Int))) (defn bool-second [] (let [pair {Pair fst 1 snd true}] (: (. pair snd) Bool)))"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn invalid [] (let [pair {Pair fst 1 snd true}] (: (. pair fst) Bool)))"))
        unknown-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn unknown [] (let [pair {Pair fst 1 snd true}] (. pair missing)))"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      (print (infer-program-analysis-diagnostic-count unknown-analysis))
      (print (infer-program-analysis-first-error-code unknown-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6", "1", "6"],
        "parametric record の field access は具体化済み schema の field 型を返し、未定義 field を診断するべき"
    );
}

/// parser-to-inference bundle: parametric record update は schema の field 型を検査する
#[test]
fn test_e2e_selfhost_parametric_record_update_uses_instantiated_schema() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn valid [] (let [pair {Pair fst 1 snd true}] (: (. {pair | snd false} snd) Bool)))"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn invalid [] (let [pair {Pair fst 1 snd true}] {pair | snd 2}))"))
        unknown-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn unknown [] (let [pair {Pair fst 1 snd true}] {pair | missing 2}))"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      (print (infer-program-analysis-diagnostic-count unknown-analysis))
      (print (infer-program-analysis-first-error-code unknown-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6", "1", "6"],
        "parametric record update は具体化済み schema の field 型を検査し、未定義 field を診断するべき"
    );
}

/// parser-to-inference bundle: Rust 互換の Type.field accessor は record schema を多相に具体化する
#[test]
fn test_e2e_selfhost_parametric_record_static_accessor_uses_instantiated_schema() {
    let harness = r#"
(defn main []
  (let [valid-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn first [] (: (Pair.fst {Pair fst 1 snd true}) Int)) (defn second [] (: (Pair.snd {Pair fst 1 snd true}) Bool))"))
        invalid-analysis
          (infer-program-analysis
            (parse-program "(type (Pair a b) (record (: fst a) (: snd b))) (defn invalid [] (: (Pair.fst {Pair fst 1 snd true}) Bool))"))]
    (do
      (print (infer-program-analysis-diagnostic-count valid-analysis))
      (print (infer-program-analysis-first-error-code valid-analysis))
      (print (infer-program-analysis-diagnostic-count invalid-analysis))
      (print (infer-program-analysis-first-error-code invalid-analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "Type.field accessor は record schema を使用し、field 型不一致を診断するべき"
    );
}

/// parser-to-inference bundle: parametric ADT 宣言は constructor と match pattern を型検査する
#[test]
fn test_e2e_selfhost_parametric_adt_registers_constructors_and_match() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(type (Maybe a) (Just a) Nothing) (defn from-int [] (Just 1)) (defn fallback [m] (match m [(Just value) value] [Nothing 0])) (defn main-value [] (fallback (Just 4)))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "parametric ADT の constructor と match pattern は同じ型スキームから検査されるべき"
    );
}

/// parser-to-inference bundle: parametric ADT constructor は使用箇所ごとに具体化される
#[test]
fn test_e2e_selfhost_parametric_adt_constructors_instantiate_per_use() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(type (Maybe a) (Just a) Nothing) (defn int-or [m] (match m [(Just value) (+ value 1)] [Nothing 0])) (defn bool-or [m] (match m [(Just value) (if value 1 0)] [Nothing 0])) (defn use-int [] (int-or (Just 1))) (defn use-bool [] (bool-or (Just true)))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "parametric ADT constructor は Int と Bool の各使用箇所で独立に具体化されるべき"
    );
}

/// parser-to-inference bundle: GADT constructor は宣言された戻り型を保持する
#[test]
fn test_e2e_selfhost_gadt_constructor_registers_refined_return_type() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(type (Expr a) (: (IntLit Int) (Expr Int)) (: (BoolLit Bool) (Expr Bool))) (defn make-int [] (IntLit 1))"))
        env (infer-program-analysis-env analysis)
        scheme (type-env-lookup env (name-hash "make-int" 0 8))
        fun-ty (scheme-type scheme)
        ty (type-fun-ret fun-ty)
        arg (type-app-arg ty 0)]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (type-tag fun-ty))
      (print (type-tag ty))
      (print (if (= (type-app-name ty) (name-hash "Expr" 0 4)) 1 0))
      (print (type-tag arg))
      (print (type-name arg))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "3", "5", "1", "1", "100"],
        "0 引数の GADT constructor は Unit -> Expr Int として登録されるべき (I-45)",
    );
}

/// parser-to-inference bundle: GADT match は arm ごとに戻り型の refinement を適用する
#[test]
fn test_e2e_selfhost_gadt_match_refines_each_constructor_arm() {
    let harness = r#"
(defn main []
  (let [analysis
          (infer-program-analysis
            (parse-program "(type (Expr a) (: (IntLit Int) (Expr Int)) (: (BoolLit Bool) (Expr Bool))) (defn eval [expr] (match expr [(IntLit value) value] [(BoolLit value) (if value 1 0)]))"))]
    (do
      (print (infer-program-analysis-diagnostic-count analysis))
      (print (infer-program-analysis-first-error-code analysis))
      0)))
"#;

    let combined = format!(
        "{}\n{}",
        selfhost_parser_typeinfer_runtime_bundle(),
        harness
    );
    let output = run_with_expanded_stack(NATIVE_HARNESS_STACK_BYTES, move || {
        compile_and_run(&combined)
    });
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "GADT match は IntLit / BoolLit の各 arm を独立に refinement できるべき",
    );
}
