use super::*;

#[test]
fn test_build_api_doc_includes_metadata_signature_and_return_docs() {
    let source = r#"
(module Geometry)
(defn add
  [x y]
  :doc "2 つの整数を加算する"
  :params [(x "左オペランド") (y "右オペランド")]
  :returns "加算結果"
  :example [(add 1 2)]
  (+ x y))
"#;

    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();

    let api = build_api_doc("demo", "0.1.0", &program, &type_results, &infer);
    let module = api.modules.first().expect("module が必要");
    let function = module.functions.first().expect("function が必要");

    assert_eq!(api.package, "demo");
    assert_eq!(module.name, "Geometry");
    assert_eq!(function.name, "add");
    assert_eq!(function.signature, "Int -> Int -> Int");
    assert_eq!(function.params.len(), 2);
    assert_eq!(function.params[0].doc.as_deref(), Some("左オペランド"));
    assert_eq!(function.returns.doc.as_deref(), Some("加算結果"));
    assert_eq!(function.doc.as_deref(), Some("2 つの整数を加算する"));
    assert_eq!(function.example.as_deref(), Some("(add 1 2)"));
}

#[test]
fn test_build_api_doc_serializes_modules_shape() {
    let source = "(module Sample) (defn main [] 42)";
    let program = lsharp_syntax::parse(source).unwrap();
    let mut infer = Infer::new();
    let type_results = infer.infer_program(&program).unwrap();

    let api = build_api_doc("sample", "0.1.0", &program, &type_results, &infer);
    let json = serde_json::to_string_pretty(&api).unwrap();

    assert!(json.contains("\"package\": \"sample\""));
    assert!(json.contains("\"modules\""));
    assert!(json.contains("\"functions\""));
    assert!(json.contains("\"signature\""));
}

#[test]
fn test_build_api_doc_for_package_collects_modules_from_src_in_sorted_order() {
    let dir = std::env::temp_dir().join("lsharp_api_doc_package");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(dir.join("src/Beta.ls"), "(module Beta)\n(defn beta [] 2)").unwrap();
    std::fs::write(
        dir.join("src/Alpha.ls"),
        "(module Alpha)\n(defn alpha [] 1)",
    )
    .unwrap();

    let api = build_api_doc_for_package(&dir, "demo", "0.1.0").unwrap();
    let names: Vec<&str> = api
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();

    assert_eq!(names, vec!["Alpha", "Beta"]);

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_api_doc_for_file_uses_file_stem_and_header_comment_for_module_metadata() {
    let dir = std::env::temp_dir().join("lsharp_api_doc_module_fallback");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("Sample.ls");
    std::fs::write(
        &file,
        r#";; Sample.ls - 説明
;;
;; モジュール概要
(defn hello
  [name]
  :doc "挨拶を返す"
  :params [(name "対象名")]
  :returns "挨拶文字列"
  :example [(hello "L#")]
  name)
"#,
    )
    .unwrap();

    let api = build_api_doc_for_file("demo", "0.1.0", &file).unwrap();
    let module = api.modules.first().expect("module が必要");

    assert_eq!(module.name, "Sample");
    assert_eq!(module.doc.as_deref(), Some("モジュール概要"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_build_api_doc_for_stdlib_public_functions_have_metadata() {
    let stdlib_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
    let mut public_functions = 0usize;

    for entry in std::fs::read_dir(&stdlib_root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("ls") {
            continue;
        }
        let api = build_api_doc_for_file("stdlib", "0.1.0", &path).unwrap();
        let module = api.modules.first().expect("module が必要");

        assert_ne!(module.name, "Main", "{} の module 名が不正", path.display());
        assert!(
            module.doc.is_some(),
            "{} の module doc が欠けている",
            path.display()
        );

        for function in &module.functions {
            assert_ne!(
                function.name, "main",
                "{} に main が公開 API として出ている",
                module.name
            );
            assert!(
                !function.name.ends_with("-impl"),
                "{}::{} が内部 helper のまま公開されている",
                module.name,
                function.name
            );
            assert!(
                function.doc.is_some(),
                "{}::{} の :doc が欠けている",
                module.name,
                function.name
            );
            assert!(
                function.params.iter().all(|param| param.doc.is_some()),
                "{}::{} の :params が欠けている",
                module.name,
                function.name
            );
            assert!(
                function.returns.doc.is_some(),
                "{}::{} の :returns が欠けている",
                module.name,
                function.name
            );
            assert!(
                function.example.is_some(),
                "{}::{} の :example が欠けている",
                module.name,
                function.name
            );
            public_functions += 1;
        }
    }

    assert!(public_functions >= 40, "stdlib 公開関数数が少なすぎる");
}

#[test]
fn test_build_api_doc_for_file_preserves_parse_error_code() {
    let dir =
        std::env::temp_dir().join(format!("lsharp_api_doc_diagnostic_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("api diagnostic directory を作成できる");
    let file = dir.join("Broken.ls");
    std::fs::write(&file, "(").expect("api diagnostic fixture を書き込める");

    let error = build_api_doc_for_file("demo", "0.1.0", &file)
        .expect_err("壊れた source は API doc 生成を失敗させるべき");
    assert!(
        error.to_string().contains("[LS0103]"),
        "API doc diagnostics は stable code を含むべき: {error}"
    );

    std::fs::remove_dir_all(&dir).expect("api diagnostic directory を削除できる");
}

#[test]
fn test_build_api_doc_for_file_missing_source_preserves_driver_io_error_code() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_api_doc_missing_source_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("api missing source directory を作成できる");
    let file = dir.join("Missing.ls");

    let error = build_api_doc_for_file("demo", "0.1.0", &file)
        .expect_err("存在しない source は API doc 生成を失敗させるべき");
    assert!(
        error.to_string().starts_with("[LS5001]"),
        "API doc file I/O diagnostics は stable code を含むべき: {error}"
    );
    assert!(error.to_string().contains("Missing.ls"));

    std::fs::remove_dir_all(&dir).expect("api missing source directory を削除できる");
}
