//! モジュールファイルの探索と source graph 構築

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::{ModuleGraph, ModuleGraphError};

/// モジュール解決に使う探索パス
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSearchPaths {
    pub package_root: PathBuf,
    pub source_root: PathBuf,
    pub package_sources: Vec<PathBuf>,
    pub stdlib_root: Option<PathBuf>,
}

impl ModuleSearchPaths {
    pub fn discover(entry_file: &Path) -> Self {
        let entry_dir = entry_file.parent().unwrap_or_else(|| Path::new("."));
        let nearest_src_root = nearest_src_root(entry_dir);
        let package_root = find_package_root(entry_dir)
            .or_else(|| {
                nearest_src_root
                    .as_ref()
                    .and_then(|src_root| src_root.parent().map(Path::to_path_buf))
            })
            .unwrap_or_else(|| entry_dir.to_path_buf());
        let source_root_candidate = package_root.join("src");
        let source_root = if source_root_candidate.is_dir() {
            source_root_candidate
        } else {
            nearest_src_root.unwrap_or_else(|| entry_dir.to_path_buf())
        };
        let package_sources = discover_package_sources(&package_root.join(".lsharp/packages"));
        Self {
            source_root,
            package_root,
            package_sources,
            stdlib_root: default_stdlib_root(),
        }
    }
}

