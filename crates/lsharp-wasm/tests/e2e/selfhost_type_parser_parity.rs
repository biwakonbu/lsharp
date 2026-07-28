use super::support::*;

// =============================================================================
// TEST-TYPE-07: error code/span/primary message の parity golden
// golden fixture に Rust 側の型エラー (error code, span, message) を記録し、
// selfhost 側が同じ error code を生成することを検証する準備
// =============================================================================

#[test]
fn test_e2e_selfhost_type_error_parity() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // 1. golden fixture の読み込み
    let golden_path = project_root.join("tests/golden/types/type_errors.json");
    assert!(
        golden_path.exists(),
        "tests/golden/types/type_errors.json が存在しない"
    );
    let golden_content =
        std::fs::read_to_string(&golden_path).expect("type_errors.json の読み込みに失敗");
    let golden: serde_json::Value =
        serde_json::from_str(&golden_content).expect("type_errors.json の JSON パースに失敗");

    // 2. golden fixture の構造検証
    let error_cases = golden
        .get("type_errors")
        .expect("type_errors セクションがない");
    assert!(error_cases.is_array(), "type_errors が配列でない");
    let cases = error_cases.as_array().unwrap();
    assert!(
        cases.len() >= 3,
        "type_errors のテストケースが 3 件未満: {}",
        cases.len()
    );

    // 3. 各テストケースで Rust 側の型推論がエラーを返すことを検証
    for (i, case) in cases.iter().enumerate() {
        let source = case
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("テストケース {} に source がない", i));
        let expected = case
            .get("expected")
            .unwrap_or_else(|| panic!("テストケース {} に expected がない", i));
        let error_code = expected
            .get("error_code")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("テストケース {} に error_code がない", i));

        let program = lsharp_syntax::parse(source);
        if let Ok(prog) = program {
            let mut infer = Infer::new();
            let result = infer.infer_program(&prog);

            // 型エラーが発生すること
            assert!(
                result.is_err(),
                "テストケース {}: '{}' で型エラーが発生しなかった (error_code: {})",
                i,
                source,
                error_code
            );

            // selfhost 側が同じ error_code を生成することを検証 (MetadataCheck.ls の実装後)
            // 現時点では Rust 側のエラー文字列に error_code が含まれることを検証
            let err_msg = format!("{}", result.unwrap_err());
            assert!(
                err_msg.contains(error_code),
                "テストケース {}: エラーメッセージに error_code '{}' が含まれない。\
                 実際のエラー: {}",
                i,
                error_code,
                err_msg
            );
        }
        // パースエラーの場合もテストケースとして記録されている可能性がある
    }
}

// =============================================================================
// TEST-TYPE-08: type variable naming + diagnostics 決定性
// 同じ入力で2回型推論した結果が完全に同じ (type variable 名, diagnostics 順序)
// であることを検証
// =============================================================================

#[test]
fn test_e2e_selfhost_type_deterministic_ordering() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // 1. golden fixture の読み込み
    let golden_path = project_root.join("tests/golden/types/deterministic_ordering.json");
    assert!(
        golden_path.exists(),
        "tests/golden/types/deterministic_ordering.json が存在しない"
    );
    let golden_content = std::fs::read_to_string(&golden_path)
        .expect("deterministic_ordering.json の読み込みに失敗");
    let golden: serde_json::Value = serde_json::from_str(&golden_content)
        .expect("deterministic_ordering.json の JSON パースに失敗");

    // 2. テストケースの構造検証
    let test_cases = golden
        .get("test_cases")
        .expect("test_cases セクションがない");
    assert!(test_cases.is_array(), "test_cases が配列でない");
    let cases = test_cases.as_array().unwrap();
    assert!(cases.len() >= 3, "テストケースが 3 件未満: {}", cases.len());

    // 3. 各テストケースで2回型推論し、結果が同一であることを検証
    for (i, case) in cases.iter().enumerate() {
        let source = case
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("テストケース {} に source がない", i));
        let expects_error = case
            .get("expected_error")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // 1回目の型推論
        let program1 = lsharp_syntax::parse(source);
        if program1.is_err() {
            continue; // パースエラーはスキップ
        }
        let program1 = program1.unwrap();
        let mut infer1 = Infer::new();
        let result1 = infer1.infer_program(&program1);

        // 2回目の型推論
        let program2 = lsharp_syntax::parse(source).unwrap();
        let mut infer2 = Infer::new();
        let result2 = infer2.infer_program(&program2);

        if expects_error {
            // エラーケース: 両方ともエラーであること
            assert!(
                result1.is_err() && result2.is_err(),
                "テストケース {}: エラーが期待されるが、1回目={}, 2回目={}",
                i,
                result1.is_err(),
                result2.is_err()
            );

            // エラーメッセージが同一であること
            let err1 = format!("{}", result1.unwrap_err());
            let err2 = format!("{}", result2.unwrap_err());
            assert_eq!(
                err1, err2,
                "テストケース {}: エラーメッセージが2回の推論で異なる。\n\
                 1回目: {}\n2回目: {}",
                i, err1, err2
            );
        } else {
            // 正常ケース: 両方とも成功すること
            assert!(
                result1.is_ok() && result2.is_ok(),
                "テストケース {}: 成功が期待されるが、1回目={}, 2回目={}",
                i,
                result1.is_ok(),
                result2.is_ok()
            );

            let types1 = result1.unwrap();
            let types2 = result2.unwrap();

            // 推論結果の数が同じ
            assert_eq!(
                types1.len(),
                types2.len(),
                "テストケース {}: 推論結果の数が異なる。1回目={}, 2回目={}",
                i,
                types1.len(),
                types2.len()
            );

            // 各推論結果の型文字列表現が同一
            for (j, (t1, t2)) in types1.iter().zip(types2.iter()).enumerate() {
                let s1 = format!("{:?}", t1);
                let s2 = format!("{:?}", t2);
                assert_eq!(
                    s1, s2,
                    "テストケース {}, 結果 {}: 型表現が2回の推論で異なる。\n\
                     1回目: {}\n2回目: {}",
                    i, j, s1, s2
                );
            }
        }
    }
}

