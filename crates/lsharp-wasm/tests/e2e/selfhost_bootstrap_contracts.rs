use super::support::*;

// === TEST-BOOT-01-A: Main.ls import-only パイプラインの compile 成功テスト ===

/// Main.ls が import-only パイプラインとして構成されていること、
/// つまりインライン再定義がなく、各モジュール固定 API (Lexer.tokenize,
/// Parser.parse-program) が import 経由で呼ばれていることを検証する。
///
/// 現状の Main.ls はインライン再定義 (mini-tokenize 等) を含んでいるため、
/// import-only 化が完了するまで FAIL する (Red Phase)。
#[test]
fn test_e2e_selfhost_main_import_only_pipeline() {
    let main_source =
        std::fs::read_to_string("../../selfhost/Main.ls").expect("selfhost/Main.ls が読み込めない");

    // 1. 必須 import 宣言の存在確認
    let required_imports = [
        "AST",
        "Lexer",
        "Parser",
        "MacroExpand",
        "TypeInfer",
        "Compiler",
        "WasmEmit",
    ];
    for module in &required_imports {
        assert!(
            main_source.contains(&format!("(import {})", module)),
            "Main.ls に (import {}) がない",
            module
        );
    }

    // 2. インライン再定義がないことを確認
    //    import-only パイプラインでは、各モジュールの関数をインラインで再定義してはいけない。
    //    以下のパターンが Main.ls に存在しないこと:
    let inline_redefinitions = [
        "mini-tokenize",          // Lexer.tokenize を使うべき
        "mini-parse-defn",        // Parser.parse-program を使うべき
        "mini-scan-one",          // Lexer の内部関数
        "mini-scan-loop",         // Lexer の内部関数
        "tok-lparen",             // Token.ls から import すべき
        "ast-lit-int",            // AST.ls から import すべき
        "ir-i64-const",           // IR.ls から import すべき
        "emit-header",            // WasmEmit.ls から import すべき
        "emit-type-section-main", // WasmEmit.ls から import すべき
    ];

    let mut found_redefinitions: Vec<&str> = Vec::new();
    for pattern in &inline_redefinitions {
        // (defn <pattern> で定義されている場合はインライン再定義
        let defn_pattern = format!("(defn {} ", pattern);
        if main_source.contains(&defn_pattern) {
            found_redefinitions.push(pattern);
        }
    }

    assert!(
        found_redefinitions.is_empty(),
        "Main.ls にインライン再定義が残っている (import-only にすべき): {:?}",
        found_redefinitions
    );

    // 3. 各モジュール固定 API が import 経由で呼ばれていること
    //    Lexer.tokenize または tokenize が Main.ls 内で参照されていること
    let api_calls = [
        ("Lexer.tokenize", "tokenize"),
        ("Parser.parse-program", "parse-program"),
    ];

    for (qualified, unqualified) in &api_calls {
        let has_qualified = main_source.contains(qualified);
        let has_unqualified = main_source.contains(&format!("({}", unqualified));
        assert!(
            has_qualified || has_unqualified,
            "Main.ls に {} または {} の呼び出しが見つからない (import 経由の API 呼び出しが必要)",
            qualified,
            unqualified
        );
    }

    // 4. Main.ls がコンパイル可能であること (import 解決はマルチファイル)
    let _wasm = compile_file_only(&selfhost_main_path());
}

// === TEST-BOOT-02-A: MacroExpand.ls direct compile テスト ===

/// MacroExpand.ls を直接コンパイルして成功することを検証する。
///
/// 現状の MacroExpand.ls は hashmap-new, hashmap-set, hashmap-get 等の
/// Rust parser が未対応の構文を含む可能性があるため、
/// 直接コンパイルが成功するまで FAIL する (Red Phase)。
#[test]
fn test_e2e_selfhost_macroexpand_direct_compile() {
    let macroexpand_source = std::fs::read_to_string("../../selfhost/MacroExpand.ls")
        .expect("selfhost/MacroExpand.ls が読み込めない");

    // 1. モジュール宣言の存在確認
    assert!(
        macroexpand_source.contains("(module MacroExpand)"),
        "MacroExpand.ls に (module MacroExpand) 宣言がない"
    );

    // 2. 主要な公開 API 関数の存在確認
    let required_functions = [
        "expand-macros",
        "collect-macros",
        "macro-table-new",
        "expand-node",
        "substitute-node",
        "filter-defmacros",
    ];

    for func in &required_functions {
        let defn_pattern = format!("(defn {} ", func);
        assert!(
            macroexpand_source.contains(&defn_pattern),
            "MacroExpand.ls に必須関数 '{}' の定義がない",
            func
        );
    }

    // 3. MacroExpand.ls を直接コンパイル (フルパイプライン: parse -> infer -> lower -> wasm)
    let wasm_bytes = compile_only(&macroexpand_source);

    // 4. 生成された Wasm バイナリの妥当性検証
    assert_valid_wasm(&wasm_bytes);

    // 5. Wasm バイナリが十分なサイズであること (空やスタブではないこと)
    assert!(
        wasm_bytes.len() > 1000,
        "MacroExpand.ls の Wasm バイナリが小さすぎる: {} bytes (本格的な実装が必要)",
        wasm_bytes.len()
    );
}

