//! モジュールグラフ
//!
//! 複数モジュール間の依存関係を管理し、
//! コンパイル順序の決定（トポロジカルソート）と循環依存の検出を行う。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const FORMATTER_TRIO_EXPR: &str = "Tools.Text.FormatterExpr";
pub const FORMATTER_TRIO_DECL: &str = "Tools.Text.FormatterDecl";
pub const FORMATTER_TRIO_MAIN: &str = "Tools.Text.Formatter";
const FORMATTER_TRIO_MODULES: [&str; 3] = [
    FORMATTER_TRIO_EXPR,
    FORMATTER_TRIO_DECL,
    FORMATTER_TRIO_MAIN,
];

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

/// モジュールグラフ
#[derive(Debug, Default)]
pub struct ModuleGraph {
    /// モジュール名 -> モジュール情報
    modules: HashMap<String, ModuleNode>,
    /// モジュール名 -> ファイルパス
    file_map: HashMap<String, String>,
    /// モジュール名 -> 直接それを参照しているモジュール群
    reverse_deps: HashMap<String, Vec<String>>,
}

/// モジュールノード
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// モジュール名
    pub name: String,
    /// このモジュールがインポートするモジュール名のリスト
    pub imports: Vec<String>,
    /// ソースファイルパス
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDiff {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

/// モジュールグラフのエラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModuleGraphError {
    #[error("循環依存が検出されました: {cycle}")]
    CyclicDependency { cycle: String },

    #[error("モジュール '{name}' が見つかりません ('{from}' からインポート)")]
    ModuleNotFound { name: String, from: String },

    #[error(
        "モジュール '{name}' は package '{package}' から公開されていません ('{from}' からインポート)"
    )]
    ModuleNotExported {
        name: String,
        package: String,
        from: String,
    },

    #[error("モジュール '{name}' が重複しています")]
    DuplicateModule { name: String },
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self::default()
    }

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

    /// モジュールを追加
    pub fn add_module(
        &mut self,
        name: String,
        imports: Vec<String>,
        file_path: Option<String>,
    ) -> Result<(), ModuleGraphError> {
        if self.modules.contains_key(&name) {
            return Err(ModuleGraphError::DuplicateModule { name });
        }

        if let Some(ref path) = file_path {
            self.file_map.insert(name.clone(), path.clone());
        }
        self.reverse_deps.entry(name.clone()).or_default();
        for import in &imports {
            let dependents = self.reverse_deps.entry(import.clone()).or_default();
            if !dependents.contains(&name) {
                dependents.push(name.clone());
                dependents.sort();
            }
        }

        self.modules.insert(
            name.clone(),
            ModuleNode {
                name,
                imports,
                file_path,
            },
        );

        Ok(())
    }

    /// 循環依存を検出
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut in_stack = HashSet::new();
        let mut path = Vec::new();

        for name in self.modules.keys() {
            if !visited.contains(name)
                && let Some(cycle) =
                    self.dfs_detect_cycle(name, &mut visited, &mut in_stack, &mut path)
            {
                return Some(cycle);
            }
        }

        None
    }

    /// DFS で循環検出
    fn dfs_detect_cycle(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        in_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        in_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(module) = self.modules.get(node) {
            for import in &module.imports {
                if !visited.contains(import) {
                    if let Some(cycle) = self.dfs_detect_cycle(import, visited, in_stack, path) {
                        return Some(cycle);
                    }
                } else if in_stack.contains(import) {
                    // 循環検出: パスの中で import が最初に出現した位置から
                    let start = path.iter().position(|n| n == import).unwrap_or(0);
                    let mut cycle: Vec<String> = path[start..].to_vec();
                    cycle.push(import.clone());
                    return Some(cycle);
                }
            }
        }

        path.pop();
        in_stack.remove(node);
        None
    }

    /// トポロジカルソート（コンパイル順序の決定）
    ///
    /// 返り値: 依存先が先に来る順序でソートされたモジュール名リスト
    pub fn topological_sort(&self) -> Result<Vec<String>, ModuleGraphError> {
        // まず循環依存を検出
        if let Some(cycle) = self.detect_cycles() {
            return Err(ModuleGraphError::CyclicDependency {
                cycle: cycle.join(" -> "),
            });
        }

        let mut visited = HashSet::new();
        let mut order = Vec::new();

        // HashMap のキー順は非決定的なため、モジュール名でソートしてから DFS 開始点を固定する
        let mut module_names: Vec<String> = self.modules.keys().cloned().collect();
        module_names.sort();

        for name in module_names {
            if !visited.contains(&name) {
                self.topo_dfs(&name, &mut visited, &mut order);
            }
        }

        Ok(order)
    }

    /// トポロジカルソートの DFS
    fn topo_dfs(&self, node: &str, visited: &mut HashSet<String>, order: &mut Vec<String>) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());

        if let Some(module) = self.modules.get(node) {
            // 同一モジュール内の import 順も、ソース順に加えて名前で安定化（並列依存の訪問順を固定）
            let mut imports: Vec<String> = module.imports.clone();
            imports.sort();
            for import in imports {
                self.topo_dfs(&import, visited, order);
            }
        }

        order.push(node.to_string());
    }

    /// 未解決のインポートを検出
    pub fn check_imports(&self) -> Vec<ModuleGraphError> {
        let mut errors = Vec::new();

        for (name, module) in &self.modules {
            for import in &module.imports {
                if !self.modules.contains_key(import) {
                    errors.push(ModuleGraphError::ModuleNotFound {
                        name: import.clone(),
                        from: name.clone(),
                    });
                }
            }
        }

        errors
    }

    /// モジュール数を取得
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// 空かどうか
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// 指定モジュールの依存 closure を依存先優先の安定順で返す
    pub fn dependency_closure(&self, module: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut deps = Vec::new();
        self.collect_dependencies(module, &mut visited, &mut deps);
        deps
    }

    /// 指定モジュールを参照しているモジュール closure を近い依存元から安定順で返す
    pub fn reverse_dependency_closure(&self, module: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut out = Vec::new();
        let mut queue = std::collections::VecDeque::new();

        let mut initial = self.reverse_deps.get(module).cloned().unwrap_or_default();
        initial.sort();
        for dependent in initial {
            if visited.insert(dependent.clone()) {
                queue.push_back(dependent.clone());
                out.push(dependent);
            }
        }

        while let Some(current) = queue.pop_front() {
            let mut dependents = self.reverse_deps.get(&current).cloned().unwrap_or_default();
            dependents.sort();
            for dependent in dependents {
                if visited.insert(dependent.clone()) {
                    queue.push_back(dependent.clone());
                    out.push(dependent);
                }
            }
        }

        out
    }

    /// 変更モジュールと、その逆依存 closure を dirty set として返す
    pub fn compute_dirty_set(&self, changed: &[String]) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut dirty = Vec::new();

        for module in expand_changed_modules(changed) {
            if seen.insert(module.clone()) {
                dirty.push(module.clone());
            }
            for dependent in self.reverse_dependency_closure(&module) {
                if seen.insert(dependent.clone()) {
                    dirty.push(dependent);
                }
            }
        }

        dirty
    }

    fn collect_dependencies(
        &self,
        module: &str,
        visited: &mut HashSet<String>,
        deps: &mut Vec<String>,
    ) {
        if let Some(node) = self.modules.get(module) {
            let mut imports = node.imports.clone();
            imports.sort();
            for import in imports {
                if visited.insert(import.clone()) {
                    self.collect_dependencies(&import, visited, deps);
                    deps.push(import);
                }
            }
        }
    }

    fn rebuild_reverse_deps(&mut self) {
        let mut reverse_deps: HashMap<String, Vec<String>> = HashMap::new();
        for name in self.modules.keys() {
            reverse_deps.entry(name.clone()).or_default();
        }
        for (module, node) in &self.modules {
            for import in &node.imports {
                reverse_deps
                    .entry(import.clone())
                    .or_default()
                    .push(module.clone());
            }
        }
        for dependents in reverse_deps.values_mut() {
            dependents.sort();
            dependents.dedup();
        }
        self.reverse_deps = reverse_deps;
    }

    pub fn diff_imports(old: &[String], new: &[String]) -> ImportDiff {
        let old_set: HashSet<&String> = old.iter().collect();
        let new_set: HashSet<&String> = new.iter().collect();
        let mut added: Vec<String> = new
            .iter()
            .filter(|module| !old_set.contains(module))
            .cloned()
            .collect();
        let mut removed: Vec<String> = old
            .iter()
            .filter(|module| !new_set.contains(module))
            .cloned()
            .collect();
        added.sort();
        added.dedup();
        removed.sort();
        removed.dedup();
        ImportDiff { added, removed }
    }

    pub fn update_module_imports(
        &mut self,
        module: &str,
        new_imports: Vec<String>,
    ) -> Result<ImportDiff, ModuleGraphError> {
        let node =
            self.modules
                .get_mut(module)
                .ok_or_else(|| ModuleGraphError::ModuleNotFound {
                    name: module.to_string(),
                    from: "update_module_imports".to_string(),
                })?;
        let diff = Self::diff_imports(&node.imports, &new_imports);
        node.imports = new_imports;
        self.rebuild_reverse_deps();
        Ok(diff)
    }

    pub fn remove_module(&mut self, module: &str) -> bool {
        let removed = self.modules.remove(module).is_some();
        if removed {
            self.file_map.remove(module);
            self.rebuild_reverse_deps();
        }
        removed
    }

    /// 親モジュール名を取得 ("A.B.C" -> Some("A.B"))
    pub fn parent_module(name: &str) -> Option<&str> {
        name.rfind('.').map(|pos| &name[..pos])
    }

    /// 直接の子モジュールを取得
    pub fn children(&self, parent: &str) -> Vec<&str> {
        let prefix = format!("{parent}.");
        self.modules
            .keys()
            .filter(|name| name.starts_with(&prefix) && !name[prefix.len()..].contains('.'))
            .map(|s| s.as_str())
            .collect()
    }

    /// 全子孫モジュールを取得
    pub fn descendants(&self, ancestor: &str) -> Vec<&str> {
        let prefix = format!("{ancestor}.");
        self.modules
            .keys()
            .filter(|name| name.starts_with(&prefix))
            .map(|s| s.as_str())
            .collect()
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
    fn to_snake_case(s: &str) -> String {
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

        // トポロジカルソート順に並べ替え
        let sorted = graph.topological_sort()?;
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
    fn to_pascal_case(s: &str) -> String {
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

    /// モジュールノードを取得
    pub fn get_module(&self, name: &str) -> Option<&ModuleNode> {
        self.modules.get(name)
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

fn is_formatter_trio_module(module: &str) -> bool {
    FORMATTER_TRIO_MODULES.contains(&module)
}

fn expand_changed_modules(changed: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    let mut seen = HashSet::new();

    for module in changed {
        if is_formatter_trio_module(module) {
            for trio_module in FORMATTER_TRIO_MODULES {
                let trio_module = trio_module.to_string();
                if seen.insert(trio_module.clone()) {
                    expanded.push(trio_module);
                }
            }
        } else if seen.insert(module.clone()) {
            expanded.push(module.clone());
        }
    }

    expanded
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = ModuleGraph::new();
        assert!(graph.is_empty());
        assert_eq!(graph.topological_sort().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn test_single_module() {
        let mut graph = ModuleGraph::new();
        graph.add_module("Main".to_string(), vec![], None).unwrap();
        assert_eq!(graph.len(), 1);
        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec!["Main"]);
    }

    #[test]
    fn test_linear_dependencies() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        graph
            .add_module("B".to_string(), vec!["A".to_string()], None)
            .unwrap();
        graph
            .add_module("C".to_string(), vec!["B".to_string()], None)
            .unwrap();

        let order = graph.topological_sort().unwrap();
        // A は B より前、B は C より前
        let pos_a = order.iter().position(|n| n == "A").unwrap();
        let pos_b = order.iter().position(|n| n == "B").unwrap();
        let pos_c = order.iter().position(|n| n == "C").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_diamond_dependencies() {
        let mut graph = ModuleGraph::new();
        graph.add_module("Base".to_string(), vec![], None).unwrap();
        graph
            .add_module("Left".to_string(), vec!["Base".to_string()], None)
            .unwrap();
        graph
            .add_module("Right".to_string(), vec!["Base".to_string()], None)
            .unwrap();
        graph
            .add_module(
                "Top".to_string(),
                vec!["Left".to_string(), "Right".to_string()],
                None,
            )
            .unwrap();

        let order = graph.topological_sort().unwrap();
        let pos_base = order.iter().position(|n| n == "Base").unwrap();
        let pos_left = order.iter().position(|n| n == "Left").unwrap();
        let pos_right = order.iter().position(|n| n == "Right").unwrap();
        let pos_top = order.iter().position(|n| n == "Top").unwrap();
        assert!(pos_base < pos_left);
        assert!(pos_base < pos_right);
        assert!(pos_left < pos_top);
        assert!(pos_right < pos_top);
    }

    #[test]
    fn test_reverse_dependency_closure_linear_chain() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        graph
            .add_module("B".to_string(), vec!["A".to_string()], None)
            .unwrap();
        graph
            .add_module("C".to_string(), vec!["B".to_string()], None)
            .unwrap();

        assert_eq!(graph.reverse_dependency_closure("A"), vec!["B", "C"]);
        assert_eq!(graph.reverse_dependency_closure("B"), vec!["C"]);
    }

    #[test]
    fn test_reverse_dependency_closure_diamond() {
        let mut graph = ModuleGraph::new();
        graph.add_module("Base".to_string(), vec![], None).unwrap();
        graph
            .add_module("Left".to_string(), vec!["Base".to_string()], None)
            .unwrap();
        graph
            .add_module("Right".to_string(), vec!["Base".to_string()], None)
            .unwrap();
        graph
            .add_module(
                "Top".to_string(),
                vec!["Left".to_string(), "Right".to_string()],
                None,
            )
            .unwrap();

        assert_eq!(
            graph.reverse_dependency_closure("Base"),
            vec!["Left", "Right", "Top"]
        );
    }

    #[test]
    fn test_reverse_dependency_closure_independent_module() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        graph
            .add_module("B".to_string(), vec!["A".to_string()], None)
            .unwrap();
        graph
            .add_module("Isolated".to_string(), vec![], None)
            .unwrap();

        assert_eq!(
            graph.reverse_dependency_closure("Isolated"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn test_compute_dirty_set_includes_reverse_dependents() {
        let mut graph = ModuleGraph::new();
        graph.add_module("Base".to_string(), vec![], None).unwrap();
        graph
            .add_module("Left".to_string(), vec!["Base".to_string()], None)
            .unwrap();
        graph
            .add_module("Right".to_string(), vec!["Base".to_string()], None)
            .unwrap();
        graph
            .add_module(
                "Top".to_string(),
                vec!["Left".to_string(), "Right".to_string()],
                None,
            )
            .unwrap();

        assert_eq!(
            graph.compute_dirty_set(&["Base".to_string()]),
            vec!["Base", "Left", "Right", "Top"]
        );
    }

    #[test]
    fn test_diff_imports_reports_added_and_removed_modules() {
        let diff = ModuleGraph::diff_imports(
            &["Base".to_string(), "Left".to_string()],
            &["Left".to_string(), "Right".to_string()],
        );

        assert_eq!(diff.added, vec!["Right"]);
        assert_eq!(diff.removed, vec!["Base"]);
    }

    #[test]
    fn test_update_module_imports_rebuilds_reverse_deps_and_topological_sort() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        graph.add_module("B".to_string(), vec![], None).unwrap();
        graph
            .add_module("C".to_string(), vec!["B".to_string()], None)
            .unwrap();

        graph
            .update_module_imports("B", vec!["A".to_string()])
            .unwrap();

        let order = graph.topological_sort().unwrap();
        let pos_a = order.iter().position(|n| n == "A").unwrap();
        let pos_b = order.iter().position(|n| n == "B").unwrap();
        let pos_c = order.iter().position(|n| n == "C").unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
        assert_eq!(graph.reverse_dependency_closure("A"), vec!["B", "C"]);
    }

    #[test]
    fn test_remove_module_updates_reverse_deps_and_dirty_set() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        graph
            .add_module("B".to_string(), vec!["A".to_string()], None)
            .unwrap();
        graph
            .add_module("C".to_string(), vec!["B".to_string()], None)
            .unwrap();

        assert!(graph.remove_module("C"));
        assert_eq!(graph.topological_sort().unwrap(), vec!["A", "B"]);
        assert_eq!(graph.reverse_dependency_closure("A"), vec!["B"]);
        assert_eq!(graph.compute_dirty_set(&["A".to_string()]), vec!["A", "B"]);
    }

    #[test]
    fn test_compute_dirty_set_expands_formatter_trio_atomically() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module(FORMATTER_TRIO_EXPR.to_string(), vec![], None)
            .unwrap();
        graph
            .add_module(FORMATTER_TRIO_DECL.to_string(), vec![], None)
            .unwrap();
        graph
            .add_module(FORMATTER_TRIO_MAIN.to_string(), vec![], None)
            .unwrap();
        graph
            .add_module(
                "Consumer".to_string(),
                vec![FORMATTER_TRIO_MAIN.to_string()],
                None,
            )
            .unwrap();

        assert_eq!(
            graph.compute_dirty_set(&[FORMATTER_TRIO_DECL.to_string()]),
            vec![
                FORMATTER_TRIO_EXPR,
                FORMATTER_TRIO_DECL,
                FORMATTER_TRIO_MAIN,
                "Consumer",
            ]
        );
    }

    /// マルチファイル compile の Wasm 決定性: HashMap 走査順に依存しないこと
    #[test]
    fn test_topological_sort_stable_across_calls() {
        let mut graph = ModuleGraph::new();
        graph.add_module("Z".to_string(), vec![], None).unwrap();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        graph
            .add_module(
                "M".to_string(),
                vec!["Z".to_string(), "A".to_string()],
                None,
            )
            .unwrap();

        let o1 = graph.topological_sort().unwrap();
        let o2 = graph.topological_sort().unwrap();
        assert_eq!(o1, o2);
        // import 名ソートにより A → Z → M（M の依存先を辞書順に処理）
        assert_eq!(o1, vec!["A", "Z", "M"]);
    }

    #[test]
    fn test_cyclic_dependency_detection() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module("A".to_string(), vec!["B".to_string()], None)
            .unwrap();
        graph
            .add_module("B".to_string(), vec!["A".to_string()], None)
            .unwrap();

        assert!(graph.detect_cycles().is_some());
        assert!(graph.topological_sort().is_err());
    }

    #[test]
    fn test_three_way_cycle() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module("A".to_string(), vec!["B".to_string()], None)
            .unwrap();
        graph
            .add_module("B".to_string(), vec!["C".to_string()], None)
            .unwrap();
        graph
            .add_module("C".to_string(), vec!["A".to_string()], None)
            .unwrap();

        let cycle = graph.detect_cycles().unwrap();
        assert!(cycle.len() > 2); // 少なくとも 3 ノードの循環
    }

    #[test]
    fn test_duplicate_module_error() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        let result = graph.add_module("A".to_string(), vec![], None);
        assert!(result.is_err());
    }

    #[test]
    fn test_missing_import_check() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module("Main".to_string(), vec!["Missing".to_string()], None)
            .unwrap();

        let errors = graph.check_imports();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            &errors[0],
            ModuleGraphError::ModuleNotFound { name, .. } if name == "Missing"
        ));
    }

    #[test]
    fn test_file_path_mapping() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module(
                "Utils".to_string(),
                vec![],
                Some("src/utils.ls".to_string()),
            )
            .unwrap();

        assert_eq!(graph.module_for_file("src/utils.ls"), Some("Utils"));
        assert_eq!(graph.module_for_file("src/other.ls"), None);
    }
}

