//! lsharp.toml 設定ファイルの読み込み

use serde::Deserialize;
use std::path::Path;

/// L# プロジェクト設定
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Config {
    /// [project] セクション
    #[serde(default)]
    pub project: ProjectConfig,

    /// [constraints] セクション
    #[serde(default)]
    pub constraints: ConstraintsConfig,

    /// [doc-review] セクション
    #[serde(rename = "doc-review", default)]
    pub doc_review: DocReviewConfig,

    /// [dependencies] セクション (P9-3)
    #[serde(default)]
    pub dependencies: std::collections::HashMap<String, DependencySpec>,

    /// [dev-dependencies] セクション (Phase 12)
    #[serde(rename = "dev-dependencies", default)]
    pub dev_dependencies: std::collections::HashMap<String, DependencySpec>,
}

/// 依存関係の指定 (P9-3)
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DependencySpec {
    /// バージョン文字列のみ: "1.0.0"
    Version(String),
    /// 詳細指定: { git = "...", branch = "..." }
    Git {
        git: String,
        #[serde(default)]
        branch: Option<String>,
        #[serde(default)]
        tag: Option<String>,
    },
    /// ローカルパス: { path = "..." }
    Path {
        path: String,
    },
}

/// [project] セクション
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectConfig {
    /// プロジェクト名
    #[serde(default)]
    pub name: String,

    /// バージョン
    #[serde(default = "default_version")]
    pub version: String,

    /// エントリポイント
    #[serde(default = "default_entry")]
    pub entry: String,

    /// 説明
    #[serde(default)]
    pub description: String,

    /// ライセンス
    #[serde(default)]
    pub license: String,

    /// 著者
    #[serde(default)]
    pub authors: Vec<String>,

    /// リポジトリ
    #[serde(default)]
    pub repository: String,

    /// キーワード
    #[serde(default)]
    pub keywords: Vec<String>,

    /// 要求する L# バージョン
    #[serde(rename = "lsharp-version", default)]
    pub lsharp_version: String,

    /// 公開モジュール設定
    #[serde(default)]
    pub exports: ProjectExportsConfig,
}

/// [project.exports] セクション
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectExportsConfig {
    #[serde(default)]
    pub modules: Vec<String>,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

fn default_entry() -> String {
    "src/main.ls".to_string()
}

/// [constraints] セクション
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ConstraintsConfig {
    /// ランダムテストの生成数
    #[serde(rename = "random-test-count", default = "default_random_test_count")]
    pub random_test_count: u32,

    /// satisfies 探索のサンプル数
    #[serde(rename = "satisfies-search-count", default = "default_satisfies_search_count")]
    pub satisfies_search_count: u32,

    /// コンパイル時制約チェックの有効化
    #[serde(rename = "compile-time-check", default = "default_true")]
    pub compile_time_check: bool,
}

impl Default for ConstraintsConfig {
    fn default() -> Self {
        Self {
            random_test_count: default_random_test_count(),
            satisfies_search_count: default_satisfies_search_count(),
            compile_time_check: true,
        }
    }
}

fn default_random_test_count() -> u32 {
    100
}

fn default_satisfies_search_count() -> u32 {
    1000
}

fn default_true() -> bool {
    true
}

/// [doc-review] セクション
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct DocReviewConfig {
    /// 構造化メタデータの検証を有効化
    #[serde(default = "default_true")]
    pub structured: bool,

    /// コメントの鮮度チェックを有効化
    #[serde(default)]
    pub comments: bool,

    /// pre-commit フックの有効化
    #[serde(rename = "pre-commit", default)]
    pub pre_commit: bool,

    /// 警告レベル (error, warning, off)
    #[serde(rename = "warning-level", default = "default_warning_level")]
    pub warning_level: String,
}

impl Default for DocReviewConfig {
    fn default() -> Self {
        Self {
            structured: true,
            comments: false,
            pre_commit: false,
            warning_level: default_warning_level(),
        }
    }
}

fn default_warning_level() -> String {
    "warning".to_string()
}

/// 設定読み込みエラー
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// ファイル読み込み失敗
    Read(String),
    /// TOML パースエラー
    Parse(String),
    /// 設定値の検証エラー
    Validation(Vec<String>),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Read(msg) => write!(f, "lsharp.toml の読み込みに失敗: {msg}"),
            ConfigError::Parse(msg) => write!(f, "lsharp.toml のパースに失敗: {msg}"),
            ConfigError::Validation(errors) => {
                write!(f, "設定値の検証エラー: {}", errors.join("; "))
            }
        }
    }
}

/// 設定値の有効性を検証
pub fn validate_config(config: &Config, project_dir: &Path) -> Vec<String> {
    let mut errors = Vec::new();

    // random-test-count が 0 の場合は警告
    if config.constraints.random_test_count == 0 {
        errors.push("constraints.random-test-count は 1 以上の値を指定してください".to_string());
    }

    // satisfies-search-count が 0 の場合は警告
    if config.constraints.satisfies_search_count == 0 {
        errors.push(
            "constraints.satisfies-search-count は 1 以上の値を指定してください".to_string(),
        );
    }

    // entry ファイルの存在確認
    if !config.project.name.is_empty() {
        let entry_path = project_dir.join(&config.project.entry);
        if !entry_path.exists() {
            errors.push(format!(
                "project.entry で指定されたファイルが存在しません: {}",
                config.project.entry
            ));
        }
    }

    // warning-level の値が有効か
    let valid_levels = ["error", "warning", "off"];
    if !valid_levels.contains(&config.doc_review.warning_level.as_str()) {
        errors.push(format!(
            "doc-review.warning-level の値が不正です: {} (有効値: error, warning, off)",
            config.doc_review.warning_level
        ));
    }

    errors
}

/// lsharp.toml を読み込み (Result 版)
pub fn load_config_result(dir: &Path) -> Result<Config, ConfigError> {
    let config_path = dir.join("lsharp.toml");
    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content =
        std::fs::read_to_string(&config_path).map_err(|e| ConfigError::Read(e.to_string()))?;

    let config: Config =
        toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;

    let validation_errors = validate_config(&config, dir);
    if !validation_errors.is_empty() {
        return Err(ConfigError::Validation(validation_errors));
    }

    Ok(config)
}

/// lsharp.toml を読み込み (後方互換: エラー時はデフォルト値を返す)
pub fn load_config(dir: &Path) -> Config {
    match load_config_result(dir) {
        Ok(config) => config,
        Err(ConfigError::Validation(_)) => {
            // 検証エラーの場合はパース自体は成功しているので、
            // ファイルを再読み込みしてデフォルトを返す
            let config_path = dir.join("lsharp.toml");
            if let Ok(content) = std::fs::read_to_string(&config_path)
                && let Ok(config) = toml::from_str::<Config>(&content)
            {
                return config;
            }
            Config::default()
        }
        Err(_) => Config::default(),
    }
}

#[cfg(test)]
mod tests {
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
        assert!(matches!(&config.dependencies["math"], DependencySpec::Version(_)));
        assert!(matches!(&config.dependencies["mylib"], DependencySpec::Git { .. }));
        assert!(matches!(&config.dependencies["local"], DependencySpec::Path { .. }));
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
}
