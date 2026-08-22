//! workspace 全域の Rust file-size gate (`RUST-FILE-SIZE-GATE-01` / Issue `I-01`)。
//!
//! per-file の targeted guard (`*_file_size.rs`) は個別 file の構成しか見ないので、
//! 新しく 800 行を超えた file が黙って増えるのを止められない。ここは
//! `crates/**/src/**` と `crates/**/tests/**` を走査し、allowlist との
//! **差集合が双方向で空**であることを要求する。
//!
//! allowlist は 2 本立てにする。src と tests では超過の規模が桁で違い
//! (2026-08-23 実測で src 6 / tests 33)、1 本にまとめると
//! 「tests を分割したら src の gate も緩んだ」という取り違えが起きる。
//!
//! **allowlist への追加は ADR を要求する。** 単調減少だけが許される。

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_RUST_SOURCE_LINES: usize = 800;
const SOURCE_ALLOWLIST_RELATIVE_PATH: &str = "tests/rust-file-size-allowlist.txt";
const TEST_ALLOWLIST_RELATIVE_PATH: &str = "tests/rust-test-file-size-allowlist.txt";

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|error| {
            panic!(
                "Rust source directory を読めない: {}: {error}",
                directory.display()
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| {
            panic!(
                "Rust source directory entry を読めない: {}: {error}",
                directory.display()
            )
        });
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .unwrap_or_else(|error| panic!("file type を読めない: {}: {error}", path.display()));
        if file_type.is_dir() {
            collect_rust_files(&path, files);
        } else if file_type.is_file() && path.extension().is_some_and(|extension| extension == "rs")
        {
            files.push(path);
        }
    }
}

fn is_crate_source(relative_path: &str) -> bool {
    relative_path.starts_with("crates/") && relative_path.contains("/src/")
}

fn is_crate_test(relative_path: &str) -> bool {
    relative_path.starts_with("crates/") && relative_path.contains("/tests/")
}

fn read_allowlist(
    project_root: &Path,
    allowlist_relative_path: &str,
    accepts_entry: fn(&str) -> bool,
    expected_pattern: &str,
) -> BTreeSet<String> {
    let allowlist_path = project_root.join(allowlist_relative_path);
    let content = fs::read_to_string(&allowlist_path).unwrap_or_else(|error| {
        panic!(
            "Rust file-size allowlist を読めない: {}: {error}",
            allowlist_path.display()
        )
    });
    let mut entries = BTreeSet::new();

    for (index, line) in content.lines().enumerate() {
        let entry = line.trim();
        if entry.is_empty() || entry.starts_with('#') {
            continue;
        }
        assert!(
            accepts_entry(entry) && entry.ends_with(".rs"),
            "{}:{} は {expected_pattern} 形式であるべき: {entry}",
            allowlist_relative_path,
            index + 1
        );
        assert!(
            entries.insert(entry.to_owned()),
            "{}:{} に重複 entry がある: {entry}",
            allowlist_relative_path,
            index + 1
        );
    }

    entries
}

#[test]
fn rust_source_files_over_800_lines_match_allowlist() {
    let project_root = project_root();
    let mut rust_files = Vec::new();
    collect_rust_files(&project_root.join("crates"), &mut rust_files);

    let oversized = rust_files
        .into_iter()
        .filter_map(|path| {
            let relative_path = path
                .strip_prefix(&project_root)
                .expect("Rust source path は project root 配下であるべき")
                .to_string_lossy()
                .replace('\\', "/");
            if !is_crate_source(&relative_path) {
                return None;
            }

            let source = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("Rust source を読めない: {}: {error}", path.display())
            });
            (source.lines().count() > MAX_RUST_SOURCE_LINES).then_some(relative_path)
        })
        .collect::<BTreeSet<_>>();
    let allowlisted = read_allowlist(
        &project_root,
        SOURCE_ALLOWLIST_RELATIVE_PATH,
        is_crate_source,
        "crates/**/src/**/*.rs",
    );

    let newly_oversized = oversized
        .difference(&allowlisted)
        .cloned()
        .collect::<Vec<_>>();
    let stale_allowlist = allowlisted
        .difference(&oversized)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        newly_oversized.is_empty() && stale_allowlist.is_empty(),
        "Rust source file-size contract mismatch (上限: {MAX_RUST_SOURCE_LINES} 行)\n新規超過: {newly_oversized:#?}\n解消済みまたは不正な allowlist: {stale_allowlist:#?}"
    );
}

#[test]
fn rust_test_files_over_800_lines_match_allowlist() {
    let project_root = project_root();
    let mut rust_files = Vec::new();
    collect_rust_files(&project_root.join("crates"), &mut rust_files);

    let oversized = rust_files
        .into_iter()
        .filter_map(|path| {
            let relative_path = path
                .strip_prefix(&project_root)
                .expect("Rust test path は project root 配下であるべき")
                .to_string_lossy()
                .replace('\\', "/");
            if !is_crate_test(&relative_path) {
                return None;
            }

            let source = fs::read_to_string(&path).unwrap_or_else(|error| {
                panic!("Rust test を読めない: {}: {error}", path.display())
            });
            (source.lines().count() > MAX_RUST_SOURCE_LINES).then_some(relative_path)
        })
        .collect::<BTreeSet<_>>();
    let allowlisted = read_allowlist(
        &project_root,
        TEST_ALLOWLIST_RELATIVE_PATH,
        is_crate_test,
        "crates/**/tests/**/*.rs",
    );

    let newly_oversized = oversized
        .difference(&allowlisted)
        .cloned()
        .collect::<Vec<_>>();
    let stale_allowlist = allowlisted
        .difference(&oversized)
        .cloned()
        .collect::<Vec<_>>();

    assert!(
        newly_oversized.is_empty() && stale_allowlist.is_empty(),
        "Rust test file-size contract mismatch (上限: {MAX_RUST_SOURCE_LINES} 行)\n新規超過: {newly_oversized:#?}\n解消済みまたは不正な allowlist: {stale_allowlist:#?}"
    );
}
