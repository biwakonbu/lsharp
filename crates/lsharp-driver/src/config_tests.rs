use super::*;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.constraints.random_test_count, 100);
    assert_eq!(config.constraints.satisfies_search_count, 1000);
    assert!(config.constraints.compile_time_check);
    assert!(config.doc_review.structured);
    assert!(!config.doc_review.pre_commit);
}

#[test]
fn test_parse_minimal_toml() {
    let content = r#"
[project]
name = "test"
"#;
    let config: Config = toml::from_str(content).unwrap();
    assert_eq!(config.project.name, "test");
    assert_eq!(config.project.version, "0.1.0");
    assert_eq!(config.constraints.random_test_count, 100);
}

#[test]
fn test_parse_full_toml() {
    let content = r#"
[project]
name = "myproject"
version = "1.0.0"
entry = "src/app.ls"

[constraints]
random-test-count = 200
satisfies-search-count = 5000
compile-time-check = false

[doc-review]
structured = true
comments = true
pre-commit = true
warning-level = "error"
"#;
    let config: Config = toml::from_str(content).unwrap();
    assert_eq!(config.project.name, "myproject");
    assert_eq!(config.project.version, "1.0.0");
    assert_eq!(config.project.entry, "src/app.ls");
    assert_eq!(config.constraints.random_test_count, 200);
    assert_eq!(config.constraints.satisfies_search_count, 5000);
    assert!(!config.constraints.compile_time_check);
    assert!(config.doc_review.structured);
    assert!(config.doc_review.comments);
    assert!(config.doc_review.pre_commit);
    assert_eq!(config.doc_review.warning_level, "error");
}

#[test]
fn test_parse_empty_toml() {
    let content = "";
    let config: Config = toml::from_str(content).unwrap();
    assert_eq!(config.project.name, "");
    assert_eq!(config.constraints.random_test_count, 100);
}

#[test]
fn test_load_nonexistent_config() {
    let config = load_config(Path::new("/nonexistent"));
    assert_eq!(config.constraints.random_test_count, 100);
}

#[test]
fn test_load_config_result_nonexistent() {
    // 存在しないディレクトリではデフォルト設定を返す
    let result = load_config_result(Path::new("/nonexistent"));
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.constraints.random_test_count, 100);
}

#[test]
fn test_load_config_result_invalid_toml() {
    // 不正な TOML ファイルではエラーを返す
    let dir = std::env::temp_dir().join("lsharp_test_invalid_toml");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("lsharp.toml"), "invalid {{{{ toml").unwrap();

    let result = load_config_result(&dir);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::Parse(msg) => {
            assert!(!msg.is_empty());
        }
        other => panic!("ParseError を期待しましたが {:?} でした", other),
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_validate_random_test_count_zero() {
    let mut config = Config::default();
    config.constraints.random_test_count = 0;
    let errors = validate_config(&config, Path::new("/tmp"));
    assert!(errors.iter().any(|e| e.contains("random-test-count")));
}

#[test]
fn test_validate_satisfies_search_count_zero() {
    let mut config = Config::default();
    config.constraints.satisfies_search_count = 0;
    let errors = validate_config(&config, Path::new("/tmp"));
    assert!(errors.iter().any(|e| e.contains("satisfies-search-count")));
}

#[test]
fn test_validate_invalid_warning_level() {
    let mut config = Config::default();
    config.doc_review.warning_level = "invalid".to_string();
    let errors = validate_config(&config, Path::new("/tmp"));
    assert!(errors.iter().any(|e| e.contains("warning-level")));
}

#[test]
fn test_validate_valid_warning_levels() {
    for level in &["error", "warning", "off"] {
        let mut config = Config::default();
        config.doc_review.warning_level = level.to_string();
        let errors = validate_config(&config, Path::new("/tmp"));
        assert!(
            !errors.iter().any(|e| e.contains("warning-level")),
            "{level} は有効な値のはず"
        );
    }
}

#[test]
fn test_validate_entry_file_not_found() {
    let mut config = Config::default();
    config.project.name = "test".to_string();
    config.project.entry = "nonexistent/main.ls".to_string();
    let errors = validate_config(&config, Path::new("/tmp"));
    assert!(errors.iter().any(|e| e.contains("entry")));
}

#[test]
fn test_validate_default_config_no_entry_check() {
    // project.name が空の場合は entry ファイルチェックをスキップ
    let config = Config::default();
    let errors = validate_config(&config, Path::new("/tmp"));
    assert!(
        !errors.iter().any(|e| e.contains("entry")),
        "name が空なら entry チェックはスキップされるべき"
    );
}

