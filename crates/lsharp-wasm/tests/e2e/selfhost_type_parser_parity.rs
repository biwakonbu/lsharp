use super::support::*;
use lsharp_syntax::ast::{Decl, Expr, Literal, TypeExpr};
use lsharp_syntax::lexer::Lexer;
use lsharp_syntax::metadata::MetadataFormKind;

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

/// qualified import の declaration scan 自体も、program 全体の native recursion にしない。
#[test]
fn test_e2e_selfhost_typeinfer_import_qualification_with_open_uses_bounded_chunks() {
    let source = selfhost_module("TypeInfer.ls");
    let rooted_body = source
        .split("(defn typeinfer-qualify-imports-with-open-rooted-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn typeinfer-predeclare-qualified-imports")
                .next()
        })
        .expect("TypeInfer.ls に import qualification rooted loop が存在すること");
    let step_body = source
        .split("(defn typeinfer-qualify-imports-with-open-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn typeinfer-qualify-imports-with-open-step-64-loop-bounded")
                .next()
        })
        .expect("TypeInfer.ls に import qualification step helper が存在すること");

    assert!(
        source.contains("(defn typeinfer-qualify-imports-with-open-step-64-loop-bounded")
            && source.contains("(defn typeinfer-qualify-imports-with-open-step-64")
            && rooted_body.contains("typeinfer-qualify-imports-with-open-step-64")
            && !step_body.contains("typeinfer-qualify-imports-with-open-rooted-v3"),
        "import qualification outer loop は 64 declaration handoff の bounded helper と rooted continuation を使うべき"
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
fn test_e2e_selfhost_parser_defn_param_form_end_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn scan-defn-param-form-end-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn scan-defn-param-form-end-v3").next())
        .expect("Parser.ls に defn param form end rooted loop が存在すること");
    let step_body = source
        .split("(defn scan-defn-param-form-end-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn scan-defn-param-form-end-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に defn param form end step helper が存在すること");

    assert!(
        source.contains("(defn scan-defn-param-form-end-step-64-loop-bounded")
            && source.contains("(defn scan-defn-param-form-end-step-64")
            && rooted_body.contains("scan-defn-param-form-end-step-64")
            && !step_body.contains("scan-defn-param-form-end-rooted-v3"),
        "defn parameter form end scan は Linux x86 native stack の深い再帰を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_param_form_end_cross_chunk_boundary() {
    let nested_type = (0..33).fold("Int".to_string(), |type_expr, _| {
        format!("(Ref {})", type_expr)
    });
    let typed_params = std::iter::once(format!("(: deep {})", nested_type))
        .chain((1..65).map(|index| format!("(: p{} Int)", index)))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(defn wide [{}] : Int deep)", typed_params);

    let program = lsharp_syntax::parse(&source).expect("Rust oracle は nested typed parameter を parse できるべき");
    let Decl::Defn {
        params,
        return_ty: Some(return_ty),
        ..
    } = &program.decls[0]
    else {
        panic!("Rust oracle の先頭 declaration は typed defn であるべき");
    };
    assert_eq!(params.len(), 65);
    assert!(params.iter().all(|param| param.ty.is_some()));

    let mut nested = params[0].ty.as_ref().expect("先頭 param の型が存在するべき");
    for depth in 0..33 {
        let TypeExpr::App(_, head, args) = nested else {
            panic!("nested Ref の {} 層目が TypeApp であるべき", depth);
        };
        assert!(matches!(head.as_ref(), TypeExpr::Named(_, name) if name == "Ref"));
        assert_eq!(args.len(), 1);
        nested = &args[0];
    }
    assert!(matches!(nested, TypeExpr::Named(_, name) if name == "Int"));
    assert!(matches!(return_ty, TypeExpr::Named(_, name) if name == "Int"));

    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn type-depth [type-expr]
  (if (= (vector-get type-expr 0) (ast-type-app))
    (+ 1 (type-depth (vector-get type-expr 3)))
    0))

(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        signature (vector-get node 69)
        first-param-type (vector-get signature 2)
        return-type (vector-get signature 67)]
    (do
      (print (vector-get node 2))
      (print (vector-get signature 0))
      (print (vector-get signature 1))
      (print (vector-length signature))
      (print (type-depth first-param-type))
      (print (vector-get first-param-type 1))
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
        ["65", "65", "65", "68", "33", "82035", "60", "73679"],
        "defn parameter form end scan は 64 token 境界を跨いでも typed signature と nested TypeApp を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_bracket_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-skip-bracket-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-skip-bracket-v3").next())
        .expect("Parser.ls に bracket skip rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-skip-bracket-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-skip-bracket-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に bracket skip step helper が存在すること");

    assert!(
        source.contains("(defn parse-skip-bracket-step-64-loop-bounded")
            && source.contains("(defn parse-skip-bracket-step-64")
            && rooted_body.contains("parse-skip-bracket-step-64")
            && !step_body.contains("parse-skip-bracket-rooted-v3"),
        "bracket skip parser は Linux x86 native stack の長い bracket payload を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_bracket_cross_chunk_boundary() {
    let expressions = (0..65)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(defn verify [] :example [{}] true)", expressions);
    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は nested bracket example fixture を parse できるべき");
    match &rust_program.decls[0] {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        } => match &metadata.forms[0].kind {
            MetadataFormKind::LegacyExample { expressions } => {
                assert_eq!(expressions.len(), 65);
            }
            form => panic!("Rust oracle の :example form が不正: {form:?}"),
        },
        decl => panic!("Rust oracle の defn metadata が不正: {decl:?}"),
    }

    let token_source = format!("{}0{} 99", "[".repeat(65), "]".repeat(65));
    let trailing_start = token_source.rfind("99").unwrap();
    let trailing_end = trailing_start + 2;
    let source_literal = token_source.replace('"', "\\\"");
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn main []
  (let [source "{}"
        spans (tokenize-with-spans source)
        pos-ref (ref-new 1)]
    (do
      (parse-skip-bracket-v3 spans pos-ref 1)
      (print (ref-get pos-ref))
      (print (p-start spans pos-ref))
      (print (p-end spans pos-ref))
      0)))