/// selfhost TypeScheme.ls テスト: generalize は source order の自由変数を漏れなく束縛する
#[test]
fn test_e2e_selfhost_typescheme_generalize_preserves_four_var_order() {
    let (type_ls, type_scheme_ls) = typescheme_runtime_modules();

    let harness = r#"
(defn main []
  (let [a (make-type-var 1)
        b (make-type-var 2)
        c (make-type-var 3)
        d (make-type-var 4)
        ty (make-type-fun a (make-type-fun b (make-type-fun c d)))
        scheme (generalize ty (map-new))
        bound (scheme-vars scheme)]
    (do
      (print (vector-length bound))
      (print (vector-get bound 0))
      (print (vector-get bound 1))
      (print (vector-get bound 2))
      (print (vector-get bound 3))
      0)))
"#;

    let combined = format!("{}\n{}\n{}", type_ls, type_scheme_ls, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "generalize deterministic ordering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "4", "generalize は 4 つの自由変数を束縛すべき");
    assert_eq!(lines[1], "1", "1 番目の束縛変数は source order 先頭");
    assert_eq!(lines[2], "2", "2 番目の束縛変数は source order 2 番目");
    assert_eq!(lines[3], "3", "3 番目の束縛変数は source order 3 番目");
    assert_eq!(lines[4], "4", "4 番目の束縛変数は source order 4 番目");
}

/// selfhost TypeScheme.ls テスト: instantiate は bound-vars 全件を順序通り新鮮化する
#[test]
fn test_e2e_selfhost_typescheme_instantiate_rewrites_all_bound_vars() {
    let (type_ls, type_scheme_ls) = typescheme_runtime_modules();

    let harness = r#"
(defn main []
  (let [a (make-type-var 1)
        b (make-type-var 2)
        c (make-type-var 3)
        d (make-type-var 4)
        ty (make-type-fun a (make-type-fun b (make-type-fun c d)))
        bound (vector-push
                (vector-push
                  (vector-push
                    (vector-push (vector-new 4) 1)
                    2)
                  3)
                4)
        scheme (poly ty bound)
        counter (make-var-counter)
        inst (instantiate scheme counter)
        fun2 (type-fun-ret inst)
        fun3 (type-fun-ret fun2)]
    (do
      (print (type-name (type-fun-param inst)))
      (print (type-name (type-fun-param fun2)))
      (print (type-name (type-fun-param fun3)))
      (print (type-name (type-fun-ret fun3)))
      0)))
"#;

    let combined = format!("{}\n{}\n{}", type_ls, type_scheme_ls, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "instantiate deterministic ordering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1000", "1 番目の束縛変数は最初の fresh ID へ");
    assert_eq!(lines[1], "1001", "2 番目の束縛変数は次の fresh ID へ");
    assert_eq!(lines[2], "1002", "3 番目の束縛変数も fresh 化すべき");
    assert_eq!(lines[3], "1003", "4 番目の束縛変数も fresh 化すべき");
}

/// selfhost TypeScheme.ls テスト: record field 型の free vars も source order で一般化する
#[test]
fn test_e2e_selfhost_typescheme_generalize_record_field_vars() {
    let (type_ls, type_scheme_ls) = typescheme_runtime_modules();

    let harness = r#"
(defn main []
  (let [a (make-type-var 1)
        b (make-type-var 2)
        rec0 (make-type-record 900)
        rec1 (type-record-add-field rec0 120 a)
        rec2 (type-record-add-field rec1 121 b)
        scheme (generalize rec2 (map-new))
        bound (scheme-vars scheme)]
    (do
      (print (vector-length bound))
      (print (vector-get bound 0))
      (print (vector-get bound 1))
      0)))
"#;

    let combined = format!("{}\n{}\n{}", type_ls, type_scheme_ls, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "record generalize deterministic ordering 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "2", "record field 型の自由変数 2 個を束縛すべき");
    assert_eq!(
        lines[1], "1",
        "record field の 1 番目の自由変数順が崩れている"
    );
    assert_eq!(
        lines[2], "2",
        "record field の 2 番目の自由変数順が崩れている"
    );
}

/// selfhost TypeScheme.ls テスト: record field 型の bound-vars も instantiate で fresh 化する
#[test]
fn test_e2e_selfhost_typescheme_instantiate_record_field_vars() {
    let (type_ls, type_scheme_ls) = typescheme_runtime_modules();

    let harness = r#"
(defn main []
  (let [a (make-type-var 1)
        b (make-type-var 2)
        rec0 (make-type-record 900)
        rec1 (type-record-add-field rec0 120 a)
        rec2 (type-record-add-field rec1 121 b)
        bound (vector-push (vector-push (vector-new 2) 1) 2)
        scheme (poly rec2 bound)
        counter (make-var-counter)
        inst (instantiate scheme counter)]
    (do
      (print (type-name (type-record-field-type inst 120)))
      (print (type-name (type-record-field-type inst 121)))
      0)))
"#;

    let combined = format!("{}\n{}\n{}", type_ls, type_scheme_ls, harness);
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 2,
        "record instantiate deterministic ordering 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "1000",
        "record field 1 の型変数が fresh 化されていない"
    );
    assert_eq!(
        lines[1], "1001",
        "record field 2 の型変数が fresh 化されていない"
    );
}

// =============================================================================
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
    let parser_content = std::fs::read_to_string(&parser_ls_path)
        .expect("selfhost/src/Syntax/Parser.ls の読み込みに失敗");

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

#[test]
fn test_e2e_selfhost_parser_params_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let params = std::iter::repeat("p").take(65).collect::<Vec<_>>().join(" ");
    let source = format!("(fn [{}] p)", params);
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)]
    (do
      (print (vector-get node 1))
      0)))