#[cfg(test)]
mod nested_module_tests {
    use super::*;

    #[test]
    fn test_nested_module_name() {
        let mut graph = ModuleGraph::new();
        graph.add_module("App".to_string(), vec![], None).unwrap();
        graph
            .add_module("App.Utils".to_string(), vec![], None)
            .unwrap();
        graph
            .add_module(
                "App.Models".to_string(),
                vec!["App.Utils".to_string()],
                None,
            )
            .unwrap();

        let order = graph.topological_sort().unwrap();
        let pos_utils = order.iter().position(|n| n == "App.Utils").unwrap();
        let pos_models = order.iter().position(|n| n == "App.Models").unwrap();
        assert!(pos_utils < pos_models);
    }

    #[test]
    fn test_nested_module_depth() {
        let mut graph = ModuleGraph::new();
        graph.add_module("A".to_string(), vec![], None).unwrap();
        graph
            .add_module("A.B".to_string(), vec!["A".to_string()], None)
            .unwrap();
        graph
            .add_module("A.B.C".to_string(), vec!["A.B".to_string()], None)
            .unwrap();

        let order = graph.topological_sort().unwrap();
        let pos_a = order.iter().position(|n| n == "A").unwrap();
        let pos_ab = order.iter().position(|n| n == "A.B").unwrap();
        let pos_abc = order.iter().position(|n| n == "A.B.C").unwrap();
        assert!(pos_a < pos_ab);
        assert!(pos_ab < pos_abc);
    }