"#,
        source_literal
    );
    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["131", &trailing_start.to_string(), &trailing_end.to_string()],
        "bracket skip parser は 64 token 境界を跨いでも nested depth を保持し trailing token へ着地するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_brace_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-skip-brace-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-skip-brace-v3").next())
        .expect("Parser.ls に brace skip rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-skip-brace-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-skip-brace-step-64-loop-bounded").next())
        .expect("Parser.ls に brace skip step helper が存在すること");

    assert!(
        source.contains("(defn parse-skip-brace-step-64-loop-bounded")
            && source.contains("(defn parse-skip-brace-step-64")
            && rooted_body.contains("parse-skip-brace-step-64")
            && step_body.contains("(== kind 99)")
            && !step_body.contains("parse-skip-brace-rooted-v3"),
        "brace skip parser は Linux x86 native stack の長い brace payload を bounded chunk と EOF guard へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_brace_cross_chunk_boundary() {
    let token_source = format!("{}0{} 99", "{".repeat(65), "}".repeat(65));
    let trailing_start = token_source.rfind("99").unwrap();
    let trailing_end = trailing_start + 2;
    let source_literal = token_source.replace('"', "\\\"");
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn main []
  (let [source "{}"
        spans (tokenize-with-spans source)
        pos-ref (ref-new 1)]
    (do
      (parse-skip-brace-v3 spans pos-ref 1)
      (print (ref-get pos-ref))
      (print (p-start spans pos-ref))
      (print (p-end spans pos-ref))
      0)))
"#,
        source_literal
    );
    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["131", &trailing_start.to_string(), &trailing_end.to_string()],
        "brace skip parser は 64 token 境界を跨いでも nested depth を保持し trailing token へ着地するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_to_close_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-skip-to-close-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-skip-to-close-v3").next())
        .expect("Parser.ls に paren skip rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-skip-to-close-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-skip-to-close-step-64-loop-bounded").next())
        .expect("Parser.ls に paren skip step helper が存在すること");

    assert!(
        source.contains("(defn parse-skip-to-close-step-64-loop-bounded")
            && source.contains("(defn parse-skip-to-close-step-64")
            && rooted_body.contains("parse-skip-to-close-step-64")
            && step_body.contains("(== kind 99)")
            && !step_body.contains("parse-skip-to-close-rooted-v3"),
        "paren skip parser は Linux x86 native stack の長い paren payload を bounded chunk と EOF guard へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_to_close_cross_chunk_boundary() {
    let token_source = format!("{}0{} 99", "(".repeat(65), ")".repeat(65));
    let trailing_start = token_source.rfind("99").unwrap();
    let trailing_end = trailing_start + 2;
    let source_literal = token_source.replace('"', "\\\"");
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn main []
  (let [source "{}"
        spans (tokenize-with-spans source)
        pos-ref (ref-new 1)]
    (do
      (parse-skip-to-close-v3 spans pos-ref 1)
      (print (ref-get pos-ref))
      (print (p-start spans pos-ref))
      (print (p-end spans pos-ref))
      0)))
"#,
        source_literal
    );
    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["131", &trailing_start.to_string(), &trailing_end.to_string()],
        "paren skip parser は 64 token 境界を跨いでも nested depth を保持し trailing token へ着地するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_optional_metadata_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn skip-optional-metadata-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn skip-optional-metadata-v3").next())
        .expect("Parser.ls に optional metadata rooted loop が存在すること");
    let step_body = source
        .split("(defn skip-optional-metadata-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn skip-optional-metadata-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に optional metadata step helper が存在すること");

    assert!(
        source.contains("(defn skip-optional-metadata-step-64-loop-bounded")
            && source.contains("(defn skip-optional-metadata-step-64")
            && rooted_body.contains("skip-optional-metadata-step-64")
            && !step_body.contains("skip-optional-metadata-rooted-v3"),
        "optional metadata parser は Linux x86 native stack の長い directive 列を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_skip_optional_metadata_cross_chunk_boundary() {
    let token_source = format!("0 {}99", ":doc \"0\" ".repeat(65));
    let trailing_start = token_source.rfind("99").unwrap();
    let trailing_end = trailing_start + 2;
    let source_literal = token_source.replace('"', "\\\"");
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn main []
  (let [source "{}"
        spans (tokenize-with-spans source)
        pos-ref (ref-new 1)]
    (do
      (skip-optional-metadata-v3 spans pos-ref source)
      (print (ref-get pos-ref))
      (print (p-start spans pos-ref))
      (print (p-end spans pos-ref))
      0)))