// === TEST-TYPE-01: Type/TypeScheme/TypeInfer 責務分離テスト ===

/// selfhost/Type.ls, selfhost/TypeScheme.ls, selfhost/TypeInfer.ls がそれぞれ存在し、
/// 責務が適切に分離されていることを検証する。
///
/// - Type.ls: 型表現 (type representation) のみ -- unify, apply-subst, occurs-check 等は含むが
///   generalize/instantiate/InferState は含まない
/// - TypeScheme.ls: mono/poly/free-type-vars/generalize/instantiate
/// - TypeInfer.ls: InferState + 推論エンジン (infer-expr 等)
///
/// Red Phase: 現状 TypeInfer.ls は unify/apply-subst/generalize 等を重複定義しているため、
/// 責務分離の assert が FAIL する。
#[test]
fn test_e2e_selfhost_type_responsibility_separation() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // 各ファイルの存在確認
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls = std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
        .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_ls = std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
        .expect("selfhost/TypeInfer.ls が読み込めない");

    // === Type.ls の責務: 型表現のみ ===
    // Type.ls には generalize / instantiate が含まれてはいけない
    assert!(
        !type_ls.contains("(defn generalize"),
        "Type.ls に generalize が含まれている: TypeScheme.ls に委譲すべき"
    );
    assert!(
        !type_ls.contains("(defn instantiate"),
        "Type.ls に instantiate が含まれている: TypeScheme.ls に委譲すべき"
    );
    assert!(
        !type_ls.contains("(defn infer-"),
        "Type.ls に infer- 関数が含まれている: TypeInfer.ls に委譲すべき"
    );
    // Type.ls には型構築・アクセス・単一化が含まれるべき
    assert!(
        type_ls.contains("(defn make-type-"),
        "Type.ls に make-type- 関数がない"
    );
    assert!(type_ls.contains("(defn unify"), "Type.ls に unify がない");

    // === TypeScheme.ls の責務: mono/poly/generalize/instantiate ===
    assert!(
        type_scheme_ls.contains("(defn mono"),
        "TypeScheme.ls に mono がない"
    );
    assert!(
        type_scheme_ls.contains("(defn poly"),
        "TypeScheme.ls に poly がない"
    );
    assert!(
        type_scheme_ls.contains("(defn generalize"),
        "TypeScheme.ls に generalize がない"
    );
    assert!(
        type_scheme_ls.contains("(defn instantiate"),
        "TypeScheme.ls に instantiate がない"
    );
    assert!(
        type_scheme_ls.contains("(defn free-vars"),
        "TypeScheme.ls に free-vars がない"
    );
    // TypeScheme.ls には推論エンジンが含まれてはいけない
    assert!(
        !type_scheme_ls.contains("(defn infer-"),
        "TypeScheme.ls に infer- 関数が含まれている: TypeInfer.ls に委譲すべき"
    );

    // === TypeInfer.ls の責務: InferState + 推論エンジン ===
    assert!(
        type_infer_ls.contains("(defn infer-expr"),
        "TypeInfer.ls に infer-expr がない"
    );
    // TypeInfer.ls は Type.ls / TypeScheme.ls を import し、
    // unify/generalize/instantiate 等を再定義していないこと
    assert!(
        type_infer_ls.contains("(import Type)"),
        "TypeInfer.ls が Type.ls を import していない"
    );
    assert!(
        type_infer_ls.contains("(import TypeScheme)"),
        "TypeInfer.ls が TypeScheme.ls を import していない"
    );

    // 重複定義の検出: TypeInfer.ls に unify/apply-subst/generalize が再定義されている場合 FAIL
    // (import しているなら再定義は不要)
    let type_infer_has_unify_redef = type_infer_ls.contains("(defn unify ");
    let type_infer_has_apply_subst_redef = type_infer_ls.contains("(defn apply-subst ");
    let type_infer_has_generalize_redef = type_infer_ls.contains("(defn generalize ");
    let type_infer_has_instantiate_redef = type_infer_ls.contains("(defn instantiate ");

    assert!(
        !type_infer_has_unify_redef,
        "TypeInfer.ls に unify が再定義されている: Type.ls の import で解決すべき"
    );
    assert!(
        !type_infer_has_apply_subst_redef,
        "TypeInfer.ls に apply-subst が再定義されている: Type.ls の import で解決すべき"
    );
    assert!(
        !type_infer_has_generalize_redef,
        "TypeInfer.ls に generalize が再定義されている: TypeScheme.ls の import で解決すべき"
    );
    assert!(
        !type_infer_has_instantiate_redef,
        "TypeInfer.ls に instantiate が再定義されている: TypeScheme.ls の import で解決すべき"
    );
}