#[test]
fn test_load_config_result_validation_error() {
    // 有効な TOML だが設定値が不正な場合
    let dir = std::env::temp_dir().join("lsharp_test_validation");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("lsharp.toml"),
        r#"
[constraints]
random-test-count = 0

[doc-review]
warning-level = "invalid"
"#,
    )
    .unwrap();

    let result = load_config_result(&dir);
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::Validation(errors) => {
            assert!(errors.len() >= 2);
        }
        other => panic!("ValidationError を期待しましたが {:?} でした", other),
    }

    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn test_parse_dependencies_version() {
    let content = r#"
[dependencies]
math = "1.0.0"
"#;
    let config: Config = toml::from_str(content).unwrap();
    assert_eq!(config.dependencies.len(), 1);
    match &config.dependencies["math"] {
        DependencySpec::Version(v) => assert_eq!(v, "1.0.0"),
        other => panic!("Version を期待しましたが {:?} でした", other),
    }
}

#[test]
fn test_parse_dependencies_git() {
    let content = r#"
[dependencies.mylib]
git = "https://github.com/user/mylib.git"
branch = "main"
"#;
    let config: Config = toml::from_str(content).unwrap();
    assert_eq!(config.dependencies.len(), 1);
    match &config.dependencies["mylib"] {
        DependencySpec::Git { git, branch, tag } => {
            assert_eq!(git, "https://github.com/user/mylib.git");
            assert_eq!(branch.as_deref(), Some("main"));
            assert!(tag.is_none());
        }
        other => panic!("Git を期待しましたが {:?} でした", other),
    }
}

#[test]
fn test_parse_dependencies_path() {
    let content = r#"
[dependencies.local-lib]
path = "../local-lib"
"#;
    let config: Config = toml::from_str(content).unwrap();
    match &config.dependencies["local-lib"] {
        DependencySpec::Path { path } => assert_eq!(path, "../local-lib"),
        other => panic!("Path を期待しましたが {:?} でした", other),
    }
}

#[test]
fn test_parse_dependencies_mixed() {
    let content = r#"
[dependencies]
math = "1.0.0"

[dependencies.mylib]
git = "https://github.com/user/mylib.git"
tag = "v2.0"

[dependencies.local]
path = "./libs/local"
"#;
    let config: Config = toml::from_str(content).unwrap();
    assert_eq!(config.dependencies.len(), 3);
    assert!(matches!(
        &config.dependencies["math"],
        DependencySpec::Version(_)
    ));
    assert!(matches!(
        &config.dependencies["mylib"],
        DependencySpec::Git { .. }
    ));
    assert!(matches!(
        &config.dependencies["local"],
        DependencySpec::Path { .. }
    ));
}

#[test]
fn test_parse_no_dependencies() {
    let content = r#"
[project]
name = "test"
"#;
    let config: Config = toml::from_str(content).unwrap();
    assert!(config.dependencies.is_empty());
}

#[test]
fn test_parse_project_metadata_exports_and_dev_dependencies() {
    let content = r#"
[project]
name = "demo"
version = "0.2.0"
description = "demo package"
license = "MIT"
authors = ["A <a@example.com>"]
repository = "https://github.com/example/demo"
keywords = ["demo", "lsharp"]
lsharp-version = ">=0.2.0"

[project.exports]
modules = ["Demo", "Demo.Util"]

[dev-dependencies]
testkit = "0.1.0"
"#;

    let config: Config = toml::from_str(content).unwrap();
    assert_eq!(config.project.description, "demo package");
    assert_eq!(config.project.license, "MIT");
    assert_eq!(config.project.authors, vec!["A <a@example.com>"]);
    assert_eq!(config.project.repository, "https://github.com/example/demo");
    assert_eq!(config.project.keywords, vec!["demo", "lsharp"]);
    assert_eq!(config.project.lsharp_version, ">=0.2.0");
    assert_eq!(config.project.exports.modules, vec!["Demo", "Demo.Util"]);
    assert!(matches!(
        config.dev_dependencies.get("testkit"),
        Some(DependencySpec::Version(v)) if v == "0.1.0"
    ));
}

#[test]
fn test_config_error_display() {
    let err = ConfigError::Read("permission denied".to_string());
    assert!(err.to_string().contains("permission denied"));

    let err = ConfigError::Parse("syntax error".to_string());
    assert!(err.to_string().contains("syntax error"));

    let err = ConfigError::Validation(vec!["err1".to_string(), "err2".to_string()]);
    let msg = err.to_string();
    assert!(msg.contains("err1"));
    assert!(msg.contains("err2"));
}
