use super::*;

#[test]
fn native_output_temp_path_is_a_unique_sibling() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_output_temp_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("native output test directory を作成できる");
    let output_path = dir.join("demo");
    let temporary_path =
        native_temp_output_path(&output_path).expect("native output の一時 path を作成できる");

    assert_eq!(temporary_path.parent(), output_path.parent());
    let name = temporary_path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("native temporary output name を取得できる");
    assert!(name.starts_with(".demo.tmp-"));
    assert_ne!(temporary_path, output_path);
    assert!(!temporary_path.exists());

    std::fs::remove_dir_all(&dir).expect("native output test directory を削除できる");
}

#[test]
fn native_codegen_failure_preserves_backend_diagnostic_code() {
    let module = Module {
        functions: vec![],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };
    let output_path = std::env::temp_dir().join(format!(
        "lsharp_native_missing_main_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));

    let error = compile_native_executable(&module, &output_path)
        .expect_err("main がない native module は codegen error になるべき");
    assert!(
        error.to_string().starts_with("[LS4001]"),
        "native backend error code が必要: {error}"
    );
}

#[test]
fn native_link_failure_cleans_temporary_output_before_returning() {
    let dir = std::env::temp_dir().join(format!(
        "lsharp_native_atomic_failure_test_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("native failure test directory を作成できる");
    let output_path = dir.join("destination");
    std::fs::create_dir(&output_path)
        .expect("rename failure 用 destination directory を作成できる");
    let module = Module {
        functions: vec![lsharp_ir::Function {
            name: "main".to_string(),
            params: vec![],
            result: lsharp_ir::IrType::I64,
            locals: vec![],
            body: vec![lsharp_ir::Instruction::I64Const(0)],
            is_export: true,
        }],
        gc_types: vec![],
        imports: vec![],
        globals: vec![],
        string_data: vec![],
    };

    let error = compile_native_executable(&module, &output_path)
        .expect_err("directory destination への atomic replacement は失敗するべき");
    assert!(
        error.to_string().starts_with("[LS4001]"),
        "native backend error code が必要: {error}"
    );
    assert!(error.to_string().contains("atomic replacement"));
    assert!(output_path.is_dir(), "失敗時も既存 destination を壊さない");
    let temporary_outputs = std::fs::read_dir(&dir)
        .expect("native failure test directory を列挙できる")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with(".destination.tmp-"))
        })
        .count();
    assert_eq!(
        temporary_outputs, 0,
        "link failure 後に temporary executable を残さない"
    );
    std::fs::remove_dir_all(&dir).expect("native failure test directory を削除できる");
}
