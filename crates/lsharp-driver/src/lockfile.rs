//! .lsharp/lock.toml ロックファイルの生成・読み書き

use crate::config::{Config, DependencySpec};
use std::path::Path;

/// ロックファイル全体
#[derive(Debug, Clone, PartialEq)]
pub struct Lockfile {
    /// 解決済み依存エントリの一覧
    pub entries: Vec<LockEntry>,
}

/// 解決済み依存の1エントリ
#[derive(Debug, Clone, PartialEq)]
pub struct LockEntry {
    /// パッケージ名
    pub name: String,
    /// バージョン文字列
    pub version: String,
    /// ソース (絶対パス、Git URL、レジストリ名など)
    pub source: String,
}

pub fn generate_lockfile_from_entries(mut entries: Vec<LockEntry>) -> Lockfile {
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Lockfile { entries }
}

/// Config から Lockfile を生成する
///
/// - Path 依存: 絶対パスに解決して記録
/// - Version 依存: バージョン文字列を記録 (レジストリ未実装)
/// - Git 依存: Git URL を記録 (クローン未実装)
#[allow(dead_code)]
pub fn generate_lockfile(config: &Config, project_dir: &Path) -> Lockfile {
    let entries: Vec<LockEntry> = config
        .dependencies
        .iter()
        .map(|(name, spec)| match spec {
            DependencySpec::Path { path } => {
                let resolved = project_dir.join(path);
                // canonicalize できれば絶対パスに、できなければ join 結果をそのまま使う
                let abs_path = resolved.canonicalize().unwrap_or(resolved);
                LockEntry {
                    name: name.clone(),
                    version: "0.0.0".to_string(),
                    source: format!("path:{}", abs_path.display()),
                }
            }
            DependencySpec::Version(v) => LockEntry {
                name: name.clone(),
                version: v.clone(),
                source: "registry:default".to_string(),
            },
            DependencySpec::Git { git, branch, tag } => {
                let ref_part = if let Some(b) = branch {
                    format!("?branch={b}")
                } else if let Some(t) = tag {
                    format!("?tag={t}")
                } else {
                    String::new()
                };
                LockEntry {
                    name: name.clone(),
                    version: "0.0.0".to_string(),
                    source: format!("git:{git}{ref_part}"),
                }
            }
        })
        .collect();

    generate_lockfile_from_entries(entries)
}

/// Lockfile を TOML 形式でファイルに書き出す
pub fn write_lockfile(lockfile: &Lockfile, path: &Path) -> Result<(), String> {
    let mut out = String::from("# .lsharp/lock.toml -- 自動生成。手動編集しないでください。\n\n");

    for entry in &lockfile.entries {
        out.push_str("[[package]]\n");
        out.push_str(&format!("name = \"{}\"\n", escape_toml(&entry.name)));
        out.push_str(&format!("version = \"{}\"\n", escape_toml(&entry.version)));
        out.push_str(&format!("source = \"{}\"\n", escape_toml(&entry.source)));
        out.push('\n');
    }

    crate::atomic_write::write_durable_atomic(path, out.as_bytes())
        .map_err(|e| format!("ロックファイルの書き込みに失敗: {e}"))
}

/// ロックファイルを読み込む
#[allow(dead_code)]
pub fn read_lockfile(path: &Path) -> Result<Lockfile, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("ロックファイルの読み込みに失敗: {e}"))?;

    // toml::Value でパースして [[package]] 配列を手動取得
    let table: toml::Value = content
        .parse()
        .map_err(|e: toml::de::Error| format!("TOML パースエラー: {e}"))?;

    let packages = table
        .get("package")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let entries = packages
        .iter()
        .map(|pkg| {
            let name = pkg
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let version = pkg
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("0.0.0")
                .to_string();
            let source = pkg
                .get("source")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            LockEntry {
                name,
                version,
                source,
            }
        })
        .collect();

    Ok(Lockfile { entries })
}

/// TOML 文字列値のエスケープ (簡易)
fn escape_toml(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
#[path = "lockfile_tests.rs"]
mod tests;
