use super::support::selfhost_project_root;
use std::fs;

fn line_count(path: &std::path::Path) -> usize {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} 読み込み失敗: {error}", path.display()))
        .lines()
        .count()
}

#[test]
fn selfhost_bootstrap_four_layer_source_stays_within_file_size_budget() {
    let root = selfhost_project_root();
    let source = root.join("crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer.rs");
    let lines = line_count(&source);
    assert!(
        lines <= 800,
        "selfhost_bootstrap_four_layer.rs は 500〜800 行の責務単位へ分割すること: {lines} 行"
    );
    let manifest = fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("{} 読み込み失敗: {error}", source.display()));

    let fragment_dir = root.join("crates/lsharp-wasm/tests/e2e/selfhost_bootstrap_four_layer");
    if !fragment_dir.exists() {
        return;
    }
    let mut fragments = fs::read_dir(&fragment_dir)
        .unwrap_or_else(|error| panic!("{} 読み込み失敗: {error}", fragment_dir.display()))
        .map(|entry| entry.expect("fragment entry の読み込みに失敗").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        .collect::<Vec<_>>();
    fragments.sort();
    assert!(!fragments.is_empty(), "fragment directory は空にしないこと");
    let expected_includes = fragments
        .iter()
        .map(|path| {
            format!(
                "include!(\"selfhost_bootstrap_four_layer/{}\");",
                path.file_name()
                    .expect("fragment filename が必要")
                    .to_string_lossy()
            )
        })
        .collect::<Vec<_>>();
    let actual_includes = manifest
        .lines()
        .filter(|line| line.starts_with("include!(\"selfhost_bootstrap_four_layer/"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(
        actual_includes, expected_includes,
        "root manifest は全 fragment を順序通り include すること"
    );
    for fragment in fragments {
        let lines = line_count(&fragment);
        assert!(
            lines <= 800,
            "{} は 800 行以下の責務単位へ分割すること: {lines} 行",
            fragment.display()
        );
    }
}