    #[test]
    fn test_nested_module_cyclic() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module("A.B".to_string(), vec!["A.C".to_string()], None)
            .unwrap();
        graph
            .add_module("A.C".to_string(), vec!["A.B".to_string()], None)
            .unwrap();

        assert!(graph.detect_cycles().is_some());
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    #[test]
    fn test_module_name_to_paths_simple() {
        let paths = ModuleGraph::module_name_to_paths("Utils");
        assert!(paths.contains(&"Utils.ls".to_string()));
        assert!(paths.contains(&"utils.ls".to_string()));
    }

    #[test]
    fn test_module_name_to_paths_pascal_case() {
        let paths = ModuleGraph::module_name_to_paths("MathUtils");
        assert!(paths.contains(&"MathUtils.ls".to_string()));
        assert!(paths.contains(&"math_utils.ls".to_string()));
    }

    #[test]
    fn test_module_name_to_paths_nested() {
        let paths = ModuleGraph::module_name_to_paths("App.Utils");
        assert!(paths.contains(&"App/Utils.ls".to_string()));
        assert!(paths.contains(&"app/utils.ls".to_string()));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(ModuleGraph::to_snake_case("Utils"), "utils");
        assert_eq!(ModuleGraph::to_snake_case("MathUtils"), "math_utils");
        assert_eq!(ModuleGraph::to_snake_case("A"), "a");
        assert_eq!(ModuleGraph::to_snake_case("abc"), "abc");
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(ModuleGraph::to_pascal_case("utils"), "Utils");
        assert_eq!(ModuleGraph::to_pascal_case("math_utils"), "MathUtils");
        assert_eq!(ModuleGraph::to_pascal_case("main"), "Main");
    }

