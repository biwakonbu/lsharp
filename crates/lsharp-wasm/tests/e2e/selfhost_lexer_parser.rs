use super::support::*;

// =================================================// selfhost Lexer.ls 拡張テスト (Step 3)
// =================================================
#[test]
fn test_e2e_selfhost_negative_int_parses_as_int() {
    let harness = r#"
(defn main []
  (let [program (parse-program "-1")
        expr (vector-get program 0)]
    (do
      (print (vector-length program))
      (print (vector-get expr 0))
      (print (vector-get expr 1))
      0)))
"#;

    let combined = format!("{}\n{}", selfhost_cli_runtime_bundle(), harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(lines[0], "1", "program は 1 式を返すべき");
    assert_eq!(lines[1], "1", "-1 は int node (tag=1) であるべき");
    assert_eq!(lines[2], "-1", "-1 の値が保持されるべき");
}

/// parser-to-inference bundle: parser が保持した defn signature を type inference が検査する
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature_rejects_mismatch() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn invalid [(: x Bool)] : Int x)")
        analysis (infer-program-analysis program)]
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
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "6"],
        "parser 経由の typed defn signature は型不一致を診断するべき"
    );
}

/// parser-to-inference bundle: applied / function signature を自己ホスト型推論へ渡せる
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature_unifies_type_app_and_fun() {
    let harness = r#"
(defn main []
  (let [program (parse-program "(defn ref-id [(: x (Ref (Vector Int)))] : (Ref (Vector Int)) x) (defn fn-id [(: f (-> Int String Bool))] : (-> Int String Bool) f)")
        analysis (infer-program-analysis program)]
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
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "parser 経由の TypeApp / TypeFun signature は型推論に渡せるべき"
    );
}

/// parser-to-inference bundle: TypeVar signature は同名を維持し異名を拒否する
#[test]
fn test_e2e_selfhost_parser_typed_defn_signature_unifies_type_var() {
    let harness = r#"
(defn main []
  (let [valid-analysis (infer-program-analysis (parse-program "(defn id [(: x a)] : a x)"))
        invalid-analysis (infer-program-analysis (parse-program "(defn invalid [(: x a)] : b x)"))]
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
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "parser 経由の TypeVar signature は同名だけを一致させるべき"
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
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0", "1", "6"],
        "closed type-alias は defn signature で String と同じ型として検査されるべき"
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
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
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "0"],
        "parametric ADT constructor は Int と Bool の各使用箇所で独立に具体化されるべき"
    );
}

#[test]
fn test_e2e_selfhost_lexer_arrow_dot() {
    // Lexer.ls が -> と . を正しくトークン化できることを検証
    let source = r#"
(defn main []
  (let [src "-> . x"
        tokens (tokenize-with-spans src)
        n (token-count tokens)]
    (do
      (print n)                      ;; トークン数
      (print (token-kind tokens 0))  ;; -> の kind
      (print (token-kind tokens 1))  ;; . の kind
      (print (token-kind tokens 2))  ;; x の kind
      0)))

;; Lexer.ls の全関数をインライン
(defn is-ws [c]
  (if (== c 32) true (if (== c 9) true (if (== c 10) true (== c 13)))))

(defn is-digit-char [c]
  (if (>= c 48) (<= c 57) false))

(defn is-alpha-char [c]
  (if (>= c 65)
    (if (<= c 90) true
      (if (>= c 97) (<= c 122) false))
    false))

(defn is-symbol-start [c]
  (if (is-alpha-char c) true
    (if (== c 95) true
      (if (== c 43) true
        (if (== c 45) true
          (if (== c 42) true
            (if (== c 47) true
              (if (== c 61) true
                (if (== c 60) true
                  (if (== c 62) true
                    (if (== c 33) true
                      (if (== c 63) true
                        (if (== c 38) true
                          (if (== c 37) true
                            (== c 126)))))))))))))))

(defn is-symbol-char [c]
  (if (is-symbol-start c) true
    (if (is-digit-char c) true
      (if (== c 46) true
        (== c 45)))))

(defn skip-comment [src pos len]
  (if (>= pos len) pos
    (if (== (string-char-at src pos) 10)
      (+ pos 1)
      (skip-comment src (+ pos 1) len))))

(defn skip-ws-loop [src pos len]
  (if (>= pos len) pos
    (let [c (string-char-at src pos)]
      (if (is-ws c)
        (skip-ws-loop src (+ pos 1) len)
        (if (== c 59)
          (let [end (skip-comment src (+ pos 1) len)]
            (skip-ws-loop src end len))
          pos)))))

(defn classify-symbol [name]
  (if (string-eq name "defn") 30
    (if (string-eq name "let") 31
      (if (string-eq name "if") 32
        (if (string-eq name "match") 33
          (if (string-eq name "type") 34
            (if (string-eq name "fn") 35
              (if (string-eq name "do") 36
                (if (string-eq name "module") 37
                  (if (string-eq name "import") 38
                    (if (string-eq name "record") 39
                      (if (string-eq name "trait") 40
                        (if (string-eq name "impl") 41
                          (if (string-eq name "where") 42
                            (if (string-eq name "private") 43
                              (if (string-eq name "true") 13
                                (if (string-eq name "false") 14
                                  20)))))))))))))))))

(defn scan-digits [src pos len]
  (if (>= pos len) pos
    (if (is-digit-char (string-char-at src pos))
      (scan-digits src (+ pos 1) len)
      pos)))

(defn scan-symbol-end [src pos len]
  (if (>= pos len) pos
    (if (is-symbol-char (string-char-at src pos))
      (scan-symbol-end src (+ pos 1) len)
      pos)))

(defn scan-string-end [src pos len]
  (if (>= pos len) pos
    (let [c (string-char-at src pos)]
      (if (== c 34) (+ pos 1)
        (if (== c 92) (scan-string-end src (+ pos 2) len)
          (scan-string-end src (+ pos 1) len))))))

(defn lex-one [src pos len]
  (if (>= pos len)
    (+ (* 99 1000000) pos)
    (let [c (string-char-at src pos)]
      (if (== c 40) (+ (* 0 1000000) (+ pos 1))
        (if (== c 41) (+ (* 1 1000000) (+ pos 1))
          (if (== c 91) (+ (* 2 1000000) (+ pos 1))
            (if (== c 93) (+ (* 3 1000000) (+ pos 1))
              (if (== c 123) (+ (* 4 1000000) (+ pos 1))
                (if (== c 125) (+ (* 5 1000000) (+ pos 1))
                  (if (== c 58) (+ (* 50 1000000) (+ pos 1))
                    (if (== c 124) (+ (* 52 1000000) (+ pos 1))
                      (if (== c 46) (+ (* 53 1000000) (+ pos 1))
                        (if (== c 39) (+ (* 18 1000000) (+ pos 1))
                          (if (== c 34)
                            (let [end (scan-string-end src (+ pos 1) len)]
                              (+ (* 12 1000000) end))
                            (if (== c 45)
                              (if (< (+ pos 1) len)
                                (if (== (string-char-at src (+ pos 1)) 62)
                                  (+ (* 51 1000000) (+ pos 2))
                                  (let [end (scan-symbol-end src (+ pos 1) len)
                                        name (substring src pos end)
                                        kind (classify-symbol name)]
                                    (+ (* kind 1000000) end)))
                                (+ (* 20 1000000) (+ pos 1)))
                              (if (is-digit-char c)
                                (let [end (scan-digits src (+ pos 1) len)]
                                  (+ (* 10 1000000) end))
                                (if (is-symbol-start c)
                                  (let [end (scan-symbol-end src (+ pos 1) len)
                                        name (substring src pos end)
                                        kind (classify-symbol name)]
                                    (+ (* kind 1000000) end))
                                  (+ (* 99 1000000) (+ pos 1)))))))))))))))))))

(defn tokenize-spans-loop [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
      (let [result (lex-one src ws-pos len)
            kind (/ result 1000000)
            end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
          (tokenize-spans-loop src end-pos len
            (vector-push (vector-push (vector-push tokens kind) ws-pos) end-pos)))))))

(defn tokenize-with-spans [src]
  (tokenize-spans-loop src 0 (string-length src) (vector-new 32)))

(defn token-count [tokens]
  (/ (vector-length tokens) 3))

(defn token-kind [tokens n]
  (vector-get tokens (* n 3)))
"#;
    let result = compile_and_run_expanded(source);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "4", "token count: -> . x EOF");
    assert_eq!(lines[1], "51", "-> = tok-arrow (51)");
    assert_eq!(lines[2], "53", ". = tok-dot (53)");
    assert_eq!(lines[3], "20", "x = tok-symbol (20)");
}

