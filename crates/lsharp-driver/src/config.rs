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

/// lsharp.toml を読み込み
pub fn load_config(dir: &Path) -> Config {
    let config_path = dir.join("lsharp.toml");
    if !config_path.exists() {
        return Config::default();
    }

    match std::fs::read_to_string(&config_path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("警告: lsharp.toml のパースに失敗: {e}");
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("警告: lsharp.toml の読み込みに失敗: {e}");
            Config::default()
        }
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
}