    #[test]
    fn test_resolve_module_file_not_found() {
        let result = ModuleGraph::resolve_module_file(
            "NonExistent",
            std::path::Path::new("/tmp/lsharp_nonexistent"),
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_module_file_found() {
        // 一時ディレクトリにファイルを作成してテスト
        let dir = std::env::temp_dir().join("lsharp_resolve_test");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Utils.ls"), "(module Utils)\n(defn helper [x] x)").unwrap();

        let result = ModuleGraph::resolve_module_file("Utils", &dir);
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("Utils.ls"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_resolve_module_file_snake_case() {
        // snake_case ファイル名でも探索可能
        let dir = std::env::temp_dir().join("lsharp_resolve_snake");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("math_utils.ls"),
            "(module MathUtils)\n(defn add [x y] (+ x y))",
        )
        .unwrap();

        let result = ModuleGraph::resolve_module_file("MathUtils", &dir);
        assert!(result.is_some());
        assert!(result.unwrap().ends_with("math_utils.ls"));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_from_entry_single_file() {
        let dir = std::env::temp_dir().join("lsharp_build_single");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("main.ls"),
            "(module Main)\n(defn main [] (print 42))",
        )
        .unwrap();

        let (graph, files) = ModuleGraph::build_from_entry(&dir.join("main.ls")).unwrap();
        assert_eq!(graph.len(), 1);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, "Main");

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_from_entry_with_import() {
        let dir = std::env::temp_dir().join("lsharp_build_import");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Utils.ls"),
            "(module Utils)\n(defn helper [x] (+ x 1))",
        )
        .unwrap();
        std::fs::write(
            dir.join("main.ls"),
            "(module Main)\n(import Utils)\n(defn main [] (print (helper 41)))",
        )
        .unwrap();

        let (graph, files) = ModuleGraph::build_from_entry(&dir.join("main.ls")).unwrap();
        assert_eq!(graph.len(), 2);
        assert_eq!(files.len(), 2);
        // トポロジカルソート順: Utils が Main より前
        let pos_utils = files.iter().position(|(n, _)| n == "Utils").unwrap();
        let pos_main = files.iter().position(|(n, _)| n == "Main").unwrap();
        assert!(pos_utils < pos_main);

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_from_entry_missing_import() {
        let dir = std::env::temp_dir().join("lsharp_build_missing");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("main.ls"),
            "(module Main)\n(import NonExistent)\n(defn main [] (print 1))",
        )
        .unwrap();

        let result = ModuleGraph::build_from_entry(&dir.join("main.ls"));
        assert!(result.is_err());

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_from_entry_prefers_package_src_root() {
        let dir = std::env::temp_dir().join("lsharp_build_package_src_root");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("examples/demo")).unwrap();
        std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"demo\"\n").unwrap();
        std::fs::write(
            dir.join("src/Utils.ls"),
            "(module Utils)\n(defn helper [x] (+ x 1))",
        )
        .unwrap();
        std::fs::write(
            dir.join("examples/demo/Main.ls"),
            "(module Main)\n(import Utils)\n(defn main [] (helper 1))",
        )
        .unwrap();

        let (graph, files) =
            ModuleGraph::build_from_entry(&dir.join("examples/demo/Main.ls")).unwrap();
        assert_eq!(graph.len(), 2);
        assert!(
            files
                .iter()
                .any(|(name, path)| name == "Utils" && path.ends_with("src/Utils.ls"))
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_from_entry_prefers_nearest_src_ancestor_without_manifest() {
        let dir = std::env::temp_dir().join("lsharp_build_nearest_src_ancestor");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("selfhost/src/App")).unwrap();
        std::fs::create_dir_all(dir.join("selfhost/src/Syntax")).unwrap();
        std::fs::write(
            dir.join("selfhost/src/Syntax/Token.ls"),
            "(module Syntax.Token)\n(defn token-tag [] 1)",
        )
        .unwrap();
        std::fs::write(
            dir.join("selfhost/src/App/Main.ls"),
            "(module App.Main)\n(import Syntax.Token)\n(defn main [] (token-tag))",
        )
        .unwrap();

        let (graph, files) =
            ModuleGraph::build_from_entry(&dir.join("selfhost/src/App/Main.ls")).unwrap();
        assert_eq!(graph.len(), 2);
        assert!(files.iter().any(|(name, path)| {
            name == "Syntax.Token" && path.ends_with("selfhost/src/Syntax/Token.ls")
        }));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_resolve_module_file_with_search_paths_uses_packages_then_stdlib() {
        let dir = std::env::temp_dir().join("lsharp_resolve_search_paths");
        let _ = std::fs::remove_dir_all(&dir);
        let pkg_src = dir.join(".lsharp/packages/demo-123/src");
        let stdlib = dir.join("custom-stdlib");
        std::fs::create_dir_all(&pkg_src).unwrap();
        std::fs::create_dir_all(&stdlib).unwrap();
        std::fs::write(
            pkg_src.join("Helpers.ls"),
            "(module Helpers)\n(defn helper [] 1)",
        )
        .unwrap();
        std::fs::write(
            stdlib.join("List.ls"),
            "(module List)\n(defn length [xs] 0)",
        )
        .unwrap();

        let search_paths = ModuleSearchPaths {
            package_root: dir.clone(),
            source_root: dir.join("src"),
            package_sources: vec![pkg_src],
            stdlib_root: Some(stdlib.clone()),
        };

        let pkg_result =
            ModuleGraph::resolve_module_file_with_search_paths("Helpers", &search_paths);
        let stdlib_result =
            ModuleGraph::resolve_module_file_with_search_paths("List", &search_paths);

        assert!(
            pkg_result
                .as_ref()
                .is_some_and(|path| path.ends_with("Helpers.ls"))
        );
        assert!(
            stdlib_result
                .as_ref()
                .is_some_and(|path| path.ends_with("List.ls"))
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_from_entry_resolves_packages_from_project_root() {
        let dir = std::env::temp_dir().join("lsharp_build_project_root_packages");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("examples/demo")).unwrap();
        std::fs::create_dir_all(dir.join(".lsharp/packages/pkg-123/src")).unwrap();
        std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"demo\"\n").unwrap();
        std::fs::write(
            dir.join(".lsharp/packages/pkg-123/src/Helpers.ls"),
            "(module Helpers)\n(defn helper [] 1)",
        )
        .unwrap();
        std::fs::write(
            dir.join("examples/demo/Main.ls"),
            "(module Main)\n(import Helpers)\n(defn main [] (helper))",
        )
        .unwrap();

        let (graph, files) =
            ModuleGraph::build_from_entry(&dir.join("examples/demo/Main.ls")).unwrap();

        assert_eq!(graph.len(), 2);
        assert!(files.iter().any(|(name, path)| {
            name == "Helpers" && path.ends_with(".lsharp/packages/pkg-123/src/Helpers.ls")
        }));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn test_build_from_entry_rejects_non_exported_package_module() {
        let dir = std::env::temp_dir().join("lsharp_build_non_exported_package_module");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join(".lsharp/packages/demo-123/src")).unwrap();
        std::fs::write(dir.join("lsharp.toml"), "[project]\nname=\"app\"\n").unwrap();
        std::fs::write(
            dir.join(".lsharp/packages/demo-123/lsharp.toml"),
            "[project]\nname=\"demo\"\n[project.exports]\nmodules=[\"Public\"]\n",
        )
        .unwrap();
        std::fs::write(
            dir.join(".lsharp/packages/demo-123/src/Hidden.ls"),
            "(module Hidden)\n(defn helper [] 1)",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/Main.ls"),
            "(module Main)\n(import Hidden)\n(defn main [] 0)",
        )
        .unwrap();

        let result = ModuleGraph::build_from_entry(&dir.join("src/Main.ls"));
        let _ = std::fs::remove_dir_all(&dir);

        assert!(
            result.is_err(),
            "非公開 package module の import は失敗するべき"
        );
    }
}

#[cfg(test)]
mod hierarchy_tests {
    use super::*;

    #[test]
    fn test_parent_module() {
        assert_eq!(ModuleGraph::parent_module("A.B.C"), Some("A.B"));
        assert_eq!(ModuleGraph::parent_module("A.B"), Some("A"));
        assert_eq!(ModuleGraph::parent_module("A"), None);
    }

    #[test]
    fn test_children() {
        let mut graph = ModuleGraph::new();
        graph.add_module("App".to_string(), vec![], None).unwrap();
        graph
            .add_module("App.Utils".to_string(), vec![], None)
            .unwrap();
        graph
            .add_module("App.Models".to_string(), vec![], None)
            .unwrap();
        graph
            .add_module("App.Models.User".to_string(), vec![], None)
            .unwrap();

        let mut children = graph.children("App");
        children.sort();
        assert_eq!(children, vec!["App.Models", "App.Utils"]);

        let children_models = graph.children("App.Models");
        assert_eq!(children_models, vec!["App.Models.User"]);
    }

    #[test]
    fn test_descendants() {
        let mut graph = ModuleGraph::new();
        graph.add_module("App".to_string(), vec![], None).unwrap();
        graph
            .add_module("App.Utils".to_string(), vec![], None)
            .unwrap();
        graph
            .add_module("App.Models".to_string(), vec![], None)
            .unwrap();
        graph
            .add_module("App.Models.User".to_string(), vec![], None)
            .unwrap();

        let mut desc = graph.descendants("App");
        desc.sort();
        assert_eq!(desc, vec!["App.Models", "App.Models.User", "App.Utils"]);
    }
}