#[test]
fn test_e2e_selfhost_lexer_additional_keywords() {
    // Lexer.ls が追加キーワード (open, constrained 等) を認識できるか検証
    let source = r#"
(defn classify-symbol [name]
  (if (string-eq name "defn") 30
    (if (string-eq name "let") 31
      (if (string-eq name "if") 32
        (if (string-eq name "match") 33
          (if (string-eq name "type") 34
            (if (string-eq name "fn") 35
              (if (string-eq name "do") 36
                (if (string-eq name "module") 37
                  (if (string-eq name "import") 38
                    (if (string-eq name "record") 39
                      (if (string-eq name "trait") 40
                        (if (string-eq name "impl") 41
                          (if (string-eq name "where") 42
                            (if (string-eq name "private") 43
                              (if (string-eq name "open") 44
                                (if (string-eq name "constrained") 45
                                  (if (string-eq name "computation") 46
                                    (if (string-eq name "defmacro") 47
                                      (if (string-eq name "true") 13
                                        (if (string-eq name "false") 14
                                          20)))))))))))))))))))))

(defn main []
  (do
    (print (classify-symbol "open"))
    (print (classify-symbol "constrained"))
    (print (classify-symbol "computation"))
    (print (classify-symbol "defmacro"))
    (print (classify-symbol "unknown"))
    0))
"#;
    let result = compile_and_run_expanded(source);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "44", "open = 44");
    assert_eq!(lines[1], "45", "constrained = 45");
    assert_eq!(lines[2], "46", "computation = 46");
    assert_eq!(lines[3], "47", "defmacro = 47");
    assert_eq!(lines[4], "20", "unknown = symbol (20)");
}

