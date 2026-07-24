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

#[derive(Default)]
struct SccState {
    next_index: usize,
    indices: HashMap<String, usize>,
    lowlinks: HashMap<String, usize>,
    stack: Vec<String>,
    on_stack: HashSet<String>,
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

impl ModuleGraphError {
    /// 利用者向けの安定した診断コードを返す。
    pub fn code(&self) -> &'static str {
        match self {
            Self::CyclicDependency { .. } => "LS3101",
            Self::ModuleNotFound { .. } => "LS3102",
            Self::ModuleNotExported { .. } => "LS3103",
            Self::DuplicateModule { .. } => "LS3104",
        }
    }
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

        let mut module_names: Vec<&String> = self.modules.keys().collect();
        module_names.sort();
        for name in module_names {
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
            let mut imports = module.imports.clone();
            imports.sort();
            for import in &imports {
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

    /// 強連結成分を、依存先が先に来る安定した順序で返す。
    ///
    /// import edge は「module -> dependency」を向くため、Tarjan の出力順は
    /// compile 用の dependency-first order（設計書でいう reverse topological order）になる。
    /// 各 SCC 内と DFS の開始/import 順は module 名で安定化し、未解決 import は
    /// `check_imports` の責務としてグラフへ暗黙に追加しない。
    pub fn scc_groups(&self) -> Vec<Vec<String>> {
        let mut module_names: Vec<String> = self.modules.keys().cloned().collect();
        module_names.sort();

        let mut state = SccState::default();
        let mut groups = Vec::new();

        for name in module_names {
            if !state.indices.contains_key(&name) {
                self.scc_visit(&name, &mut state, &mut groups);
            }
        }

        groups
    }

    fn scc_visit(&self, node: &str, state: &mut SccState, groups: &mut Vec<Vec<String>>) {
        let index = state.next_index;
        state.next_index += 1;
        state.indices.insert(node.to_string(), index);
        state.lowlinks.insert(node.to_string(), index);
        state.stack.push(node.to_string());
        state.on_stack.insert(node.to_string());

        let mut imports = self
            .modules
            .get(node)
            .map(|module| module.imports.clone())
            .unwrap_or_default();
        imports.sort();

        for import in imports {
            if !self.modules.contains_key(&import) {
                continue;
            }
            if !state.indices.contains_key(&import) {
                self.scc_visit(&import, state, groups);
                let child_lowlink = state.lowlinks[&import];
                let current_lowlink = state.lowlinks[node];
                state
                    .lowlinks
                    .insert(node.to_string(), current_lowlink.min(child_lowlink));
            } else if state.on_stack.contains(&import) {
                let import_index = state.indices[&import];
                let current_lowlink = state.lowlinks[node];
                state
                    .lowlinks
                    .insert(node.to_string(), current_lowlink.min(import_index));
            }
        }

        if state.lowlinks[node] == state.indices[node] {
            let mut group = Vec::new();
            loop {
                let member = state
                    .stack
                    .pop()
                    .expect("SCC root must have a member on the stack");
                state.on_stack.remove(&member);
                group.push(member.clone());
                if member == node {
                    break;
                }
            }
            group.sort();
            groups.push(group);
        }
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

        let mut module_names: Vec<&String> = self.modules.keys().collect();
        module_names.sort();
        for name in module_names {
            let Some(module) = self.modules.get(name) else {
                continue;
            };
            let mut imports = module.imports.clone();
            imports.sort();
            for import in imports {
                if !self.modules.contains_key(&import) {
                    errors.push(ModuleGraphError::ModuleNotFound {
                        name: import,
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
mod tests;

#[cfg(test)]
mod nested_module_tests;

#[cfg(test)]
mod resolve_tests;

#[cfg(test)]
mod hierarchy_tests;