// === TEST-SYNTAX-02: Rust AST 全ノード型の 1:1 対応 golden fixture ===

/// Rust の AST ノード型 (Expr/Decl/Pattern enum variants) を列挙し、
/// selfhost/AST.ls に対応する constructor が全て存在することを検証する。
///
/// Golden fixture: tests/golden/syntax/ast_node_map.json
///
/// Red Phase: selfhost/AST.ls は基本的な Expr バリアントのみ実装しているため、
/// 多くのバリアント (Ann, RecordLit, FieldAccess, 等) に対応する constructor がなく FAIL する。
#[test]
fn test_e2e_selfhost_ast_full_coverage() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // golden fixture を読み込む
    let golden_path = project_root.join("tests/golden/syntax/ast_node_map.json");
    assert!(
        golden_path.exists(),
        "tests/golden/syntax/ast_node_map.json が存在しない"
    );
    let golden_content =
        std::fs::read_to_string(&golden_path).expect("ast_node_map.json の読み込みに失敗");
    let golden: serde_json::Value =
        serde_json::from_str(&golden_content).expect("ast_node_map.json の JSON パースに失敗");

    // Rust AST の Expr variant 列挙 (ast.rs から)
    let rust_expr_variants = [
        "Lit",
        "Var",
        "If",
        "Let",
        "Lambda",
        "App",
        "Match",
        "Do",
        "Ann",
        "RecordLit",
        "FieldAccess",
        "RecordUpdate",
        "Computation",
        "Quote",
        "Unquote",
        "UnquoteSplice",
    ];

    // Rust AST の Decl variant 列挙
    let rust_decl_variants = [
        "Defn",
        "TypeDef",
        "RecordDef",
        "TypeAlias",
        "TypeConstrained",
        "ModuleDecl",
        "ImportDecl",
        "TraitDef",
        "ImplDef",
        "Private",
        "ComputationBuilder",
        "DefMacro",
    ];

    // Rust AST の Pattern variant 列挙
    let rust_pattern_variants = ["Wildcard", "Var", "Lit", "Constructor", "RecordPat"];

    // selfhost/AST.ls を読み込む
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");

    // golden fixture の expr_variants と実際の Rust variants が一致すること
    let golden_expr = golden.get("expr_variants").expect("expr_variants がない");
    for variant in &rust_expr_variants {
        assert!(
            golden_expr.get(variant).is_some(),
            "golden fixture に Expr::{} のエントリがない",
            variant
        );
    }

    // golden fixture の decl_variants と実際の Rust variants が一致すること
    let golden_decl = golden.get("decl_variants").expect("decl_variants がない");
    for variant in &rust_decl_variants {
        assert!(
            golden_decl.get(variant).is_some(),
            "golden fixture に Decl::{} のエントリがない",
            variant
        );
    }

    // golden fixture の pattern_variants と実際の Rust variants が一致すること
    let golden_pat = golden
        .get("pattern_variants")
        .expect("pattern_variants がない");
    for variant in &rust_pattern_variants {
        assert!(
            golden_pat.get(variant).is_some(),
            "golden fixture に Pattern::{} のエントリがない",
            variant
        );
    }

    // selfhost/AST.ls に全 Expr variant の constructor が存在すること
    // 各 variant にはタグ定数 (defn ast-xxx) または構築関数 (defn make-xxx) が必要
    let mut missing_expr: Vec<&str> = Vec::new();
    for variant in &rust_expr_variants {
        let variant_lower = variant.to_lowercase();
        // ast-{name} タグ定数 または make-{name} 構築関数 のいずれかが存在するか
        let has_tag = ast_ls.contains(&format!("ast-{}", variant_lower))
            || ast_ls.contains(&format!(
                "ast-{}",
                variant.to_lowercase().replace("splice", "-splice")
            ));
        let has_make = ast_ls.contains(&format!("make-{}", variant_lower))
            || ast_ls.contains(&format!(
                "make-{}",
                variant.to_lowercase().replace("lit", "lit-")
            ));
        if !has_tag && !has_make {
            missing_expr.push(variant);
        }
    }

    assert!(
        missing_expr.is_empty(),
        "selfhost/AST.ls に以下の Expr variant の constructor がない: {:?}\n\
         全 {} variant に対応する ast-xxx タグ定数 or make-xxx 構築関数が必要",
        missing_expr,
        rust_expr_variants.len()
    );

    // selfhost/AST.ls に全 Decl variant の constructor が存在すること
    let mut missing_decl: Vec<&str> = Vec::new();
    for variant in &rust_decl_variants {
        let variant_lower = variant.to_lowercase();
        let has_tag = ast_ls.contains(&format!("ast-{}", variant_lower))
            || ast_ls.contains(&format!("ast-{}", variant_lower.replace("decl", "-decl")));
        let has_make = ast_ls.contains(&format!("make-{}", variant_lower));
        if !has_tag && !has_make {
            missing_decl.push(variant);
        }
    }

    assert!(
        missing_decl.is_empty(),
        "selfhost/AST.ls に以下の Decl variant の constructor がない: {:?}\n\
         全 {} variant に対応する ast-xxx タグ定数 or make-xxx 構築関数が必要",
        missing_decl,
        rust_decl_variants.len()
    );

    // selfhost/AST.ls に全 Pattern variant の constructor が存在すること
    let mut missing_pat: Vec<&str> = Vec::new();
    for variant in &rust_pattern_variants {
        let variant_lower = variant.to_lowercase();
        let has_tag = ast_ls.contains(&format!("ast-pat-{}", variant_lower))
            || ast_ls.contains(&format!("ast-{}", variant_lower));
        let has_make = ast_ls.contains(&format!("make-pat-{}", variant_lower))
            || ast_ls.contains(&format!("make-{}", variant_lower));
        if !has_tag && !has_make {
            missing_pat.push(variant);
        }
    }

    assert!(
        missing_pat.is_empty(),
        "selfhost/AST.ls に以下の Pattern variant の constructor がない: {:?}\n\
         全 {} variant に対応する ast-pat-xxx タグ定数 or make-pat-xxx 構築関数が必要",
        missing_pat,
        rust_pattern_variants.len()
    );
}

