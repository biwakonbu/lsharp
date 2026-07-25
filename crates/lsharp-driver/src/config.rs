//! lsharp.toml 設定ファイルの読み込み

use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

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

    /// [validation] セクション
    #[serde(default)]
    pub validation: ValidationConfig,

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
    Path { path: String },
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

/// [validation] セクション
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ValidationConfig {
    /// `lsharp validate` が入力を省略した場合に使う manifest
    #[serde(default)]
    pub manifest: Option<String>,
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
    #[serde(
        rename = "satisfies-search-count",
        default = "default_satisfies_search_count"
    )]
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
        errors
            .push("constraints.satisfies-search-count は 1 以上の値を指定してください".to_string());
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

    let config: Config = toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;

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

/// `[validation].manifest` を project root から安全に解決する。
///
/// 設定ファイル経由の入力だけは project-relative に限定し、絶対 path、親方向への
/// traversal、存在しないファイル、project root 外へ出る symlink を受け付けない。
pub fn resolve_validation_manifest_path(
    project_dir: &Path,
    configured: Option<&str>,
) -> Result<PathBuf, String> {
    let configured = configured.ok_or_else(|| {
        "[validation].manifest が未設定です。manifest を明示するか lsharp.toml に設定してください"
            .to_string()
    })?;
    if configured.trim().is_empty() {
        return Err("[validation].manifest は空にできません".to_string());
    }

    let relative = Path::new(configured);
    if relative.is_absolute() {
        return Err(format!(
            "[validation].manifest は project-relative path が必要です: {configured}"
        ));
    }
    if relative
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(format!(
            "[validation].manifest に project root 外への '..' は指定できません: {configured}"
        ));
    }

    let project_root = project_dir
        .canonicalize()
        .map_err(|error| format!("project root の解決に失敗しました: {error}"))?;
    let manifest_path = project_root.join(relative);
    let resolved = manifest_path.canonicalize().map_err(|error| {
        format!(
            "[validation].manifest が見つかりません ({}): {error}",
            manifest_path.display()
        )
    })?;
    if !resolved.starts_with(&project_root) {
        return Err(format!(
            "[validation].manifest は project root 外を指せません: {configured}"
        ));
    }
    if !resolved.is_file() {
        return Err(format!(
            "[validation].manifest は通常のファイルを指してください: {}",
            resolved.display()
        ));
    }

    Ok(resolved)
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