#[test]
fn test_e2e_selfhost_lexer_keyword_token_consistency() {
    let token_ls = std::fs::read_to_string(selfhost_source_path("Token.ls"))
        .expect("canonical Token.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(selfhost_source_path("Lexer.ls"))
        .expect("canonical Lexer.ls が読み込めない");

    let harness = r#"
(defn main []
  (do
    (print (if (= (classify-symbol "open") (tok-open-kw)) 1 0))
    (print (if (= (classify-symbol "constrained") (tok-constrained)) 1 0))
    (print (if (= (classify-symbol "computation") (tok-computation)) 1 0))
    (print (if (= (classify-symbol "defmacro") (tok-defmacro)) 1 0))
    (print (if (= (classify-symbol "builder") (tok-builder)) 1 0))
    0))
"#;

    let combined = format!("{}\n{}\n{}", token_ls, lexer_ls, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "追加キーワードの整合性出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "open は Token.tok-open-kw と一致すべき");
    assert_eq!(
        lines[1], "1",
        "constrained は Token.tok-constrained と一致すべき"
    );
    assert_eq!(
        lines[2], "1",
        "computation は Token.tok-computation と一致すべき"
    );
    assert_eq!(lines[3], "1", "defmacro は Token.tok-defmacro と一致すべき");
    assert_eq!(lines[4], "1", "builder は Token.tok-builder と一致すべき");
}

#[test]
fn test_e2e_selfhost_lexer_special_token_consistency() {
    let token_ls = std::fs::read_to_string(selfhost_source_path("Token.ls"))
        .expect("canonical Token.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(selfhost_source_path("Lexer.ls"))
        .expect("canonical Lexer.ls が読み込めない");
    let lexer_compat_ls = std::fs::read_to_string(selfhost_source_path("LexerCompat.ls"))
        .expect("canonical LexerCompat.ls が読み込めない");

    let harness = r#"
(defn main []
  (let [tokens (tokenize-with-spans "' ~ ~@ # @")]
    (do
      (print (if (= (token-kind tokens 0) (tok-quote)) 1 0))
      (print (if (= (token-kind tokens 1) (tok-unquote)) 1 0))
      (print (if (= (token-kind tokens 2) (tok-splice-unquote)) 1 0))
      (print (if (= (token-kind tokens 3) (tok-hash)) 1 0))
      (print (if (= (token-kind tokens 4) (tok-at)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}",
        token_ls, lexer_ls, lexer_compat_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "特殊トークンの整合性出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", "quote は Token.tok-quote と一致すべき");
    assert_eq!(lines[1], "1", "unquote は Token.tok-unquote と一致すべき");
    assert_eq!(
        lines[2], "1",
        "splice-unquote は Token.tok-splice-unquote と一致すべき"
    );
    assert_eq!(lines[3], "1", "hash は Token.tok-hash と一致すべき");
    assert_eq!(lines[4], "1", "at は Token.tok-at と一致すべき");
}

#[test]
fn test_e2e_selfhost_lexer_tokenizes_large_input_without_stack_trap() {
    let token_ls = std::fs::read_to_string(selfhost_source_path("Token.ls"))
        .expect("canonical Token.ls が読み込めない");
    let lexer_ls = std::fs::read_to_string(selfhost_source_path("Lexer.ls"))
        .expect("canonical Lexer.ls が読み込めない");
    let lexer_compat_ls = std::fs::read_to_string(selfhost_source_path("LexerCompat.ls"))
        .expect("canonical LexerCompat.ls が読み込めない");
    let repeated_symbols = std::iter::repeat_n("x ", 5000).collect::<String>();
    let harness = format!(
        r#"
(defn main []
  (let [tokens (tokenize-with-spans "{repeated_symbols}")]
    (do
      (print (token-count tokens))
      (print (token-kind tokens 0))
      (print (token-kind tokens 4999))
      (print (token-kind tokens 5000))
      0)))
"#
    );

    let combined = format!(
        "{}\n{}\n{}\n{}",
        token_ls, lexer_ls, lexer_compat_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "大入力 tokenization の出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "5001", "5000 symbol + EOF を返すべき");
    assert_eq!(lines[1], "20", "先頭 token は symbol");
    assert_eq!(lines[2], "20", "末尾直前 token も symbol");
    assert_eq!(lines[3], "99", "最後は EOF");
}

// =================================================// selfhost Parser.ls 全構文テスト (Step 4)
// =================================================
#[test]
fn test_e2e_selfhost_parser_full_sexp() {
    // Parser が完全な S 式をパースして AST を構築できることを検証
    // parse-expr-v3: span ベースのトークンから再帰的に AST を構築
    let source = r#"
;; AST タグ定数
;; 1=int, 2=bool, 4=var, 5=apply, 6=if, 7=let, 8=lambda, 9=do, 10=match, 20=defn

;; パーサー状態: ref-cell で位置を管理
;; トークンは (kind, start, end) の3つ組 Vector

;; N 番目のトークンの kind
(defn span-kind [spans n]
  (vector-get spans (* n 3)))

;; パーサー位置を1つ進める
(defn p-advance [pos-ref]
  (ref-set pos-ref (+ (ref-get pos-ref) 1)))

;; 現在のトークン kind を取得
(defn p-current [spans pos-ref]
  (span-kind spans (ref-get pos-ref)))

;; 整数リテラルのパース
(defn parse-int-v3 [spans pos-ref src]
  (let [n (ref-get pos-ref)
        start (vector-get spans (+ (* n 3) 1))
        end (vector-get spans (+ (* n 3) 2))
        value (parse-int-from-str src start end 0)]
    (do (p-advance pos-ref)
        ;; [1, value]
        (vector-push (vector-push (vector-new 2) 1) value))))

(defn parse-int-from-str [src pos end acc]
  (if (>= pos end) acc
    (let [digit (- (string-char-at src pos) 48)]
      (parse-int-from-str src (+ pos 1) end (+ (* acc 10) digit)))))

;; 変数参照のパース (名前はソース位置で識別)
(defn parse-var-v3 [spans pos-ref src]
  (let [n (ref-get pos-ref)
        start (vector-get spans (+ (* n 3) 1))]
    (do (p-advance pos-ref)
        (vector-push (vector-push (vector-new 2) 4) start))))

;; 式のパース (メインディスパッチ)
(defn parse-expr-v3 [spans pos-ref src]
  (let [kind (p-current spans pos-ref)]
    (if (== kind 10)  ;; Int
      (parse-int-v3 spans pos-ref src)
      (if (== kind 13)  ;; true
        (do (p-advance pos-ref)
            (vector-push (vector-push (vector-new 2) 2) 1))
        (if (== kind 14)  ;; false
          (do (p-advance pos-ref)
              (vector-push (vector-push (vector-new 2) 2) 0))
          (if (== kind 20)  ;; Symbol
            (parse-var-v3 spans pos-ref src)
            (if (== kind 0)  ;; LParen -> S 式
              (parse-sexp-v3 spans pos-ref src)
              ;; unknown
              (vector-push (vector-push (vector-new 2) 0) 0))))))))

;; S 式のパース (( の後のキーワードディスパッチ)
(defn parse-sexp-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; ( を消費
    (let [kind (p-current spans pos-ref)]
      (if (== kind 32)  ;; if
        (parse-if-v3 spans pos-ref src)
        (if (== kind 31)  ;; let
          (parse-let-v3 spans pos-ref src)
          (if (== kind 36)  ;; do
            (parse-do-v3 spans pos-ref src)
            ;; apply (関数呼び出し)
            (parse-apply-v3 spans pos-ref src)))))))

;; if 式のパース
(defn parse-if-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; if を消費
    (let [cond-node (parse-expr-v3 spans pos-ref src)
          then-node (parse-expr-v3 spans pos-ref src)
          else-node (parse-expr-v3 spans pos-ref src)]
      (do
        (p-advance pos-ref)  ;; ) を消費
        (let [n (vector-new 8)]
          (vector-push (vector-push (vector-push (vector-push n 6)
            cond-node) then-node) else-node))))))

;; let 式のパース (簡易版: 1 バインディング)
(defn parse-let-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; let を消費
    (p-advance pos-ref)  ;; [ を消費
    (let [;; name (ソース位置で識別)
          name-n (ref-get pos-ref)
          name-start (vector-get spans (+ (* name-n 3) 1))]
      (do
        (p-advance pos-ref)  ;; name を消費
        (let [init (parse-expr-v3 spans pos-ref src)]
          (do
            (p-advance pos-ref)  ;; ] を消費
            (let [body (parse-expr-v3 spans pos-ref src)]
              (do
                (p-advance pos-ref)  ;; ) を消費
                (let [n (vector-new 8)]
                  (vector-push (vector-push (vector-push (vector-push n 7)
                    name-start) init) body))))))))))

;; do 式のパース (最後の式の値を返す)
(defn parse-do-v3 [spans pos-ref src]
  (do
    (p-advance pos-ref)  ;; do を消費
    (let [first-expr (parse-expr-v3 spans pos-ref src)
          second-expr (if (== (p-current spans pos-ref) 1) ;; ) で終わり?
                        first-expr
                        (parse-expr-v3 spans pos-ref src))]
      (do
        ;; 残りの式をスキップして ) まで
        (p-advance pos-ref)  ;; ) を消費
        (let [n (vector-new 8)]
          (vector-push (vector-push (vector-push n 9)
            first-expr) second-expr))))))

;; apply 式のパース (func arg1 arg2)
(defn parse-apply-v3 [spans pos-ref src]
  (let [func-node (parse-expr-v3 spans pos-ref src)
        ;; 引数を収集
        arg1 (if (== (p-current spans pos-ref) 1)
                0  ;; 引数なし
                (parse-expr-v3 spans pos-ref src))
        arg2 (if (== (p-current spans pos-ref) 1)
                0  ;; 2番目の引数なし
                (parse-expr-v3 spans pos-ref src))]
    (do
      (p-advance pos-ref)  ;; ) を消費
      (let [n (vector-new 8)]
        (vector-push (vector-push (vector-push (vector-push n 5)
          func-node) arg1) arg2)))))

;; === Lexer (インライン) ===
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
          (if (== c 63) true (if (== c 38) true
            (if (== c 37) true (== c 126)))))))))))))))
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
        (if (== c 59) (let [end (skip-comment src (+ pos 1) len)] (skip-ws-loop src end len))
          pos)))))

