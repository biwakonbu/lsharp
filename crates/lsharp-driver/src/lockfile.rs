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

/// Config から Lockfile を生成する
///
/// - Path 依存: 絶対パスに解決して記録
/// - Version 依存: バージョン文字列を記録 (レジストリ未実装)
/// - Git 依存: Git URL を記録 (クローン未実装)
pub fn generate_lockfile(config: &Config, project_dir: &Path) -> Lockfile {
    let mut entries: Vec<LockEntry> = config
        .dependencies
        .iter()
        .map(|(name, spec)| match spec {
            DependencySpec::Path { path } => {
                let resolved = project_dir.join(path);
                // canonicalize できれば絶対パスに、できなければ join 結果をそのまま使う
                let abs_path = resolved
                    .canonicalize()
                    .unwrap_or(resolved);
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

    // 出力順を安定させるためソート
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    Lockfile { entries }
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

    std::fs::write(path, &out).map_err(|e| format!("ロックファイルの書き込みに失敗: {e}"))
}

/// ロックファイルを読み込む
#[allow(dead_code)]
pub fn read_lockfile(path: &Path) -> Result<Lockfile, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("ロックファイルの読み込みに失敗: {e}"))?;

    // toml::Value でパースして [[package]] 配列を手動取得
    let table: toml::Value =
        content.parse().map_err(|e: toml::de::Error| format!("TOML パースエラー: {e}"))?;

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
mod tests {
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
            lockfile.entries[0].source.contains("lsharp_lockfile_test_path"),
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
        deps.insert("math".to_string(), DependencySpec::Version("1.0.0".to_string()));
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

        let netlib = lockfile.entries.iter().find(|e| e.name == "netlib").unwrap();
        assert_eq!(netlib.source, "git:https://github.com/user/netlib.git?branch=main");
    }

    /// 存在しないファイルの読み込みはエラーを返す
    #[test]
    fn test_read_lockfile_not_found() {
        let result = read_lockfile(Path::new("/nonexistent/.lsharp/lock.toml"));
        assert!(result.is_err());
    }
}
