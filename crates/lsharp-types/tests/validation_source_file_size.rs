#[test]
fn validation_source_root_stays_within_the_test_file_budget() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("validation_source.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} を読み込めません: {error}", path.display()));
    let lines = source.lines().count();

    assert!(
        lines <= 500,
        "validation_source.rs は責務別 module へ分割し、500 行以下に保つ (actual={lines})"
    );
}