"#,
        source
    );

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    assert_eq!(output.trim(), "65", "64 件を超える params も全件保持すべき");
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

/// TEST-SYNTAX-02c3: bare module の後ろに複数 import と defn を top-level で保持できる
#[test]
fn test_e2e_selfhost_parser_bare_module_with_multiple_imports() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [src "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] 0)"
        program (parse-program src)
        node0 (vector-get program 0)
        node1 (vector-get program 1)
        node2 (vector-get program 2)
        node3 (vector-get program 3)]
    (do
      (print (vector-length program))
      (print (if (= (vector-get node0 0) (ast-module-decl)) 1 0))
      (print (if (= (vector-get node0 1) (name-hash "App.Main" 0 8)) 1 0))
      (print (if (= (vector-get node1 0) (ast-import-decl)) 1 0))
      (print (if (= (vector-get node1 1) (name-hash "App.CompilerMode" 0 16)) 1 0))
      (print (if (= (name-hash src (vector-get node1 2) (vector-get node1 3)) (name-hash "App.CompilerMode" 0 16)) 1 0))
      (print (if (= (vector-get node2 0) (ast-import-decl)) 1 0))
      (print (if (= (vector-get node2 1) (name-hash "App.PipelineSmoke" 0 17)) 1 0))
      (print (if (= (name-hash src (vector-get node2 2) (vector-get node2 3)) (name-hash "App.PipelineSmoke" 0 17)) 1 0))
      (print (if (= (vector-get node3 0) (ast-defn)) 1 0))
      (print (if (= (vector-get node3 1) (name-hash "main" 0 4)) 1 0))
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
        "multiple-import parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "4", "program は top-level 4 node を返すべき");
    assert_eq!(lines[1], "1", "node0 は module decl であるべき");
    assert_eq!(lines[2], "1", "module 名 hash が一致すべき");
    assert_eq!(lines[3], "1", "node1 は import decl であるべき");
    assert_eq!(lines[4], "1", "1 個目 import 名 hash が一致すべき");
    assert_eq!(
        lines[5], "1",
        "1 個目 import の start/end から再計算した hash が一致すべき"
    );
    assert_eq!(lines[6], "1", "node2 は import decl であるべき");
    assert_eq!(lines[7], "1", "2 個目 import 名 hash が一致すべき");
    assert_eq!(
        lines[8], "1",
        "2 個目 import の start/end から再計算した hash が一致すべき"
    );
    assert_eq!(lines[9], "1", "node3 は defn であるべき");
    assert_eq!(lines[10], "1", "defn 名 hash が一致すべき");
}