(defn classify-symbol [name]
  (if (string-eq name "defn") 30
    (if (string-eq name "let") 31
      (if (string-eq name "if") 32
        (if (string-eq name "match") 33
          (if (string-eq name "type") 34
            (if (string-eq name "fn") 35
              (if (string-eq name "do") 36
                (if (string-eq name "module") 37
                  (if (string-eq name "import") 38
                    (if (string-eq name "record") 39
                      (if (string-eq name "trait") 40
                        (if (string-eq name "impl") 41
                          (if (string-eq name "where") 42
                            (if (string-eq name "private") 43
                              (if (string-eq name "true") 13
                                (if (string-eq name "false") 14
                                  20)))))))))))))))))

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

(defn tokenize-spans-loop [src pos len tokens]
  (let [ws-pos (skip-ws-loop src pos len)]
    (if (>= ws-pos len)
      (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
      (let [result (lex-one src ws-pos len)
            kind (/ result 1000000)
            end-pos (- result (* kind 1000000))]
        (if (== kind 99)
          (vector-push (vector-push (vector-push tokens 99) ws-pos) ws-pos)
          (tokenize-spans-loop src end-pos len
            (vector-push (vector-push (vector-push tokens kind) ws-pos) end-pos)))))))

(defn tokenize-with-spans [src]
  (tokenize-spans-loop src 0 (string-length src) (vector-new 32)))

(defn token-count [tokens]
  (/ (vector-length tokens) 3))

(defn main []
  (let [src "(if (> x 10) 42 0)"
        spans (tokenize-with-spans src)
        pos-ref (ref-new 0)
        ast (parse-expr-v3 spans pos-ref src)
        ;; AST のタグを確認
        tag (vector-get ast 0)]
    (do
      (print tag)  ;; 6 (if)
      ;; let 式テスト
      (let [src2 "(let [y 5] (+ y 1))"
            spans2 (tokenize-with-spans src2)
            pos2 (ref-new 0)
            ast2 (parse-expr-v3 spans2 pos2 src2)
            tag2 (vector-get ast2 0)]
        (do
          (print tag2)  ;; 7 (let)
          ;; do 式テスト
          (let [src3 "(do (print 1) 42)"
                spans3 (tokenize-with-spans src3)
                pos3 (ref-new 0)
                ast3 (parse-expr-v3 spans3 pos3 src3)
                tag3 (vector-get ast3 0)]
            (do
              (print tag3)  ;; 9 (do)
              ;; apply 式テスト
              (let [src4 "(+ 1 2)"
                    spans4 (tokenize-with-spans src4)
                    pos4 (ref-new 0)
                    ast4 (parse-expr-v3 spans4 pos4 src4)
                    tag4 (vector-get ast4 0)]
                (do
                  (print tag4)  ;; 5 (apply)
                  0)))))))))
"#;
    let result = compile_and_run_expanded(source);
    let lines: Vec<&str> = result.trim().lines().collect();
    assert_eq!(lines[0], "6", "if 式のパース: tag=6");
    assert_eq!(lines[1], "7", "let 式のパース: tag=7");
    assert_eq!(lines[2], "9", "do 式のパース: tag=9");
    assert_eq!(lines[3], "5", "apply 式のパース: tag=5");
}