"#,
        source_literal
    );
    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["196", &trailing_start.to_string(), &trailing_end.to_string()],
        "optional metadata parser は 64 directive 境界を跨いでも trailing token へ着地するべき"
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
fn test_e2e_selfhost_parser_record_decl_fields_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-record-decl-fields-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-record-decl-fields-v3").next())
        .expect("Parser.ls に record declaration fields rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-record-decl-fields-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-record-decl-fields-step-64-loop-bounded").next())
        .expect("Parser.ls に record declaration fields step が存在すること");

    assert!(
        source.contains("(defn parse-record-decl-fields-step-64-loop-bounded")
            && source.contains("(defn parse-record-decl-fields-step-64")
            && rooted_body.contains("parse-record-decl-fields-step-64")
            && !step_body.contains(
                "(parse-record-decl-fields-rooted-v3 spans pos-ref src record-name-hash"
            ),
        "record declaration field parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_record_decl_fields_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let fields = (0..65)
        .map(|index| format!("(: f{} Int)", index))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(type Wide (record {}))", fields);
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        fields (vector-get node 2)
        first-type (vector-get fields 2)
        last-field (vector-get fields 192)
        last-accessor (vector-get fields 193)
        last-type (vector-get fields 194)]
    (do
      (print (vector-get node 0))
      (print (vector-length fields))
      (print (if (= (vector-get fields 0) (name-hash "f0" 0 2)) 1 0))
      (print (vector-get first-type 0))
      (print (if (= last-field (name-hash "f64" 0 3)) 1 0))
      (print (if (= last-accessor (name-hash "Wide.f64" 0 8)) 1 0))
      (print (vector-get last-type 0))
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
        ["22", "195", "1", "60", "1", "1", "60"],
        "record declaration parser は 64 要素を跨いでも field/accessor/type layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_type_variants_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let field_rooted_body = source
        .split("(defn parse-type-variant-fields-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-variant-fields-v3").next())
        .expect("Parser.ls に type variant field rooted loop が存在すること");
    let field_step_body = source
        .split("(defn parse-type-variant-fields-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-variant-fields-step-64-loop-bounded").next())
        .expect("Parser.ls に type variant field step が存在すること");
    let variants_rooted_body = source
        .split("(defn parse-type-variants-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-variants-v3").next())
        .expect("Parser.ls に type variants rooted loop が存在すること");
    let variants_step_body = source
        .split("(defn parse-type-variants-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-variants-step-64-loop-bounded").next())
        .expect("Parser.ls に type variants step が存在すること");

    assert!(
        source.contains("(defn parse-type-variant-fields-step-64-loop-bounded")
            && source.contains("(defn parse-type-variant-fields-step-64")
            && field_rooted_body.contains("parse-type-variant-fields-step-64")
            && !field_step_body.contains(
                "(parse-type-variant-fields-rooted-v3 spans pos-ref src next-fields",
            )
            && source.contains("(defn parse-type-variants-step-64-loop-bounded")
            && source.contains("(defn parse-type-variants-step-64")
            && variants_rooted_body.contains("parse-type-variants-step-64")
            && !variants_step_body.contains(
                "(parse-type-variants-rooted-v3 spans pos-ref src next-variants",
            ),
        "ADT variant/field parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_type_variants_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let fields = (0..65).map(|_| "Int").collect::<Vec<_>>().join(" ");
    let variants = (1..65)
        .map(|index| format!("V{}", index))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(type Wide (V0 {}) {})", fields, variants);
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        variants (vector-get node 2)
        first-variant (vector-get variants 0)
        first-fields (vector-get first-variant 1)
        last-variant (vector-get variants 64)]
    (do
      (print (vector-get node 0))
      (print (vector-length variants))
      (print (if (= (vector-get first-variant 0) (name-hash "V0" 0 2)) 1 0))
      (print (vector-length first-fields))
      (print (vector-get (vector-get first-fields 64) 0))
      (print (if (= (vector-get last-variant 0) (name-hash "V64" 0 3)) 1 0))
      (print (vector-length (vector-get last-variant 1)))
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
        ["21", "65", "1", "65", "60", "1", "0"],
        "ADT variant/field parser は 64 要素境界を跨いでも variant と field type layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_type_alias_params_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-type-alias-param-hashes-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-alias-param-hashes-v3").next())
        .expect("Parser.ls に type-alias parameter rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-type-alias-param-hashes-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-type-alias-param-hashes-step-64-loop-bounded").next())
        .expect("Parser.ls に type-alias parameter step が存在すること");

    assert!(
        source.contains("(defn parse-type-alias-param-hashes-step-64-loop-bounded")
            && source.contains("(defn parse-type-alias-param-hashes-step-64")
            && rooted_body.contains("parse-type-alias-param-hashes-step-64")
            && !step_body.contains(
                "(parse-type-alias-param-hashes-rooted-v3 spans pos-ref src next-params",
            ),
        "type-alias parameter parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_type_alias_params_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let params = (0..65)
        .map(|index| format!("p{}", index))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(type-alias (Wide {}) p0)", params);
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        params (vector-get node 2)
        target (vector-get node 3)]
    (do
      (print (vector-length node))
      (print (vector-length params))
      (print (if (= (vector-get params 0) (name-hash "p0" 0 2)) 1 0))
      (print (if (= (vector-get params 64) (name-hash "p64" 0 3)) 1 0))
      (print (vector-get target 0))
      (print (if (= (vector-get target 1) (name-hash "p0" 0 2)) 1 0))
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
        ["4", "65", "1", "1", "63", "1"],
        "parametric type-alias parser は 64 要素境界を跨いでも parameter と target layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_computation_steps_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-computation-steps-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-computation-steps-v3").next())
        .expect("Parser.ls に computation steps rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-computation-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-computation-steps-rooted-v3").next())
        .expect("Parser.ls に computation step helper が存在すること");

    assert!(
        source.contains("(defn parse-computation-step-64-loop-bounded")
            && source.contains("(defn parse-computation-step-64")
            && rooted_body.contains("parse-computation-step-64")
            && !step_body.contains(
                "(parse-computation-steps-rooted-v3 spans pos-ref src next-result",
            ),
        "computation step parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_computation_steps_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let steps = (0..65)
        .map(|index| format!("value{}", index))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(computation maybe-builder {})", steps);
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        first-expr (vector-get node 5)
        last-kind (vector-get node 195)
        last-expr (vector-get node 197)]
    (do
      (print (if (= (vector-get node 0) (ast-computation)) 1 0))
      (print (vector-get node 2))
      (print (vector-length node))
      (print (if (= (vector-get node 3) (computation-step-expr)) 1 0))
      (print (if (= (vector-get first-expr 0) (ast-var)) 1 0))
      (print (if (= (vector-get first-expr 1) (name-hash "value0" 0 6)) 1 0))
      (print (if (= last-kind (computation-step-expr)) 1 0))
      (print (if (= (vector-get last-expr 0) (ast-var)) 1 0))
      (print (if (= (vector-get last-expr 1) (name-hash "value64" 0 7)) 1 0))
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
        ["1", "65", "198", "1", "1", "1", "1", "1", "1"],
        "computation parser は 64 要素境界を跨いでも step/expr layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_import_only_symbols_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-import-only-symbols-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-import-only-symbols-v3").next())
        .expect("Parser.ls に import :only symbols rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-import-only-symbols-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-import-only-symbols-rooted-v3").next())
        .expect("Parser.ls に import :only symbols step helper が存在すること");

    assert!(
        source.contains("(defn parse-import-only-symbols-step-64-loop-bounded")
            && source.contains("(defn parse-import-only-symbols-step-64")
            && rooted_body.contains("parse-import-only-symbols-step-64")
            && !step_body.contains(
                "(parse-import-only-symbols-rooted-v3 spans pos-ref src next-result",
            ),
        "import :only symbol parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_import_only_symbols_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let symbols = (0..65)
        .map(|index| format!("selected{}", index))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(import Lib :only [{}])", symbols);
    let rust_program =
        lsharp_syntax::parse(&source).expect("Rust oracle は 65 symbols の import :only を parse できるべき");
    match &rust_program.decls[0] {
        lsharp_syntax::ast::Decl::ImportDecl { only, .. } => {
            assert_eq!(only.as_ref().map(Vec::len), Some(65));
        }
        decl => panic!("Rust oracle の import decl が不正: {decl:?}"),
    }

    let harness = format!(
        r#"
(defn main []
  (let [program (parse-program "{}")
        node (vector-get program 0)
        only (vector-get node 5)]
    (do
      (print (vector-length program))
      (print (vector-get node 0))
      (print (if (= (vector-get node 1) (name-hash "Lib" 0 3)) 1 0))
      (print (vector-length node))
      (print (vector-length only))
      (print (if (= (vector-get only 0) (name-hash "selected0" 0 9)) 1 0))
      (print (if (= (vector-get only 64) (name-hash "selected64" 0 10)) 1 0))
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
        ["1", "26", "1", "6", "65", "1", "1"],
        "import :only parser は 64 要素境界を跨いでも symbol hash vector と AST layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_import_options_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-import-options-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-import-options-v3").next())
        .unwrap_or("");
    let step_body = source
        .split("(defn parse-import-options-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-import-options-step-64-loop-bounded").next())
        .unwrap_or("");

    assert!(
        source.contains("(defn parse-import-options-step-64-loop-bounded")
            && source.contains("(defn parse-import-options-step-64")
            && rooted_body.contains("parse-import-options-step-64")
            && !step_body.contains("parse-import-options-rooted-v3"),
        "import options parser は Linux x86 native stack の長い option 列を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_import_options_cross_chunk_boundary() {
    let options = (0..65).map(|_| ":open").collect::<Vec<_>>().join(" ");
    let source = format!("(import Lib {})", options);
    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は 65 個の import :open option を parse できるべき");
    match &rust_program.decls[0] {
        lsharp_syntax::ast::Decl::ImportDecl { module, alias, only, open, .. } => {
            assert_eq!(module, "Lib");
            assert_eq!(*alias, None);
            assert_eq!(*only, None);
            assert!(*open);
        }
        decl => panic!("Rust oracle の import options decl が不正: {decl:?}"),
    }

    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)]
    (do
      (print (vector-length node))
      (print (vector-get node 0))
      (print (if (= (vector-get node 1) (name-hash "Lib" 0 3)) 1 0))
      (print (vector-get node 4))
      (print (vector-get node 5))
      (print (vector-get node 6))
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
        ["7", "26", "1", "0", "0", "1"],
        "import options parser は 64 option 境界を跨いでも import AST layout と open flag を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_metadata_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-defn-metadata-loop-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-metadata-loop-v3").next())
        .expect("Parser.ls に defn metadata rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-defn-metadata-step-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-metadata-step-64-loop-bounded").next())
        .expect("Parser.ls に defn metadata step helper が存在すること");

    assert!(
        source.contains("(defn parse-defn-metadata-step-64-loop-bounded")
            && source.contains("(defn parse-defn-metadata-step-64")
            && rooted_body.contains("parse-defn-metadata-outer-64")
            && !step_body.contains("parse-defn-metadata-loop-rooted-v3"),
        "defn metadata parser は Linux x86 native stack の深い再帰を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_metadata_cross_chunk_boundary() {
    let docs = (0..65)
        .map(|index| format!(":doc \"doc-{index}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!(
        "(defn identity [] {docs} true) (defn tail [] false)"
    );
    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は 65 個の defn :doc metadata と後続 declaration を parse できるべき");
    assert_eq!(rust_program.decls.len(), 2);
    match &rust_program.decls[0] {
        Decl::Defn {
            body,
            metadata: Some(metadata),
            ..
        } => {
            assert_eq!(metadata.doc.as_deref(), Some("doc-64"));
            assert!(matches!(body, Expr::Lit(_, Literal::Bool(true))));
        }
        decl => panic!("Rust oracle の先頭 declaration が不正: {decl:?}"),
    }

    let source_literal = source.replace('"', "\\\"");
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn main []
  (let [program (parse-program "{}")
        node (vector-get program 0)
        tail (vector-get program 1)
        body (vector-get node 3)
        meta (vector-get node 4)]
    (do
      (print (vector-length program))
      (print (if (= (vector-get body 0) (ast-lit-bool)) 1 0))
      (print (if (string-eq (vector-get meta 0) "doc-64") 1 0))
      (print (if (= (vector-get tail 1) (name-hash "tail" 0 4)) 1 0))
      (print (if (= (vector-get (vector-get tail 3) 0) (ast-lit-bool)) 1 0))
      0)))
"#,
        source_literal
    );
    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["2", "1", "1", "1", "1"],
        "defn metadata parser は 64 要素境界を跨いでも最後の doc、body、後続 declaration を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_assert_predicates_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-defn-meta-assert-loop-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-meta-assert-loop-v3").next())
        .expect("Parser.ls に defn :assert metadata rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-defn-meta-assert-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-defn-meta-assert-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に defn :assert metadata step helper が存在すること");

    assert!(
        source.contains("(defn parse-defn-meta-assert-step-64-loop-bounded")
            && source.contains("(defn parse-defn-meta-assert-step-64")
            && rooted_body.contains("parse-defn-meta-assert-step-64")
            && !step_body.contains(
                "(parse-defn-meta-assert-loop-rooted-v3 spans pos-ref src next-predicates",
            ),
        "defn :assert metadata parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_assert_predicates_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let predicates = (0..65)
        .map(|_| "true".to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(defn positive [] :assert [{}] true)", predicates);
    let first_start = source.find("true").expect("first predicate が見つかる");
    let first_end = first_start + "true".len();
    let body_marker = source.rfind("] true").expect("defn body marker が見つかる");
    let last_start = source[..body_marker]
        .rfind("true")
        .expect("last predicate が見つかる");
    let last_end = last_start + "true".len();
    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は 65 assert predicates を parse できるべき");
    match &rust_program.decls[0] {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        } => match &metadata.forms[0].kind {
            MetadataFormKind::Assertion { predicates } => {
                assert_eq!(predicates.len(), 65);
            }
            form => panic!("Rust oracle の :assert form が不正: {form:?}"),
        },
        decl => panic!("Rust oracle の defn metadata が不正: {decl:?}"),
    }

    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        meta (vector-get node (- (vector-length node) 1))
        forms (vector-get meta 5)
        assertion (vector-get forms 0)
        predicates (vector-get assertion 1)
        spans (vector-get assertion 4)]
    (do
      (print (vector-length node))
      (print (vector-length forms))
      (print (vector-get assertion 0))
      (print (vector-length predicates))
      (print (if (= (vector-get (vector-get predicates 0) 0) (ast-lit-bool)) 1 0))
      (print (if (= (vector-get (vector-get predicates 64) 0) (ast-lit-bool)) 1 0))
      (print (vector-length spans))
      (print (vector-get spans 0))
      (print (vector-get spans 1))
      (print (vector-get spans 128))
      (print (vector-get spans 129))
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
            "5",
            "1",
            "3",
            "65",
            "1",
            "1",
            "130",
            &first_start.to_string(),
            &first_end.to_string(),
            &last_start.to_string(),
            &last_end.to_string(),
        ],
        "defn :assert parser は 64 要素境界を跨いでも predicate と source span layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_case_expectations_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-defn-meta-case-loop-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-meta-case-loop-v3").next())
        .expect("Parser.ls に defn :case metadata rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-defn-meta-case-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-defn-meta-case-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に defn :case metadata step helper が存在すること");

    assert!(
        source.contains("(defn parse-defn-meta-case-step-64-loop-bounded")
            && source.contains("(defn parse-defn-meta-case-step-64")
            && rooted_body.contains("parse-defn-meta-case-step-64")
            && !step_body.contains(
                "(parse-defn-meta-case-loop-rooted-v3 spans pos-ref src next-expectations",
            ),
        "defn :case metadata parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_defn_case_expectations_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let expectations = (0..65)
        .map(|index| format!("(expect {index} {})", index + 1))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(defn identity [] :case [{}] 0)", expectations);
    let first_start = source
        .find("(expect 0 1)")
        .expect("first case expectation が見つかる");
    let first_end = first_start + "(expect 0 1)".len();
    let first_actual_start = first_start + "(expect ".len();
    let first_actual_end = first_actual_start + "0".len();
    let first_expected_start = first_actual_end + 1;
    let first_expected_end = first_expected_start + "1".len();
    let last_start = source
        .rfind("(expect 64 65)")
        .expect("last case expectation が見つかる");
    let last_end = last_start + "(expect 64 65)".len();
    let last_actual_start = last_start + "(expect ".len();
    let last_actual_end = last_actual_start + "64".len();
    let last_expected_start = last_actual_end + 1;
    let last_expected_end = last_expected_start + "65".len();
    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は 65 case expectations を parse できるべき");
    match &rust_program.decls[0] {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        } => match &metadata.forms[0].kind {
            MetadataFormKind::Case { expectations } => {
                assert_eq!(expectations.len(), 65);
                assert_eq!(expectations[0].source_span().start, first_start);
                assert_eq!(expectations[0].source_span().end, first_end);
                assert_eq!(expectations[0].actual().span().start, first_actual_start);
                assert_eq!(expectations[0].actual().span().end, first_actual_end);
                assert_eq!(expectations[0].expected().span().start, first_expected_start);
                assert_eq!(expectations[0].expected().span().end, first_expected_end);
                assert_eq!(expectations[64].source_span().start, last_start);
                assert_eq!(expectations[64].source_span().end, last_end);
                assert_eq!(expectations[64].actual().span().start, last_actual_start);
                assert_eq!(expectations[64].actual().span().end, last_actual_end);
                assert_eq!(expectations[64].expected().span().start, last_expected_start);
                assert_eq!(expectations[64].expected().span().end, last_expected_end);
            }
            form => panic!("Rust oracle の :case form が不正: {form:?}"),
        },
        decl => panic!("Rust oracle の defn metadata が不正: {decl:?}"),
    }

    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        meta (vector-get node (- (vector-length node) 1))
        forms (vector-get meta 5)
        case-form (vector-get forms 0)
        expectations (vector-get case-form 1)
        first (vector-get expectations 0)
        last (vector-get expectations 64)]
    (do
      (print (vector-length node))
      (print (vector-length forms))
      (print (vector-get case-form 0))
      (print (vector-length expectations))
      (print (vector-get (vector-get first 0) 0))
      (print (vector-get (vector-get first 1) 1))
      (print (vector-get (vector-get last 0) 1))
      (print (vector-get (vector-get last 1) 1))
      (print (vector-length first))
      (print (vector-get first 2))
      (print (vector-get first 3))
      (print (vector-get first 4))
      (print (vector-get first 5))
      (print (vector-get first 6))
      (print (vector-get first 7))
      (print (vector-get last 2))
      (print (vector-get last 3))
      (print (vector-get last 4))
      (print (vector-get last 5))
      (print (vector-get last 6))
      (print (vector-get last 7))
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
            "5",
            "1",
            "4",
            "65",
            "1",
            "1",
            "64",
            "65",
            "8",
            &first_start.to_string(),
            &first_end.to_string(),
            &first_actual_start.to_string(),
            &first_actual_end.to_string(),
            &first_expected_start.to_string(),
            &first_expected_end.to_string(),
            &last_start.to_string(),
            &last_end.to_string(),
            &last_actual_start.to_string(),
            &last_actual_end.to_string(),
            &last_expected_start.to_string(),
            &last_expected_end.to_string(),
        ],
        "defn :case parser は 64 要素境界を跨いでも expectation と source span layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_source_evidence_shrinks_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-source-evidence-shrinks-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-source-evidence-shrinks-loop-v3").next())
        .expect("Parser.ls に source evidence shrinks rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-source-evidence-shrinks-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-source-evidence-shrinks-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に source evidence shrinks step helper が存在すること");

    assert!(
        source.contains("(defn parse-source-evidence-shrinks-step-64-loop-bounded")
            && source.contains("(defn parse-source-evidence-shrinks-step-64")
            && rooted_body.contains("parse-source-evidence-shrinks-step-64")
            && !step_body.contains(
                "(parse-source-evidence-shrinks-rooted-v3 spans pos-ref src next-values",
            ),
        "source evidence shrinks parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_source_evidence_shrinks_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let shrinks = (0..65)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!(
        r#"(defn verify []
  :evidence "evidence:bulk"
    :subject "claim:bulk"
    :method "property"
    :outcome "pass"
    :runner "selfhost"
    :target "x86_64-unknown-linux-gnu"
    :source-commit "commit"
    :artifact-digest "sha256:digest"
    :cases 1
    :seed 42
    :generator "bulk"
    :shrinks [{}]
    :producer "lsharp"
    :tool-version "0.2"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)"#,
        shrinks
    );
    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は 65 source evidence shrinks を parse できるべき");
    match &rust_program.decls[0] {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        } => match &metadata.forms[0].kind {
            MetadataFormKind::Evidence { record } => {
                assert_eq!(record.shrinks().len(), 65);
                assert_eq!(record.shrinks().first(), Some(&0));
                assert_eq!(record.shrinks().last(), Some(&64));
            }
            form => panic!("Rust oracle の :evidence form が不正: {form:?}"),
        },
        decl => panic!("Rust oracle の defn metadata が不正: {decl:?}"),
    }

    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        meta (vector-get node (- (vector-length node) 1))
        forms (vector-get meta 5)
        evidence (vector-get forms 0)
        payload (vector-get evidence 1)
        shrinks (vector-get payload 11)]
    (do
      (print (vector-length node))
      (print (vector-length forms))
      (print (vector-get evidence 0))
      (print (vector-length shrinks))
      (print (vector-get shrinks 0))
      (print (vector-get shrinks 64))
      0)))