/// EC-M1-01: selfhost parser が import の :as alias を AST に保持すること
#[test]
fn test_e2e_selfhost_parser_import_alias() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [src "(import Lib :as L)"
        program (parse-program src)
        node (vector-get program 0)]
    (do
      (print (vector-length program))
      (print (vector-get node 0))
      (print (if (= (vector-get node 1) (name-hash "Lib" 0 3)) 1 0))
      (print (if (= (vector-length node) 5) 1 0))
      (print (if (= (vector-get node 4) (name-hash "L" 0 1)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 5,
        "import alias parser 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "1", ":as import は単一 AST decl を返すべき");
    assert_eq!(lines[1], "26", ":as import は ast-import-decl を返すべき");
    assert_eq!(lines[2], "1", "module 名 hash は Lib と一致すべき");
    assert_eq!(lines[3], "1", ":as import AST は alias slot を持つべき");
    assert_eq!(lines[4], "1", "alias hash は L と一致すべき");
}

/// EC-M1-01: selfhost parser が import の :only symbols を AST に保持すること
#[test]
fn test_e2e_selfhost_parser_import_only() {
    let source = "(import Lib :only [helper extra])";
    let rust_program =
        lsharp_syntax::parse(source).expect("Rust oracle は import :only を parse できるべき");
    match &rust_program.decls[0] {
        lsharp_syntax::ast::Decl::ImportDecl {
            module,
            alias,
            only,
            open,
            ..
        } => {
            assert_eq!(module, "Lib");
            assert_eq!(*alias, None);
            assert_eq!(only, &Some(vec!["helper".to_string(), "extra".to_string()]));
            assert!(!open);
        }
        decl => panic!("Rust oracle の import decl が不正: {decl:?}"),
    }

    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [src "(import Lib :only [helper extra])"
        program (parse-program src)
        node (vector-get program 0)
        only (vector-get node 5)]
    (do
      (print (vector-length program))
      (print (vector-get node 0))
      (print (if (= (vector-get node 1) (name-hash "Lib" 0 3)) 1 0))
      (print (if (= (vector-length node) 6) 1 0))
      (print (vector-get node 4))
      (print (vector-length only))
      (print (if (= (vector-get only 0) (name-hash "helper" 0 6)) 1 0))
      (print (if (= (vector-get only 1) (name-hash "extra" 0 5)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "26", "1", "1", "0", "2", "1", "1"],
        ":only import は alias slot と選択 symbol hash vector を AST に保持するべき"
    );
}

/// EC-M1-01: selfhost parser が import の :open を AST に保持すること
#[test]
fn test_e2e_selfhost_parser_import_open() {
    let source = "(import Lib :open)";
    let rust_program =
        lsharp_syntax::parse(source).expect("Rust oracle は import :open を parse できるべき");
    match &rust_program.decls[0] {
        lsharp_syntax::ast::Decl::ImportDecl {
            module,
            alias,
            only,
            open,
            ..
        } => {
            assert_eq!(module, "Lib");
            assert_eq!(*alias, None);
            assert_eq!(*only, None);
            assert!(*open);
        }
        decl => panic!("Rust oracle の import decl が不正: {decl:?}"),
    }

    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();

    let harness = r#"
(defn main []
  (let [src "(import Lib :open)"
        program (parse-program src)
        node (vector-get program 0)]
    (do
      (print (vector-length program))
      (print (vector-get node 0))
      (print (if (= (vector-get node 1) (name-hash "Lib" 0 3)) 1 0))
      (print (if (= (vector-length node) 7) 1 0))
      (print (vector-get node 4))
      (print (vector-get node 5))
      (print (vector-get node 6))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "26", "1", "1", "0", "0", "1"],
        ":open import は open flag を AST に保持するべき"
    );
}

/// EC-M1-01: selfhost parser が import の :as と :only を同時に AST へ保持すること
#[test]
fn test_e2e_selfhost_parser_import_alias_only() {
    let source = "(import Lib :as L :only [helper extra])";
    let rust_program = lsharp_syntax::parse(source)
        .expect("Rust oracle は import :as + :only を parse できるべき");
    match &rust_program.decls[0] {
        lsharp_syntax::ast::Decl::ImportDecl {
            module,
            alias,
            only,
            open,
            ..
        } => {
            assert_eq!(module, "Lib");
            assert_eq!(alias.as_deref(), Some("L"));
            assert_eq!(only, &Some(vec!["helper".to_string(), "extra".to_string()]));
            assert!(!open);
        }
        decl => panic!("Rust oracle の複合 import decl が不正: {decl:?}"),
    }

    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = r#"
(defn main []
  (let [src "(import Lib :as L :only [helper extra])"
        program (parse-program src)
        node (vector-get program 0)
        only (vector-get node 5)]
    (do
      (print (vector-length program))
      (print (vector-get node 0))
      (print (if (= (vector-get node 1) (name-hash "Lib" 0 3)) 1 0))
      (print (if (= (vector-length node) 6) 1 0))
      (print (if (= (vector-get node 4) (name-hash "L" 0 1)) 1 0))
      (print (if (= (vector-length only) 2) 1 0))
      (print (if (= (vector-get only 0) (name-hash "helper" 0 6)) 1 0))
      (print (if (= (vector-get only 1) (name-hash "extra" 0 5)) 1 0))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "26", "1", "1", "1", "1", "1", "1"],
        ":as + :only import は alias と選択 symbol hash vector を AST に保持するべき"
    );
}

/// EC-M1-01: selfhost parser が import の :open と :only を同時に AST へ保持すること
#[test]
fn test_e2e_selfhost_parser_import_open_only() {
    let source = "(import Lib :open :only [helper])";
    let rust_program = lsharp_syntax::parse(source)
        .expect("Rust oracle は import :open + :only を parse できるべき");
    match &rust_program.decls[0] {
        lsharp_syntax::ast::Decl::ImportDecl {
            module,
            alias,
            only,
            open,
            ..
        } => {
            assert_eq!(module, "Lib");
            assert_eq!(*alias, None);
            assert_eq!(only, &Some(vec!["helper".to_string()]));
            assert!(*open);
        }
        decl => panic!("Rust oracle の :open + :only import decl が不正: {decl:?}"),
    }

    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = r#"
(defn main []
  (let [src "(import Lib :open :only [helper])"
        program (parse-program src)
        node (vector-get program 0)
        only (vector-get node 5)]
    (do
      (print (vector-length program))
      (print (vector-get node 0))
      (print (if (= (vector-get node 1) (name-hash "Lib" 0 3)) 1 0))
      (print (if (= (vector-length node) 7) 1 0))
      (print (vector-get node 4))
      (print (if (= (vector-length only) 1) 1 0))
      (print (if (= (vector-get only 0) (name-hash "helper" 0 6)) 1 0))
      (print (vector-get node 6))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "26", "1", "1", "0", "1", "1", "1"],
        ":open + :only import は open flag と選択 symbol hash vector を AST に保持するべき"
    );
}
/// TEST-SYNTAX-02c4: 2 import 後の defn は compiler register/compile でも拾える
#[test]
fn test_e2e_selfhost_compiler_sees_defn_after_multiple_imports() {
    let token_ls = selfhost_module("Token.ls").to_string();
    let ast_ls = selfhost_module("AST.ls").to_string();
    let lexer_ls = selfhost_module("Lexer.ls").to_string();
    let parser_ls = selfhost_module("Parser.ls").to_string();
    let ir_ls = selfhost_module("IR.ls").to_string();
    let compiler_ls = selfhost_module("Compiler.ls").to_string();

    let harness = r#"
(defn main []
  (let [src "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] 0)"
        program (parse-program src)
        reg (register-defns program 0 (vector-length program) (ftable-new) 6)
        ftable (vector-get reg 0)
        functions (compile-defn-functions-with-source program 0 (vector-length program) src ftable (ref-new (vector-new 8)) (vector-new 8))]
    (do
      (print (vector-length program))
      (print (ftable-lookup ftable (name-hash "main" 0 4)))
      (print (vector-length functions))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, ir_ls, compiler_ls, harness
    );
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "compiler multiple-import 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "4", "program は top-level 4 node を返すべき");
    assert_eq!(lines[1], "6", "main は func index 6 に登録されるべき");
    assert_eq!(
        lines[2], "1",
        "compiler は defn main を 1 関数として拾うべき"
    );
}

/// TEST-SYNTAX-02c5: fabricated src-decl pairs では multiple import 相当でも functions を落とさない
#[test]
fn test_e2e_selfhost_compiler_mode_pair_pipeline_keeps_functions() {
    let token_ls = selfhost_module("Token.ls").to_string();
    let ast_ls = selfhost_module("AST.ls").to_string();
    let lexer_ls = selfhost_module("Lexer.ls").to_string();
    let parser_ls = selfhost_module("Parser.ls").to_string();
    let ir_ls = selfhost_module("IR.ls").to_string();
    let compiler_ls = selfhost_module("Compiler.ls").to_string();
    let module_resolver_ls = selfhost_module("ModuleResolver.ls").to_string();
    let wasi_backend_ls = selfhost_module("WasiBackend.ls").to_string();
    let wasm_emit_ls = selfhost_module("WasmEmit.ls").to_string();
    let compiler_mode_ls = selfhost_module("CompilerMode.ls").to_string();

    let harness = r#"
(defn main []
  (let [src-main "(module App.Main)\n(import App.CompilerMode)\n(import App.PipelineSmoke)\n(defn main [] 0)"
        src-a "(module App.CompilerMode)\n(defn compile-file-mode [] 1)"
        src-b "(module App.PipelineSmoke)\n(defn run-main-smoke [] 2)"
        pair-a (make-src-decl-pair src-a (parse-program src-a))
        pair-b (make-src-decl-pair src-b (parse-program src-b))
        pair-main (make-src-decl-pair src-main (parse-program src-main))
        pairs1 (vector-push (vector-new 8) pair-a)
        pairs2 (vector-push pairs1 pair-b)
        all-pairs (vector-push pairs2 pair-main)
        n (vector-length all-pairs)
        reg (register-all-pairs all-pairs 0 n (ftable-new) 6)
        ftable (vector-get reg 0)
        functions (compile-all-src-decl-pairs all-pairs 0 n ftable (ref-new (vector-new 8)) (vector-new 8))]
    (do
      (print n)
      (print (ftable-lookup ftable (name-hash "main" 0 4)))
      (print (vector-length functions))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        token_ls,
        ast_ls,
        lexer_ls,
        parser_ls,
        ir_ls,
        compiler_ls,
        module_resolver_ls,
        wasi_backend_ls,
        wasm_emit_ls,
        compiler_mode_ls,
    ) + "\n"
        + harness;
    let output = compile_and_run(&combined);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 3,
        "compiler-mode pair pipeline 出力が不足: {:?}",
        lines
    );
    assert_eq!(lines[0], "3", "fabricated pair 数は 3 であるべき");
    assert_eq!(
        lines[1], "8",
        "main は 3 個目の関数 index 8 に登録されるべき"
    );
    assert_eq!(lines[2], "3", "functions は 3 個保持されるべき");
}

/// TEST-SYNTAX-02c6: CompilerMode は clean hit で src-decl-pair cache を再利用する
#[test]
fn test_e2e_selfhost_compiler_mode_cached_pairs_skip_reparse_on_clean_hit() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_selfhost_compiler_mode_cached_pairs_clean_{}",
        std::process::id()
    ));
    let app_dir = dir.join("src/App");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.Lib)\n(defn main [] (helper))\n",
    )
    .unwrap();
    std::fs::write(
        app_dir.join("Lib.ls"),
        "(module App.Lib)\n(defn helper [] 7)\n",
    )
    .unwrap();

    let token_ls = selfhost_module("Token.ls").to_string();
    let ast_ls = selfhost_module("AST.ls").to_string();
    let lexer_ls = selfhost_module("Lexer.ls").to_string();
    let parser_ls = selfhost_module("Parser.ls").to_string();
    let ir_ls = selfhost_module("IR.ls").to_string();
    let compiler_ls = selfhost_module("Compiler.ls").to_string();
    let module_resolver_ls = selfhost_module("ModuleResolver.ls").to_string();
    let wasi_backend_ls = selfhost_module("WasiBackend.ls").to_string();
    let wasm_emit_ls = selfhost_module("WasmEmit.ls").to_string();
    let compiler_mode_ls = selfhost_module("CompilerMode.ls").to_string();

    let harness = r#"
(defn main []
  (let [path "src/App/Main.ls"
        cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pairs1 (compile-file-pairs-with-cache path cache-ref parse-count-ref)
        count1 (ref-get parse-count-ref)
        pairs2 (compile-file-pairs-with-cache path cache-ref parse-count-ref)
        count2 (ref-get parse-count-ref)
        n2 (vector-length pairs2)
        reg (register-all-pairs pairs2 0 n2 (ftable-new) 7)
        ftable (vector-get reg 0)
        functions (compile-all-src-decl-pairs pairs2 0 n2 ftable (ref-new (vector-new 8)) (vector-new 8))]
    (do
      (print count1)
      (print count2)
      (print n2)
      (print (vector-length functions))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        token_ls,
        ast_ls,
        lexer_ls,
        parser_ls,
        ir_ls,
        compiler_ls,
        module_resolver_ls,
        wasi_backend_ls,
        wasm_emit_ls,
        compiler_mode_ls,
    ) + "\n"
        + harness;
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "compiler-mode cached pairs clean-hit 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "2",
        "初回 compile では Main/Lib の 2 モジュールを parse するべき"
    );
    assert_eq!(lines[1], "2", "clean hit では parse-count が増えないべき");
    assert_eq!(lines[2], "2", "all-pairs は Main/Lib の 2 ペアを返すべき");
    assert_eq!(lines[3], "2", "functions は 2 個保持されるべき");
}

/// TEST-SYNTAX-02c7: CompilerMode は stale import cache entry だけ再 parse する
#[test]
fn test_e2e_selfhost_compiler_mode_cached_pairs_reparse_only_stale_import_entry() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_test_selfhost_compiler_mode_cached_pairs_changed_{}",
        std::process::id()
    ));
    let app_dir = dir.join("src/App");
    std::fs::create_dir_all(&app_dir).unwrap();
    std::fs::write(
        app_dir.join("Main.ls"),
        "(module App.Main)\n(import App.Lib)\n(defn main [] (helper))\n",
    )
    .unwrap();
    std::fs::write(
        app_dir.join("Lib.ls"),
        "(module App.Lib)\n(defn helper [] 7)\n",
    )
    .unwrap();

    let token_ls = selfhost_module("Token.ls").to_string();
    let ast_ls = selfhost_module("AST.ls").to_string();
    let lexer_ls = selfhost_module("Lexer.ls").to_string();
    let parser_ls = selfhost_module("Parser.ls").to_string();
    let ir_ls = selfhost_module("IR.ls").to_string();
    let compiler_ls = selfhost_module("Compiler.ls").to_string();
    let module_resolver_ls = selfhost_module("ModuleResolver.ls").to_string();
    let wasi_backend_ls = selfhost_module("WasiBackend.ls").to_string();
    let wasm_emit_ls = selfhost_module("WasmEmit.ls").to_string();
    let compiler_mode_ls = selfhost_module("CompilerMode.ls").to_string();

    let harness = r#"
(defn main []
  (let [path "src/App/Main.ls"
        cache-ref (ref-new (map-new))
        parse-count-ref (ref-new 0)
        pairs1 (compile-file-pairs-with-cache path cache-ref parse-count-ref)
        count1 (ref-get parse-count-ref)
        stale-pair (make-src-decl-pair "(module App.Lib)\n(defn helper [] 7)\n" (parse-program "(module App.Lib)\n(defn helper [] 7)\n"))
        _ (ref-set cache-ref (map-insert (ref-get cache-ref) (src-decl-cache-key "src/App/Lib.ls") (make-src-decl-cache-entry 0 stale-pair)))
        pairs2 (compile-file-pairs-with-cache path cache-ref parse-count-ref)
        count2 (ref-get parse-count-ref)
        n2 (vector-length pairs2)
        reg (register-all-pairs pairs2 0 n2 (ftable-new) 7)
        ftable (vector-get reg 0)
        functions (compile-all-src-decl-pairs pairs2 0 n2 ftable (ref-new (vector-new 8)) (vector-new 8))]
    (do
      (print count1)
      (print count2)
      (print n2)
      (print (vector-length functions))
      0)))
"#;

    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}\n{}",
        token_ls,
        ast_ls,
        lexer_ls,
        parser_ls,
        ir_ls,
        compiler_ls,
        module_resolver_ls,
        wasi_backend_ls,
        wasm_emit_ls,
        compiler_mode_ls,
    ) + "\n"
        + harness;
    let output = compile_and_run_with_dir(&combined, &dir);
    let _ = std::fs::remove_dir_all(&dir);
    let lines: Vec<&str> = output.trim().lines().collect();

    assert!(
        lines.len() >= 4,
        "compiler-mode cached pairs stale-import 出力が不足: {:?}",
        lines
    );
    assert_eq!(
        lines[0], "2",
        "初回 compile では Main/Lib の 2 モジュールを parse するべき"
    );
    assert_eq!(
        lines[1], "3",
        "stale import cache entry を 1 個だけ差し替えた場合は追加 parse が 1 回だけ増えるべき"
    );
    assert_eq!(lines[2], "2", "all-pairs は Main/Lib の 2 ペアを返すべき");
    assert_eq!(lines[3], "2", "functions は 2 個保持されるべき");
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

/// TEST-SYNTAX-02f1: record literal field scan は 64 要素境界を越えて保持する
#[test]
fn test_e2e_selfhost_parser_record_literal_fields_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let fields = (0..65)
        .map(|index| format!("x{} {}", index, index))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("{{Point {}}}", fields);
    let harness = format!(
        r#"
(defn main []
  (let [record-node (vector-get (parse-program "{}") 0)
        first-value (vector-get (vector-get record-node 4) 1)
        last-field (vector-get record-node 131)
        last-value (vector-get (vector-get record-node 132) 1)]
    (do
      (print (if (= (vector-get record-node 0) (ast-recordlit)) 1 0))
      (print (if (= (vector-get record-node 1) (name-hash "Point" 0 5)) 1 0))
      (print (vector-get record-node 2))
      (print (vector-length record-node))
      (print (if (= (vector-get record-node 3) (name-hash "x0" 0 2)) 1 0))
      (print first-value)
      (print (if (= last-field (name-hash "x64" 0 3)) 1 0))
      (print last-value)
      0)))
"#,
        source
    );

    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["1", "1", "65", "135", "1", "0", "1", "64"],
        "record literal parser は 64 要素を跨いでも field/value layout を保持するべき"
    );
}

/// TEST-SYNTAX-02m5: function raw type expression は 64 要素を越えて保持する
#[test]
fn test_e2e_selfhost_parser_type_fun_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let types = std::iter::repeat("Int").take(65).collect::<Vec<_>>().join(" ");
    let source = format!("(: 0 (-> {}))", types);
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        type-node (vector-get node 2)
        first-param (vector-get type-node 2)
        last-param (vector-get type-node 65)
        return-type (vector-get type-node 66)]
    (do
      (print (vector-get type-node 0))
      (print (vector-get type-node 1))
      (print (vector-length type-node))
      (print (vector-get first-param 0))
      (print (vector-get first-param 1))
      (print (vector-get last-param 0))
      (print (vector-get last-param 1))
      (print (vector-get return-type 0))
      (print (vector-get return-type 1))
      0)))