// === TEST-TYPE-02: unify/generalize/instantiate の公開挙動 golden テスト ===

/// Rust の TypeInfer::unify/generalize/instantiate の入出力ペアを golden fixture として記録し、
/// selfhost の対応関数が同じ入出力を生成することを検証する準備テスト。
///
/// Golden fixture: tests/golden/types/hm_core.json
///
/// Red Phase: selfhost の TypeInfer.ls を実行して golden fixture の各ケースを検証するが、
/// selfhost モジュール連結コンパイルが Rust 版と完全一致しないため FAIL する。
#[test]
fn test_e2e_selfhost_type_hm_core_golden() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // 1. golden fixture の読み込み
    let golden_path = project_root.join("tests/golden/types/hm_core.json");
    assert!(
        golden_path.exists(),
        "tests/golden/types/hm_core.json が存在しない"
    );
    let golden_content =
        std::fs::read_to_string(&golden_path).expect("hm_core.json の読み込みに失敗");
    let golden: serde_json::Value =
        serde_json::from_str(&golden_content).expect("hm_core.json の JSON パースに失敗");

    // 2. golden fixture の構造検証
    let unify_cases = golden.get("unify").expect("unify セクションがない");
    assert!(unify_cases.is_array(), "unify セクションが配列でない");
    assert!(
        unify_cases.as_array().unwrap().len() >= 5,
        "unify テストケースが 5 件未満: {}",
        unify_cases.as_array().unwrap().len()
    );

    let generalize_cases = golden
        .get("generalize")
        .expect("generalize セクションがない");
    assert!(
        generalize_cases.is_array() && generalize_cases.as_array().unwrap().len() >= 3,
        "generalize テストケースが 3 件未満"
    );

    let instantiate_cases = golden
        .get("instantiate")
        .expect("instantiate セクションがない");
    assert!(
        instantiate_cases.is_array() && instantiate_cases.as_array().unwrap().len() >= 2,
        "instantiate テストケースが 2 件未満"
    );

    // 3. Rust 側の unify はプライベートなので、selfhost 側の動作検証に集中する
    // (Rust 側の unify 公開は TYPE-01 タスクで実施)

    // 4. selfhost Type.ls を実行して同等のケースを検証
    let selfhost_unify_source = r#"