"#,
        source_literal
    );

    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["5", "1", "15", "65", "0", "64"],
        "source evidence shrinks parser は 64 要素境界を跨いでも sampling vector を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_source_evidence_fields_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-source-evidence-fields-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-source-evidence-fields-loop-v3").next())
        .unwrap_or("");
    let step_body = source
        .split("(defn parse-source-evidence-fields-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-source-evidence-fields-step-64-loop-bounded")
                .next()
        })
        .unwrap_or("");

    assert!(
        source.contains("(defn parse-source-evidence-fields-step-64-loop-bounded")
            && source.contains("(defn parse-source-evidence-fields-step-64")
            && rooted_body.contains("parse-source-evidence-fields-step-64")
            && !step_body.contains("parse-source-evidence-fields-rooted-v3"),
        "source evidence fields parser は Linux x86 native stack の長い field 列を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_source_evidence_fields_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let fields = (0..65)
        .map(|index| format!(":subject \"subject-{}\" ", index))
        .collect::<String>();
    let source = format!("0 {}:unknown \"ignored\" 99", fields);
    let mut rust_lexer = Lexer::new(&source);
    let rust_tokens = rust_lexer
        .tokenize()
        .expect("Rust lexer は evidence fields boundary fixture を tokenize できるべき");
    assert_eq!(
        rust_tokens.len(),
        201,
        "Rust lexer の同一 fixture token 数は 65 field と unknown tail を保持するべき"
    );

    let eof_source = "0 :subject \"subject-eof\"";
    let source_literal = source.replace('"', "\\\"");
    let eof_source_literal = eof_source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [source "{}"
        spans (tokenize-with-spans source)
        pos-ref (ref-new 1)
        payload (parse-source-evidence-fields-loop-v3
          spans pos-ref source (make-empty-source-evidence-payload-v3 "")
        )
        eof-source "{}"
        eof-spans (tokenize-with-spans eof-source)
        eof-pos-ref (ref-new 1)
        eof-payload (parse-source-evidence-fields-loop-v3
          eof-spans eof-pos-ref eof-source
          (make-empty-source-evidence-payload-v3 "")
        )]
    (do
      (print (ref-get pos-ref))
      (print (p-current spans pos-ref))
      (print-string (vector-get payload 1))
      (print-string "\n")
      (print (ref-get eof-pos-ref))
      (print (p-current eof-spans eof-pos-ref))
      (print-string (vector-get eof-payload 1))
      (print-string "\n")
      0)))