"#,
        source
    );

    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "62", "64", "67", "60", "73679", "60", "73679", "60", "73679"
        ],
        "function type parser は 64 要素を跨いでも param/return type を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_record_pattern_fields_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-recordpat-fields-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-recordpat-fields-v3").next())
        .expect("Parser.ls に record pattern fields rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-recordpat-fields-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-recordpat-fields-step-64-loop-bounded").next())
        .expect("Parser.ls に record pattern fields step が存在すること");

    assert!(
        source.contains("(defn parse-recordpat-fields-step-64-loop-bounded")
            && source.contains("(defn parse-recordpat-fields-step-64")
            && rooted_body.contains("parse-recordpat-fields-step-64")
            && !step_body.contains(
                "(parse-recordpat-fields-rooted-v3 spans pos-ref src next-result"
            ),
        "record pattern field parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_record_pattern_fields_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let fields = (0..65)
        .map(|index| format!("x{} v{}", index, index))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(match value [{{Point {}}} value])", fields);
    let harness = format!(
        r#"
(defn main []
  (let [match-node (vector-get (parse-program "{}") 0)
        pattern-node (vector-get match-node 3)
        first-child (vector-get pattern-node 3)
        last-field (vector-get pattern-node 130)
        last-child (vector-get pattern-node 131)]
    (do
      (print (vector-get pattern-node 0))
      (print (vector-get pattern-node 1))
      (print (vector-length pattern-node))
      (print (if (= (vector-get pattern-node 2) (name-hash "x0" 0 2)) 1 0))
      (print (vector-get first-child 0))
      (print (if (= last-field (name-hash "x64" 0 3)) 1 0))
      (print (vector-get last-child 0))
      0)))
"#,
        source
    );

    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["44", "65", "134", "1", "41", "1", "41"],
        "record pattern parser は 64 要素を跨いでも field/pattern layout を保持するべき"
    );
}