impl ModuleGraph {
    fn source_override<'a>(
        path: &Path,
        source_overrides: Option<&'a HashMap<PathBuf, String>>,
    ) -> Option<&'a str> {
        source_overrides
            .and_then(|overrides| overrides.get(path))
            .map(String::as_str)
    }

    fn read_source(
        path: &Path,
        source_overrides: Option<&HashMap<PathBuf, String>>,
    ) -> Result<String, ModuleGraphError> {
        if let Some(source) = Self::source_override(path, source_overrides) {
            return Ok(source.to_string());
        }

        std::fs::read_to_string(path).map_err(|e| ModuleGraphError::ModuleNotFound {
            name: path.display().to_string(),
            from: format!("ファイル読み込みエラー: {e}"),
        })
    }

    /// ファイルパスからモジュール名を取得
    pub fn module_for_file(&self, path: &str) -> Option<&str> {
        self.file_map
            .iter()
            .find(|(_, p)| p.as_str() == path)
            .map(|(name, _)| name.as_str())
    }

    /// モジュール名をファイルパス候補に変換
    ///
    /// `ModuleName` → `["ModuleName.ls", "module_name.ls"]`
    /// `A.B` → `["A/B.ls", "a/b.ls"]`
    pub fn module_name_to_paths(name: &str) -> Vec<String> {
        let mut candidates = Vec::new();

        // ドット区切りをパス区切りに変換
        let path_parts: Vec<&str> = name.split('.').collect();

        // PascalCase 版: ModuleName.ls or A/B.ls
        let pascal_path = format!("{}.ls", path_parts.join("/"));
        candidates.push(pascal_path);

        // snake_case 版: module_name.ls or a/b.ls
        let snake_parts: Vec<String> = path_parts
            .iter()
            .map(|part| Self::to_snake_case(part))
            .collect();
        let snake_path = format!("{}.ls", snake_parts.join("/"));
        if !candidates.contains(&snake_path) {
            candidates.push(snake_path);
        }

        candidates
    }

    /// PascalCase を snake_case に変換
    pub(super) fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(ch.to_lowercase().next().unwrap());
            } else {
                result.push(ch);
            }
        }
        result
    }

    /// 指定ディレクトリからモジュールファイルを探索
    ///
    /// `base_dir` を基準にモジュール名からファイルを探し、
    /// 最初に見つかったパスを返す。
    pub fn resolve_module_file(
        name: &str,
        base_dir: &std::path::Path,
    ) -> Option<std::path::PathBuf> {
        let candidates = Self::module_name_to_paths(name);
        for candidate in &candidates {
            let path = base_dir.join(candidate);
            if path.exists() {
                return Some(path);
            }
        }
        None
    }

    pub fn resolve_module_file_with_search_paths(
        name: &str,
        search_paths: &ModuleSearchPaths,
    ) -> Option<std::path::PathBuf> {
        let candidates = Self::module_name_to_paths(name);
        for candidate in &candidates {
            let local = search_paths.source_root.join(candidate);
            if local.exists() {
                return Some(local);
            }
            for package_source in &search_paths.package_sources {
                let package_path = package_source.join(candidate);
                if package_path.exists() {
                    return Some(package_path);
                }
            }
            if let Some(stdlib_root) = &search_paths.stdlib_root {
                let stdlib_path = stdlib_root.join(candidate);
                if stdlib_path.exists() {
                    return Some(stdlib_path);
                }
            }
        }
        None
    }

    pub fn resolve_module_import_path(
        name: &str,
        from: &str,
        search_paths: &ModuleSearchPaths,
    ) -> Result<Option<std::path::PathBuf>, ModuleGraphError> {
        let Some(path) = Self::resolve_module_file_with_search_paths(name, search_paths) else {
            return Ok(None);
        };

        if let Some(package_root) =
            external_package_root_for_path(&path, &search_paths.package_sources)
            && !is_module_exported_from_package(name, &package_root)
        {
            return Err(ModuleGraphError::ModuleNotExported {
                name: name.to_string(),
                package: package_name_for_root(&package_root),
                from: from.to_string(),
            });
        }

        Ok(Some(path))
    }

    /// ソースファイルから import を抽出し、依存グラフを構築
    ///
    /// `entry_file` をエントリポイントとし、再帰的に依存ファイルを探索する。
    pub fn build_from_entry(
        entry_file: &std::path::Path,
    ) -> Result<(Self, Vec<(String, std::path::PathBuf)>), ModuleGraphError> {
        Self::build_from_entry_with_overrides(entry_file, &HashMap::new())
    }

    pub fn build_from_entry_with_overrides(
        entry_file: &std::path::Path,
        source_overrides: &HashMap<PathBuf, String>,
    ) -> Result<(Self, Vec<(String, std::path::PathBuf)>), ModuleGraphError> {
        Self::build_from_entry_with_overrides_mode(entry_file, source_overrides, false)
    }

    /// SCC を含むエントリから、依存先が先に来る順序でファイルを探索する。
    ///
    /// 通常の `build_from_entry` は既存互換のため循環をエラーにする。この経路は
    /// SCC 単位の一括推論を行う compile pipeline 専用で、循環したグループを許容する。
    pub fn build_from_entry_with_scc(
        entry_file: &std::path::Path,
    ) -> Result<(Self, Vec<(String, std::path::PathBuf)>), ModuleGraphError> {
        Self::build_from_entry_with_overrides_mode(entry_file, &HashMap::new(), true)
    }

    /// source override を含むエントリから、循環を許容した SCC compile 用 graph を構築する。
    pub fn build_from_entry_with_overrides_scc(
        entry_file: &std::path::Path,
        source_overrides: &HashMap<PathBuf, String>,
    ) -> Result<(Self, Vec<(String, std::path::PathBuf)>), ModuleGraphError> {
        Self::build_from_entry_with_overrides_mode(entry_file, source_overrides, true)
    }

    fn build_from_entry_with_overrides_mode(
        entry_file: &std::path::Path,
        source_overrides: &HashMap<PathBuf, String>,
        allow_cycles: bool,
    ) -> Result<(Self, Vec<(String, std::path::PathBuf)>), ModuleGraphError> {
        use std::collections::VecDeque;

        let search_paths = ModuleSearchPaths::discover(entry_file);
        let mut graph = Self::new();
        // (モジュール名, ファイルパス) のリスト（トポソ順で返す）
        let mut file_list: Vec<(String, std::path::PathBuf)> = Vec::new();
        let mut queue: VecDeque<(String, std::path::PathBuf)> = VecDeque::new();

        // エントリファイルのモジュール名を取得
        let entry_module = Self::extract_module_name(entry_file, Some(source_overrides))?;
        queue.push_back((entry_module.clone(), entry_file.to_path_buf()));

        while let Some((mod_name, mod_path)) = queue.pop_front() {
            if graph.modules.contains_key(&mod_name) {
                continue;
            }

            // ソースファイルからインポートを抽出
            let imports = Self::extract_imports(&mod_path, Some(source_overrides))?;

            graph.add_module(
                mod_name.clone(),
                imports.clone(),
                Some(mod_path.display().to_string()),
            )?;
            file_list.push((mod_name.clone(), mod_path.clone()));

            // 依存モジュールをキューに追加
            for imp in &imports {
                if !graph.modules.contains_key(imp) {
                    if let Some(imp_path) =
                        Self::resolve_module_import_path(imp, &mod_name, &search_paths)?
                    {
                        queue.push_back((imp.clone(), imp_path));
                    } else {
                        return Err(ModuleGraphError::ModuleNotFound {
                            name: imp.clone(),
                            from: mod_name.clone(),
                        });
                    }
                }
            }
        }

        // トポロジカルソート順に並べ替え。SCC compile 経路だけは循環群を flatten する。
        let sorted = match graph.topological_sort() {
            Ok(sorted) => sorted,
            Err(ModuleGraphError::CyclicDependency { .. }) if allow_cycles => {
                graph.scc_groups().into_iter().flatten().collect()
            }
            Err(error) => return Err(error),
        };
        let sorted_list: Vec<(String, std::path::PathBuf)> = sorted
            .iter()
            .filter_map(|name| file_list.iter().find(|(n, _)| n == name).cloned())
            .collect();

        graph.rebuild_reverse_deps();
        Ok((graph, sorted_list))
    }

    /// ソースファイルからモジュール名を抽出
    ///
    /// `(module Name)` があればその名前を、なければファイル名から生成する。
    fn extract_module_name(
        path: &std::path::Path,
        source_overrides: Option<&HashMap<PathBuf, String>>,
    ) -> Result<String, ModuleGraphError> {
        let source = Self::read_source(path, source_overrides)?;

        // パースして module 宣言を探す
        if let Ok(program) = lsharp_syntax::parse(&source) {
            for decl in &program.decls {
                if let lsharp_syntax::ast::Decl::ModuleDecl { name, .. } = decl {
                    return Ok(name.clone());
                }
            }
        }

        // module 宣言がなければファイル名から生成
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("Main");
        // snake_case を PascalCase に変換
        Ok(Self::to_pascal_case(stem))
    }

    /// ソースファイルから import モジュール名を抽出
    fn extract_imports(
        path: &std::path::Path,
        source_overrides: Option<&HashMap<PathBuf, String>>,
    ) -> Result<Vec<String>, ModuleGraphError> {
        let source = Self::read_source(path, source_overrides)?;

        let mut imports = Vec::new();
        if let Ok(program) = lsharp_syntax::parse(&source) {
            for decl in &program.decls {
                if let lsharp_syntax::ast::Decl::ImportDecl { module, .. } = decl {
                    imports.push(module.clone());
                }
            }
        }
        Ok(imports)
    }

    /// snake_case を PascalCase に変換
    pub(super) fn to_pascal_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => {
                        let mut result = c.to_uppercase().to_string();
                        result.extend(chars);
                        result
                    }
                }
            })
            .collect()
    }
}

