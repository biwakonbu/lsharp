use super::*;

#[test]
fn test_offset_to_position_basic() {
    let source = "hello\nworld";
    assert_eq!(offset_to_position(source, 0), Position::new(0, 0));
    assert_eq!(offset_to_position(source, 5), Position::new(0, 5));
    assert_eq!(offset_to_position(source, 6), Position::new(1, 0));
    assert_eq!(offset_to_position(source, 8), Position::new(1, 2));
}

#[test]
fn test_position_to_offset_basic() {
    let source = "hello\nworld";
    assert_eq!(position_to_offset(source, Position::new(0, 0)), Some(0));
    assert_eq!(position_to_offset(source, Position::new(1, 0)), Some(6));
    assert_eq!(position_to_offset(source, Position::new(1, 2)), Some(8));
}

#[test]
fn positions_use_utf16_code_units_for_non_ascii_source() {
    let source = "😀 x\n日本語";

    assert_eq!(offset_to_position(source, 1), Position::new(0, 0));
    assert_eq!(offset_to_position(source, 4), Position::new(0, 2));
    assert_eq!(offset_to_position(source, 6), Position::new(0, 4));
    assert_eq!(offset_to_position(source, 16), Position::new(1, 3));

    assert_eq!(position_to_offset(source, Position::new(0, 1)), Some(0));
    assert_eq!(position_to_offset(source, Position::new(0, 2)), Some(4));
    assert_eq!(position_to_offset(source, Position::new(0, 3)), Some(5));
    assert_eq!(position_to_offset(source, Position::new(1, 3)), Some(16));
}

#[test]
fn test_symbol_at_position_basic() {
    let source = "(defn add [x y] (+ x y))";
    // "add" は offset 6 から
    assert_eq!(symbol_at_position(source, 6), Some("add".to_string()));
    // "(" はシンボルではない
    assert_eq!(symbol_at_position(source, 0), None);
}

#[test]
fn test_symbol_range_at_position_basic() {
    let source = "(defn add [x y] (+ x y))";
    let result = symbol_range_at_position(source, 6);
    assert!(result.is_some());
    let (name, start, end) = result.unwrap();
    assert_eq!(name, "add");
    assert_eq!(&source[start..end], "add");
}

#[test]
fn test_collect_usages_basic() {
    let source = "(defn f [x] (+ x x))";
    let program = lsharp_syntax::parse(source).unwrap();
    let usages = collect_usages(&program);
    // "+" と "x" の使用が収集される
    let x_usages: Vec<_> = usages.iter().filter(|u| u.name == "x").collect();
    assert_eq!(x_usages.len(), 2, "x は 2 箇所で使用されるべき");
}

#[test]
fn syntax_diagnostics_expose_stable_code_and_source_range() {
    let diagnostics = parse_only("(unknown-form)");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(NumberOrString::String("LS0103".to_string()))
    );
    assert_eq!(
        diagnostics[0].range,
        Range::new(Position::new(0, 1), Position::new(0, 13))
    );
}

#[test]
fn type_diagnostics_expose_stable_code_and_non_empty_source_range() {
    let diagnostics = parse_and_check("(defn bad [] (+ 1 true))");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(NumberOrString::String("LS1004".to_string()))
    );
    assert_ne!(diagnostics[0].range, Range::default());
}

#[test]
fn incremental_type_diagnostics_forward_stable_code_and_source_range() {
    let mut cache = lsharp_ir::CompilationCache::new();
    let diagnostics = parse_and_check_incremental("Main", "(defn bad [] (+ 1 true))", &mut cache);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(NumberOrString::String("LS1004".to_string()))
    );
    assert_ne!(diagnostics[0].range, Range::default());
}

#[test]
fn incremental_module_diagnostics_forward_stable_code() {
    use std::collections::HashMap;

    let dir = std::env::temp_dir().join(format!(
        "lsharp_lsp_incremental_module_diagnostic_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let entry = dir.join("Main.ls");
    let source = "(module Main)\n(import Missing)\n(defn main [] 1)\n";
    std::fs::write(&entry, source).unwrap();

    let uri = Url::from_file_path(&entry).expect("entry path は file URI へ変換できるべき");
    let mut overrides = HashMap::new();
    overrides.insert(entry.clone(), source.to_string());
    let mut cache = lsharp_ir::CompilationCache::new();
    let diagnostics = parse_and_check_uri_incremental(&uri, source, &overrides, &mut cache);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(NumberOrString::String("LS3102".to_string()))
    );
    assert!(diagnostics[0].message.contains("Missing"));
    assert_eq!(
        diagnostics[0].range,
        Range::new(Position::new(1, 0), Position::new(1, 16))
    );

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn incremental_module_type_diagnostics_forward_stable_code() {
    use std::collections::HashMap;

    let dir = std::env::temp_dir().join(format!(
        "lsharp_lsp_incremental_module_type_diagnostic_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let entry = dir.join("Main.ls");
    let helper = dir.join("Helpers.ls");
    let main_source = "(module Main)\n(import Helpers)\n(defn main [] (+ (helper) 1))\n";
    std::fs::write(&entry, main_source).unwrap();
    std::fs::write(&helper, "(module Helpers)\n(defn helper [] true)\n").unwrap();

    let uri = Url::from_file_path(&entry).expect("entry path は file URI へ変換できるべき");
    let mut overrides = HashMap::new();
    overrides.insert(entry.clone(), main_source.to_string());
    let mut cache = lsharp_ir::CompilationCache::new();
    let diagnostics = parse_and_check_uri_incremental(&uri, main_source, &overrides, &mut cache);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(NumberOrString::String("LS1004".to_string()))
    );
    assert!(diagnostics[0].message.contains("Main.ls"));

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn incremental_module_parse_diagnostics_forward_stable_code() {
    use std::collections::HashMap;

    let dir = std::env::temp_dir().join(format!(
        "lsharp_lsp_incremental_module_parse_diagnostic_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock は unix epoch より後であるべき")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let entry = dir.join("Main.ls");
    let helper = dir.join("Helpers.ls");
    let main_source = "(module Main)\n(import Helpers)\n(defn main []";
    std::fs::write(&entry, main_source).unwrap();
    std::fs::write(&helper, "(module Helpers)\n(defn helper [] 1)\n").unwrap();

    let uri = Url::from_file_path(&entry).expect("entry path は file URI へ変換できるべき");
    let mut overrides = HashMap::new();
    overrides.insert(entry.clone(), main_source.to_string());
    let mut cache = lsharp_ir::CompilationCache::new();
    let diagnostics = parse_and_check_uri_incremental(&uri, main_source, &overrides, &mut cache);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(
        diagnostics[0].code,
        Some(NumberOrString::String("LS0101".to_string()))
    );

    std::fs::remove_dir_all(&dir).unwrap();
}