/// check 用 flatten の再帰 handoff で、次の step と累積 vector を GC root に保持する。
///
/// 大きな current-source graph では loader と compile probe が完走した後、
/// `load-check-program` の flatten 境界だけで native stage0 が落ちるため、
/// 再帰呼び出しの root 契約をソース上で固定する。
#[test]
fn test_e2e_selfhost_check_flatten_recursive_handoff_roots_live_values() {
    let source = selfhost_module("CompilerMode.ls");

    let function_body = |name: &str| {
        let marker = format!("(defn {name} [");
        let start = source
            .find(&marker)
            .unwrap_or_else(|| panic!("CompilerMode.ls に {name} が存在しない"));
        let tail = &source[start..];
        let end = tail.find("\n(defn ").unwrap_or(tail.len());
        &tail[..end]
    };

    for (name, recursive_call, roots) in [
        (
            "append-check-decls",
            "(append-check-decls decls (vector-get step 1) n (vector-get step 2))",
            ["(root_push decls)", "(root_push program)", "(root_push step)"],
        ),
        (
            "append-check-pairs",
            "(append-check-pairs pairs (vector-get step 1) n (vector-get step 2))",
            ["(root_push pairs)", "(root_push program)", "(root_push step)"],
        ),
        (
            "append-check-owner-decls-step-64-loop-bounded",
            "(append-check-owner-decls-step-64-loop-bounded\n",
            ["(root_push owners)", "(root_push owner)", "(root_push step)"],
        ),
    ] {
        let body = function_body(name);
        let call_offset = body
            .find(recursive_call)
            .unwrap_or_else(|| panic!("{name} の再帰 handoff が存在しない"));
        let handoff = &body[..call_offset];
        for root in roots {
            assert!(
                handoff.contains(root),
                "{name} は再帰 handoff 前に {root} を root 化するべき"
            );
        }
    }
}