"#,
        source_literal, eof_source_literal
    );
    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["196", "50", "subject-64", "4", "99", "subject-eof"],
        "source evidence fields parser は 64 field 境界、unknown field 未消費、EOF 停止、payload 更新を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_source_evidence_coverage_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-source-evidence-coverage-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-source-evidence-coverage-loop-v3").next())
        .expect("Parser.ls に source evidence coverage rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-source-evidence-coverage-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-source-evidence-coverage-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に source evidence coverage step helper が存在すること");

    assert!(
        source.contains("(defn parse-source-evidence-coverage-step-64-loop-bounded")
            && source.contains("(defn parse-source-evidence-coverage-step-64")
            && rooted_body.contains("parse-source-evidence-coverage-step-64")
            && !step_body.contains(
                "(parse-source-evidence-coverage-rooted-v3 spans pos-ref src next-values",
            ),
        "source evidence coverage parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_source_evidence_coverage_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let coverage = (0..65)
        .map(|value| format!("(\"bucket-{}\" {})", value, value))
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!(
        r#"(defn verify []
  :evidence "evidence:bulk"
    :subject "claim:bulk"
    :method "property"
    :outcome "pass"
    :runner "selfhost"
    :target "x86_64-unknown-linux-gnu"
    :source-commit "commit"
    :artifact-digest "sha256:digest"
    :cases 1
    :seed 42
    :generator "bulk"
    :coverage [{}]
    :producer "lsharp"
    :tool-version "0.2"
    :timestamp "2026-07-28T00:00:00Z"
    :independence "same-author"
  true)"#,
        coverage
    );
    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は 65 source evidence coverage buckets を parse できるべき");
    match &rust_program.decls[0] {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        } => match &metadata.forms[0].kind {
            MetadataFormKind::Evidence { record } => {
                assert_eq!(record.coverage().len(), 65);
                assert_eq!(record.coverage().first(), Some(&("bucket-0".to_string(), 0)));
                assert_eq!(record.coverage().last(), Some(&("bucket-64".to_string(), 64)));
            }
            form => panic!("Rust oracle の :evidence form が不正: {form:?}"),
        },
        decl => panic!("Rust oracle の defn metadata が不正: {decl:?}"),
    }

    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        meta (vector-get node (- (vector-length node) 1))
        forms (vector-get meta 5)
        evidence (vector-get forms 0)
        payload (vector-get evidence 1)
        coverage (vector-get payload 12)
        first (vector-get coverage 0)
        last (vector-get coverage 64)]
    (do
      (print (vector-length node))
      (print (vector-length forms))
      (print (vector-get evidence 0))
      (print (vector-length coverage))
      (print-string (vector-get first 0))
      (print-string "\n")
      (print (vector-get first 1))
      (print-string (vector-get last 0))
      (print-string "\n")
      (print (vector-get last 1))
      0)))
