//! モジュールグラフ
//!
//! 複数モジュール間の依存関係を管理し、
//! コンパイル順序の決定（トポロジカルソート）と循環依存の検出を行う。

use std::collections::{HashMap, HashSet};

use lsharp_syntax::span::Span;

mod mutation;
#[cfg(test)]
mod mutation_tests;
mod resolve;
mod scc;
pub use resolve::ModuleSearchPaths;

pub const FORMATTER_TRIO_EXPR: &str = "Tools.Text.FormatterExpr";
pub const FORMATTER_TRIO_DECL: &str = "Tools.Text.FormatterDecl";
pub const FORMATTER_TRIO_MAIN: &str = "Tools.Text.Formatter";
const FORMATTER_TRIO_MODULES: [&str; 3] = [
    FORMATTER_TRIO_EXPR,
    FORMATTER_TRIO_DECL,
    FORMATTER_TRIO_MAIN,
];

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

    #[error("モジュール '{name}' が見つかりません ('{from}' からインポート) ({span})")]
    ModuleNotFoundAt {
        name: String,
        from: String,
        span: Span,
    },

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
            Self::ModuleNotFound { .. } | Self::ModuleNotFoundAt { .. } => "LS3102",
            Self::ModuleNotExported { .. } => "LS3103",
            Self::DuplicateModule { .. } => "LS3104",
        }
    }

    pub fn span(&self) -> Option<Span> {
        match self {
            Self::ModuleNotFoundAt { span, .. } => Some(*span),
            _ => None,
        }
    }
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self::default()
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
        scc::compute_groups(&self.modules)
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

    /// モジュールノードを取得
    pub fn get_module(&self, name: &str) -> Option<&ModuleNode> {
        self.modules.get(name)
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

#[cfg(test)]
mod scc_tests;