/// qualified import の source scan は、program 全体を一つの native recursion にしない。
#[test]
fn test_e2e_selfhost_typeinfer_import_source_scan_uses_bounded_chunks() {
    let source = selfhost_module("TypeInfer.ls");
    assert!(
        source.contains("(defn typeinfer-qualify-import-source-step"),
        "TypeInfer.ls は source qualification の一要素 step を持つべき"
    );
    assert!(
        source.contains("(defn typeinfer-qualify-import-source-loop-bounded"),
        "TypeInfer.ls は source qualification の bounded loop を持つべき"
    );
    assert!(
        source.contains("typeinfer-qualify-import-source-loop-bounded")
            && source.contains(" 64))"),
        "source qualification の公開 loop は bounded chunk size 64 を使うべき"
    );
}

/// source metadata pair の二つの文字列は、深い nested let/do を作らず保持する。
#[test]
fn test_e2e_selfhost_parser_source_metadata_pair_uses_flat_root_bindings() {
    let source = selfhost_module("Parser.ls");
    let body = source
        .split("(defn parse-source-metadata-pair-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-source-evidence-int-v3").next())
        .expect("Parser.ls に source metadata pair helper が存在すること");

    assert!(
        body.contains("first-root (root_push first)")
            && body.contains("second-root (root_push second)")
            && body.contains("result (vector-push-pair-rooted-v3"),
        "source metadata pair は first/second root handoff を flat binding で保持するべき"
    );
    assert!(
        !body.contains("(root_push first)\n      (let [second")
            && !body.contains("(root_push second)\n          (let [result"),
        "source metadata pair は native parser の深い nested root handoff を作らないべき"
    );

    let int_body = source
        .split("(defn parse-source-evidence-int-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn advance-if-token-v3").next())
        .expect("Parser.ls に source evidence integer helper が存在すること");
    assert!(
        int_body.contains("advanced (p-advance pos-ref)")
            && !int_body.contains("(do\n        (p-advance pos-ref)"),
        "source evidence integer helper は advance を flat binding で保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_do_expr_collection_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-do-exprs-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-do-exprs-v3").next())
        .expect("Parser.ls に do expression rooted loop が存在すること");

    assert!(
        source.contains("(defn parse-do-expr-step-64-loop-bounded")
            && source.contains("(defn parse-do-expr-step-64")
            && rooted_body.contains("parse-do-expr-step-64")
            && !rooted_body.contains(
                "(parse-do-exprs-rooted-v3 spans pos-ref src next-result (+ count 1))"
            ),
        "do expression parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_do_expr_handoff_balances_root_slots() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-do-exprs-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-do-exprs-v3").next())
        .expect("Parser.ls に do expression rooted loop が存在すること");

    assert!(
        rooted_body.contains("(root_pop)\n                (root_pop)\n                parsed")
            && !rooted_body.contains(
                "(root_pop)\n                (root_pop)\n                (root_pop)\n                parsed"
            ),
        "do expression の chunk handoff は push した step/next-result と同数だけ root を pop するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_let_bindings_use_bounded_collection_and_fold() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-let-rest-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-let-rest-v3").next())
        .expect("Parser.ls に let rest helper が存在すること");

    assert!(
        source.contains("(defn parse-let-binding-step-64-loop-bounded")
            && source.contains("(defn parse-let-fold-step-64-loop-bounded")
            && rooted_body.contains("parse-let-bindings-v3")
            && rooted_body.contains("parse-let-fold-bindings-v3")
            && !rooted_body.contains(
                "(parse-let-rest-rooted-v3 spans pos-ref src)"
            ),
        "let binding parser は collection/fold の bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_constructor_pattern_args_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-constructor-pattern-args-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-constructor-pattern-args-v3").next())
        .expect("Parser.ls に constructor pattern args rooted loop が存在すること");

    assert!(
        source.contains("(defn parse-constructor-pattern-args-step-64-loop-bounded")
            && source.contains("(defn parse-constructor-pattern-args-step-64")
            && rooted_body.contains("parse-constructor-pattern-args-step-64")
            && !rooted_body.contains(
                "(parse-constructor-pattern-args-rooted-v3 spans pos-ref src next-result (+ count 1))"
            ),
        "constructor pattern args parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_match_arms_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-match-arms-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-match-arms-v3").next())
        .expect("Parser.ls に match arms rooted loop が存在すること");

    assert!(
        source.contains("(defn parse-match-arm-step-64-loop-bounded")
            && source.contains("(defn parse-match-arm-step-64")
            && rooted_body.contains("parse-match-arm-step-64")
            && !rooted_body.contains(
                "(parse-match-arms-rooted-v3 spans pos-ref src next-result (+ count 1))"
            ),
        "match arms parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_params_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-params-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-params-v3").next())
        .expect("Parser.ls に params rooted loop が存在すること");

    assert!(
        source.contains("(defn parse-params-step-64-loop-bounded")
            && source.contains("(defn parse-params-step-64")
            && rooted_body.contains("parse-params-step-64")
            && !rooted_body.contains(
                "(parse-params-rooted-v3 spans pos-ref src next-result (+ count 1))"
            ),
        "params parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_signature_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-defn-param-signature-loop-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-param-signature-v3").next())
        .expect("Parser.ls に defn signature loop が存在すること");

    assert!(
        source.contains("(defn parse-defn-param-signature-step-64-loop-bounded")
            && source.contains("(defn parse-defn-param-signature-step-64")
            && rooted_body.contains("parse-defn-param-signature-step-64")
            && !rooted_body.contains(
                "(parse-defn-param-signature-loop-v3 spans (+ idx 1) end src signature)"
            ),
        "defn signature parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_record_literal_fields_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-recordlit-fields-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-recordlit-fields-v3").next())
        .expect("Parser.ls に record literal fields rooted loop が存在すること");

    assert!(
        source.contains("(defn parse-recordlit-fields-step-64-loop-bounded")
            && source.contains("(defn parse-recordlit-fields-step-64")
            && rooted_body.contains("parse-recordlit-fields-step-64")
            && !rooted_body.contains(
                "(parse-recordlit-fields-rooted-v3 spans pos-ref src next-result (+ count 1))"
            ),
        "record literal field parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_type_expr_list_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-type-expr-list-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-expr-list-v3").next())
        .expect("Parser.ls に type expression list rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-type-expr-list-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-expr-list-step-64-loop-bounded").next())
        .expect("Parser.ls に type expression list step が存在すること");

    assert!(
        source.contains("(defn parse-type-expr-list-step-64-loop-bounded")
            && source.contains("(defn parse-type-expr-list-step-64")
            && rooted_body.contains("parse-type-expr-list-step-64")
            && !step_body.contains(
                "(parse-type-expr-list-rooted-v3 spans pos-ref src next-result)"
            ),
        "type expression list parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}