"#,
        source_literal
    );

    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["5", "1", "15", "65", "bucket-0", "0", "bucket-64", "64"],
        "source evidence coverage parser は 64 要素境界を跨いでも bucket/count layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_example_expression_spans_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn collect-example-expression-spans-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn collect-example-expression-spans-v3").next())
        .expect("Parser.ls に example expression spans rooted loop が存在すること");
    let step_body = source
        .split("(defn collect-example-expression-spans-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn collect-example-expression-spans-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に example expression spans step helper が存在すること");

    assert!(
        source.contains("(defn collect-example-expression-spans-step-64-loop-bounded")
            && source.contains("(defn collect-example-expression-spans-step-64")
            && rooted_body.contains("collect-example-expression-spans-step-64")
            && !step_body.contains(
                "(collect-example-expression-spans-rooted-v3 spans next-idx end next-result",
            ),
        "example expression spans parser は Linux x86 native stack の深い再帰を避けるため bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_example_expression_spans_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let expressions = (0..65)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let source = format!("(defn verify [] :example [{}] true)", expressions);
    let example_prefix = ":example [";
    let first_start = source.find(example_prefix).unwrap() + example_prefix.len();
    let first_end = first_start + 1;
    let last_start = source.rfind("64").unwrap();
    let last_end = last_start + 2;

    let rust_program = lsharp_syntax::parse(&source)
        .expect("Rust oracle は 65 example expressions を parse できるべき");
    match &rust_program.decls[0] {
        Decl::Defn {
            metadata: Some(metadata),
            ..
        } => match &metadata.forms[0].kind {
            MetadataFormKind::LegacyExample { expressions } => {
                assert_eq!(expressions.len(), 65);
                assert!(matches!(
                    expressions.first(),
                    Some(Expr::Lit(_, Literal::Int(0)))
                ));
                assert!(matches!(
                    expressions.last(),
                    Some(Expr::Lit(_, Literal::Int(64)))
                ));
            }
            form => panic!("Rust oracle の :example form が不正: {form:?}"),
        },
        decl => panic!("Rust oracle の defn metadata が不正: {decl:?}"),
    }

    let source_literal = source.replace('"', "\\\"");
    let harness = format!(
        r#"
(defn main []
  (let [node (vector-get (parse-program "{}") 0)
        meta (vector-get node (- (vector-length node) 1))
        forms (vector-get meta 5)
        form (vector-get forms 0)
        spans (vector-get form 4)]
    (do
      (print (vector-length forms))
      (print (vector-get form 0))
      (print (vector-length form))
      (print (vector-length spans))
      (print (vector-get spans 0))
      (print (vector-get spans 1))
      (print (vector-get spans 128))
      (print (vector-get spans 129))
      0)))
"#,
        source_literal
    );

    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        [
            "1",
            "1",
            "5",
            "130",
            &first_start.to_string(),
            &first_end.to_string(),
            &last_start.to_string(),
            &last_end.to_string(),
        ],
        "example expression spans parser は 64 要素境界を跨いでも expression count と source span layout を保持するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_delimiter_balance_uses_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let rooted_body = source
        .split("(defn parse-delimiter-balance-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-delimiter-diagnostic-code").next())
        .expect("Parser.ls に delimiter balance rooted loop が存在すること");
    let step_body = source
        .split("(defn parse-delimiter-balance-step-v3")
        .nth(1)
        .and_then(|tail| {
            tail.split("(defn parse-delimiter-balance-step-64-loop-bounded")
                .next()
        })
        .expect("Parser.ls に delimiter balance step helper が存在すること");

    assert!(
        source.contains("(defn parse-delimiter-balance-step-64-loop-bounded")
            && source.contains("(defn parse-delimiter-balance-step-64")
            && rooted_body.contains("parse-delimiter-balance-step-64")
            && !step_body.contains(
                "(parse-delimiter-balance-rooted-v3 spans next-idx next-paren-depth",
            ),
        "delimiter balance parser は Linux x86 native stack の長い token 列を bounded chunk へ委譲するべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_delimiter_balance_cross_chunk_boundary() {
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let values = (0..65).map(|_| "true").collect::<Vec<_>>().join(" ");
    let balanced = format!("(defn main [] (do {}))", values);
    let unclosed_paren = format!("(defn main [] (do {})", values);
    let unclosed_bracket = format!("(defn main [] [{}", values);
    let unexpected_then_unclosed = format!(") [ {}", values);

    assert!(
        lsharp_syntax::parse(&balanced).is_ok(),
        "Rust oracle は balanced delimiter fixture を parse できるべき"
    );
    assert!(
        lsharp_syntax::parse(&unclosed_paren).is_err()
            && lsharp_syntax::parse(&unclosed_bracket).is_err()
            && lsharp_syntax::parse(&unexpected_then_unclosed).is_err(),
        "Rust oracle は delimiter failure fixtures を reject するべき"
    );

    let harness = format!(
        r#"
(defn delimiter-code [source]
  (parse-delimiter-diagnostic-code (tokenize-with-spans source)))

(defn main []
  (do
    (print (delimiter-code "{}"))
    (print (delimiter-code "{}"))
    (print (delimiter-code "{}"))
    (print (delimiter-code "{}"))
    0))
"#,
        balanced, unclosed_paren, unclosed_bracket, unexpected_then_unclosed
    );

    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["0", "1001", "1002", "1001"],
        "delimiter balance parser は 64 token 境界を跨いでも balanced/unclosed/first-error code を保持するべき"
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

#[test]
fn test_e2e_selfhost_parser_outer_loops_use_bounded_chunks() {
    let source = selfhost_module("Parser.ls");
    let metadata_rooted = source
        .split("(defn parse-defn-metadata-loop-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-defn-metadata-loop-v3").next())
        .expect("defn metadata outer loop が存在すること");
    let program_rooted = source
        .split("(defn parse-program-loop-rooted-v3")
        .nth(1)
        .and_then(|tail| tail.split("(defn parse-program-loop-v3").next())
        .expect("program outer loop が存在すること");
    assert!(
        source.contains("parse-defn-metadata-outer-64-loop-bounded")
            && source.contains("parse-program-outer-64-loop-bounded")
            && metadata_rooted.contains("parse-defn-metadata-outer-64")
            && program_rooted.contains("parse-program-outer-64"),
        "Parser の外側 continuation は 64 handoff 単位の bounded helper を使うべき"
    );
}