(defn main []
  (let [int1 (vector-push (vector-push (vector-new 2) 1) 100)
        int2 (vector-push (vector-push (vector-new 2) 1) 100)
        result (if (= (vector-get int1 0) (vector-get int2 0))
                 (if (= (vector-get int1 0) 1)
                   (if (= (vector-get int1 1) (vector-get int2 1)) 1 0)
                   0)
                 0)]
    (print result)))
"#;
    let selfhost_output = compile_and_run(selfhost_unify_source);
    assert_eq!(
        selfhost_output.trim(),
        "1",
        "selfhost: Int==Int の型比較が一致しない"
    );

    // 5. selfhost TypeInfer.ls を全依存モジュール連結でコンパイル + 実行
    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("selfhost/Token.ls が読み込めない");
    let ast_ls = std::fs::read_to_string(project_root.join("selfhost/AST.ls"))
        .expect("selfhost/AST.ls が読み込めない");
    let type_ls = std::fs::read_to_string(project_root.join("selfhost/Type.ls"))
        .expect("selfhost/Type.ls が読み込めない");
    let type_scheme_ls = std::fs::read_to_string(project_root.join("selfhost/TypeScheme.ls"))
        .expect("selfhost/TypeScheme.ls が読み込めない");
    let type_infer_core_ls =
        std::fs::read_to_string(project_root.join("selfhost/TypeInferCore.ls"))
            .expect("selfhost/TypeInferCore.ls が読み込めない");
    let type_infer_ls = std::fs::read_to_string(project_root.join("selfhost/TypeInfer.ls"))
        .expect("selfhost/TypeInfer.ls が読み込めない");

    // モジュール連結 (依存順)
    let combined = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        token_ls, ast_ls, type_ls, type_scheme_ls, type_infer_core_ls, type_infer_ls
    );

    // コンパイル + 実行: TypeInfer.ls の main() が golden fixture と同じ結果を出力するか
    let output = compile_and_run(&combined);

    // TypeInfer.ls の main() の期待出力 (golden fixture と対応):
    // テスト 1: result_failed=0, ty_tag=1, ty_name=100 (Int リテラル -> Int)
    // テスト 2: ty_tag=1, ty_name=200 (Bool リテラル -> Bool)
    // テスト 3: result_failed=0, ty_tag=1, ty_name=100 (if true 42 0 -> Int)
    // テスト 4: result_failed=0, ty_tag=1, ty_name=100 (let x=42 in x -> Int)
    // テスト 5: result_failed=0, ty_name=200 (変数 -> Bool)
    // テスト 6: result_failed=1 (未定義変数 -> エラー)
    // テスト 7: result_failed=0, ty_name=200 (do -> Bool)
    // 連結ソースでは **最後**の main (TypeInfer.ls) が実行される
    // (emit_wasm_wasi は複数 defn main があるとき rposition でエントリを選ぶ)
    let expected_lines = [
        "0", "1", "100", "1", "200", "0", "1", "100", "0", "1", "100", "0", "200", "1", "0", "200",
        "1",
    ];

    let output_lines: Vec<&str> = output.lines().collect();
    assert_eq!(
        output_lines.len(),
        expected_lines.len(),
        "selfhost 連結ソースの出力行数が不一致。\n\
         期待: {} 行, 実際: {} 行\n実際の出力:\n{}",
        expected_lines.len(),
        output_lines.len(),
        output
    );

    for (i, (actual, expected)) in output_lines.iter().zip(expected_lines.iter()).enumerate() {
        assert_eq!(
            actual,
            expected,
            "selfhost 連結ソース出力の {} 行目が不一致: 期待='{}', 実際='{}'",
            i + 1,
            expected,
            actual
        );
    }
}