fn find_package_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        if current.join("lsharp.toml").exists() {
            return Some(current);
        }
        let parent = current.parent()?;
        if parent == current {
            return None;
        }
        current = parent.to_path_buf();
    }
}

fn nearest_src_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };
    loop {
        if current.file_name().and_then(|name| name.to_str()) == Some("src") {
            return Some(current);
        }
        let parent = current.parent()?;
        if parent == current {
            return None;
        }
        current = parent.to_path_buf();
    }
}

fn discover_package_sources(base: &Path) -> Vec<PathBuf> {
    let mut sources = Vec::new();
    if !base.exists() {
        return sources;
    }
    let Ok(entries) = std::fs::read_dir(base) else {
        return sources;
    };
    for entry in entries.flatten() {
        let path = entry.path().join("src");
        if path.is_dir() {
            sources.push(path);
        }
    }
    sources.sort();
    sources
}

fn external_package_root_for_path(path: &Path, package_sources: &[PathBuf]) -> Option<PathBuf> {
    for package_source in package_sources {
        if path.starts_with(package_source) {
            return package_source.parent().map(Path::to_path_buf);
        }
    }
    None
}

fn package_name_for_root(package_root: &Path) -> String {
    let config_path = package_root.join("lsharp.toml");
    let Ok(source) = std::fs::read_to_string(config_path) else {
        return package_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
    };
    let Ok(value) = source.parse::<toml::Value>() else {
        return package_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
    };
    value
        .get("project")
        .and_then(|project| project.get("name"))
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| {
            package_root
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown")
        })
        .to_string()
}

fn is_module_exported_from_package(module: &str, package_root: &Path) -> bool {
    let config_path = package_root.join("lsharp.toml");
    let Ok(source) = std::fs::read_to_string(config_path) else {
        return true;
    };
    let Ok(value) = source.parse::<toml::Value>() else {
        return true;
    };
    let Some(modules) = value
        .get("project")
        .and_then(|project| project.get("exports"))
        .and_then(|exports| exports.get("modules"))
        .and_then(toml::Value::as_array)
    else {
        return true;
    };

    if modules.is_empty() {
        return true;
    }

    modules
        .iter()
        .filter_map(toml::Value::as_str)
        .any(|name| name == module)
}

fn default_stdlib_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("LSHARP_STDLIB_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }
    let bundled = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
    if bundled.exists() {
        Some(bundled)
    } else {
        None
    }
}
