use super::*;
use std::collections::HashMap;

/// Path 依存を含む Config から Lockfile を生成できる
#[test]
fn test_generate_lockfile_path_dependency() {
    // テスト用の一時ディレクトリを作成
    let tmp = std::env::temp_dir().join("lsharp_lockfile_test_path");
    let lib_dir = tmp.join("mylib");
    std::fs::create_dir_all(&lib_dir).unwrap();

    let mut deps = HashMap::new();
    deps.insert(
        "mylib".to_string(),
        DependencySpec::Path {
            path: "mylib".to_string(),
        },
    );

    let config = Config {
        dependencies: deps,
        ..Config::default()
    };

    let lockfile = generate_lockfile(&config, &tmp);
    assert_eq!(lockfile.entries.len(), 1);
    assert_eq!(lockfile.entries[0].name, "mylib");
    assert!(
        lockfile.entries[0].source.starts_with("path:"),
        "source は path: プレフィックスを持つべき: {}",
        lockfile.entries[0].source
    );
    // 絶対パスが含まれるはず
    assert!(
        lockfile.entries[0]
            .source
            .contains("lsharp_lockfile_test_path"),
        "解決済みパスにテストディレクトリ名が含まれるべき: {}",
        lockfile.entries[0].source
    );

    std::fs::remove_dir_all(&tmp).unwrap();
}

/// 依存なしの Config からは空の Lockfile が生成される
#[test]
fn test_generate_lockfile_empty_config() {
    let config = Config::default();
    let lockfile = generate_lockfile(&config, Path::new("/tmp"));
    assert!(lockfile.entries.is_empty());
}

/// write -> read のラウンドトリップでデータが保持される
#[test]
fn test_lockfile_write_read_roundtrip() {
    let tmp = std::env::temp_dir().join("lsharp_lockfile_roundtrip");
    std::fs::create_dir_all(&tmp).unwrap();
    let lock_path = tmp.join("lock.toml");

    let lockfile = Lockfile {
        entries: vec![
            LockEntry {
                name: "alpha".to_string(),
                version: "1.2.3".to_string(),
                source: "registry:default".to_string(),
            },
            LockEntry {
                name: "beta".to_string(),
                version: "0.0.0".to_string(),
                source: "git:https://github.com/user/beta.git?branch=main".to_string(),
            },
            LockEntry {
                name: "gamma".to_string(),
                version: "0.0.0".to_string(),
                source: "path:/home/user/libs/gamma".to_string(),
            },
        ],
    };

    write_lockfile(&lockfile, &lock_path).expect("書き込み成功");
    let loaded = read_lockfile(&lock_path).expect("読み込み成功");

    assert_eq!(lockfile, loaded);

    std::fs::remove_dir_all(&tmp).unwrap();
}

/// Version 依存と Git 依存の生成を確認
#[test]
fn test_generate_lockfile_version_and_git() {
    let mut deps = HashMap::new();
    deps.insert(
        "math".to_string(),
        DependencySpec::Version("1.0.0".to_string()),
    );
    deps.insert(
        "netlib".to_string(),
        DependencySpec::Git {
            git: "https://github.com/user/netlib.git".to_string(),
            branch: Some("main".to_string()),
            tag: None,
        },
    );

    let config = Config {
        dependencies: deps,
        ..Config::default()
    };

    let lockfile = generate_lockfile(&config, Path::new("/tmp"));
    assert_eq!(lockfile.entries.len(), 2);

    let math = lockfile.entries.iter().find(|e| e.name == "math").unwrap();
    assert_eq!(math.version, "1.0.0");
    assert_eq!(math.source, "registry:default");

    let netlib = lockfile
        .entries
        .iter()
        .find(|e| e.name == "netlib")
        .unwrap();
    assert_eq!(
        netlib.source,
        "git:https://github.com/user/netlib.git?branch=main"
    );
}

/// 存在しないファイルの読み込みはエラーを返す
#[test]
fn test_read_lockfile_not_found() {
    let result = read_lockfile(Path::new("/nonexistent/.lsharp/lock.toml"));
    assert!(result.is_err());
}