#[test]
fn test_e2e_selfhost_parser_outer_loops_preserve_large_inputs() {
    let docs = (0..4097)
        .map(|index| format!(":doc \"doc-{index}\""))
        .collect::<Vec<_>>()
        .join(" ");
    let metadata_source = format!(
        "(defn identity [] {docs} true) (defn tail [] false)"
    );
    let top_level_source = (0..129)
        .map(|index| format!("(defn f{index} [] {index})"))
        .collect::<Vec<_>>()
        .join(" ");

    let metadata_literal = metadata_source.replace('"', "\\\"");
    let top_level_literal = top_level_source.replace('"', "\\\"");
    let (token_ls, ast_ls, lexer_ls, parser_ls) = parser_runtime_modules();
    let harness = format!(
        r#"
(defn main []
  (let [metadata-program (parse-program "{}")
        metadata-node (vector-get metadata-program 0)
        metadata (vector-get metadata-node 4)
        top-level (parse-program "{}")]
    (do
      (print (vector-length metadata-program))
      (print (if (string-eq (vector-get metadata 0) "doc-4096") 1 0))
      (print (vector-length top-level))
      0)))
"#,
        metadata_literal, top_level_literal
    );
    let output = compile_and_run(&format!(
        "{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, lexer_ls, parser_ls, harness
    ));
    let lines: Vec<&str> = output.trim().lines().collect();

    assert_eq!(
        lines,
        ["2", "1", "129"],
        "Parser の外側 loop は 64 handoff を越えて declaration と metadata の境界を保持するべき"
    );
}