// =============================================================================
// TEST-TYPE-05: MetadataCheck.ls metadata validation
// selfhost/MetadataCheck.ls が存在し、:doc, :params, :returns メタデータの
// validation を行う関数を公開していることを検証
// =============================================================================

#[test]
fn test_e2e_selfhost_metadata_check() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // 1. selfhost/MetadataCheck.ls が存在すること
    let metadata_check_path = project_root.join("selfhost/MetadataCheck.ls");
    assert!(
        metadata_check_path.exists(),
        "selfhost/MetadataCheck.ls が存在しない。\
         メタデータ検証モジュールを作成してください。"
    );

    // 2. ソースを読み込み、必須関数が定義されていることを検証
    let source = std::fs::read_to_string(&metadata_check_path)
        .expect("selfhost/MetadataCheck.ls の読み込みに失敗");

    // module 宣言の確認
    assert!(
        source.contains("(module MetadataCheck)"),
        "MetadataCheck.ls に (module MetadataCheck) 宣言がない"
    );

    // :doc メタデータの validation 関数
    assert!(
        source.contains("validate-doc"),
        "MetadataCheck.ls に validate-doc 関数がない"
    );

    // :params メタデータの validation 関数
    assert!(
        source.contains("validate-params"),
        "MetadataCheck.ls に validate-params 関数がない"
    );

    // :returns メタデータの validation 関数
    assert!(
        source.contains("validate-returns"),
        "MetadataCheck.ls に validate-returns 関数がない"
    );

    // 3. コンパイルが通ること (全依存モジュール連結)
    let token_ls = std::fs::read_to_string(project_root.join("selfhost/Token.ls"))
        .expect("Token.ls 読み込み失敗");
    let ast_ls =
        std::fs::read_to_string(project_root.join("selfhost/AST.ls")).expect("AST.ls 読み込み失敗");
    let span_ls = std::fs::read_to_string(project_root.join("selfhost/Span.ls"))
        .expect("Span.ls 読み込み失敗");

    let combined = format!("{}\n{}\n{}\n{}", token_ls, ast_ls, span_ls, source);

    // パース + 型チェック + コンパイルが通ること
    let program = parse_for_pipeline(&combined);
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();
    let mut lower = Lower::new();
    let module = lower.lower_program(&program, &type_results).unwrap();
    let wasm_bytes = lsharp_wasm::wasi::emit_wasm_wasi(&module).unwrap();
    assert_valid_wasm(&wasm_bytes);
}

// =============================================================================
// TEST-TYPE-06: HKT/GADT/alias/record update
// TypeInfer.ls が HKT, GADT, type alias, record update の最小完了集合を
// 実装していることを検証
// =============================================================================

#[test]
fn test_e2e_selfhost_hkt_gadt_alias_record() {
    let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");

    // selfhost/TypeInfer.ls を読み込み
    let type_infer_path = project_root.join("selfhost/TypeInfer.ls");
    assert!(
        type_infer_path.exists(),
        "selfhost/TypeInfer.ls が存在しない"
    );
    let source =
        std::fs::read_to_string(&type_infer_path).expect("selfhost/TypeInfer.ls の読み込みに失敗");

    // HKT (Higher-Kinded Types) 関連関数
    assert!(
        source.contains("hkt-apply"),
        "TypeInfer.ls に hkt-apply 関数がない。\
         HKT の型適用を実装してください。"
    );

    // GADT (Generalized Algebraic Data Types) 関連関数
    assert!(
        source.contains("gadt-check"),
        "TypeInfer.ls に gadt-check 関数がない。\
         GADT のコンストラクタ型チェックを実装してください。"
    );

    // Type alias 解決関数
    assert!(
        source.contains("resolve-alias"),
        "TypeInfer.ls に resolve-alias 関数がない。\
         型エイリアスの解決を実装してください。"
    );

    // Record update 推論関数
    assert!(
        source.contains("infer-record-update"),
        "TypeInfer.ls に infer-record-update 関数がない。\
         レコード更新の型推論を実装してください。"
    );
}
