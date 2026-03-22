//! モジュールグラフ
//!
//! 複数モジュール間の依存関係を管理し、
//! コンパイル順序の決定（トポロジカルソート）と循環依存の検出を行う。

use std::collections::{HashMap, HashSet};

/// モジュールグラフ
#[derive(Debug, Default)]
pub struct ModuleGraph {
    /// モジュール名 -> モジュール情報
    modules: HashMap<String, ModuleNode>,
    /// モジュール名 -> ファイルパス
    file_map: HashMap<String, String>,
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

/// モジュールグラフのエラー
#[derive(Debug, Clone, thiserror::Error)]
pub enum ModuleGraphError {
    #[error("循環依存が検出されました: {cycle}")]
    CyclicDependency { cycle: String },

    #[error("モジュール '{name}' が見つかりません ('{from}' からインポート)")]
    ModuleNotFound { name: String, from: String },

    #[error("モジュール '{name}' が重複しています")]
    DuplicateModule { name: String },
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
            if !visited.contains(name) {
                if let Some(cycle) = self.dfs_detect_cycle(name, &mut visited, &mut in_stack, &mut path) {
                    return Some(cycle);
                }
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

        for name in self.modules.keys() {
            if !visited.contains(name) {
                self.topo_dfs(name, &mut visited, &mut order);
            }
        }

        Ok(order)
    }

    /// トポロジカルソートの DFS
    fn topo_dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) {
        if visited.contains(node) {
            return;
        }
        visited.insert(node.to_string());

        if let Some(module) = self.modules.get(node) {
            for import in &module.imports {
                self.topo_dfs(import, visited, order);
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


    /// 親モジュール名を取得 ("A.B.C" -> Some("A.B"))
    pub fn parent_module(name: &str) -> Option<&str> {
        name.rfind('.').map(|pos| &name[..pos])
    }

    /// 直接の子モジュールを取得
    pub fn children(&self, parent: &str) -> Vec<&str> {
        let prefix = format!("{parent}.");
        self.modules
            .keys()
            .filter(|name| {
                name.starts_with(&prefix)
                    && !name[prefix.len()..].contains('.')
            })
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
        graph
            .add_module("Main".to_string(), vec![], None)
            .unwrap();
        assert_eq!(graph.len(), 1);
        let order = graph.topological_sort().unwrap();
        assert_eq!(order, vec!["Main"]);
    }

    #[test]
    fn test_linear_dependencies() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module("A".to_string(), vec![], None)
            .unwrap();
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
        graph
            .add_module("Base".to_string(), vec![], None)
            .unwrap();
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
        graph
            .add_module("A".to_string(), vec![], None)
            .unwrap();
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
        graph
            .add_module("App".to_string(), vec![], None)
            .unwrap();
        graph
            .add_module("App.Utils".to_string(), vec![], None)
            .unwrap();
        graph
            .add_module("App.Models".to_string(), vec!["App.Utils".to_string()], None)
            .unwrap();

        let order = graph.topological_sort().unwrap();
        let pos_utils = order.iter().position(|n| n == "App.Utils").unwrap();
        let pos_models = order.iter().position(|n| n == "App.Models").unwrap();
        assert!(pos_utils < pos_models);
    }

    #[test]
    fn test_nested_module_depth() {
        let mut graph = ModuleGraph::new();
        graph
            .add_module("A".to_string(), vec![], None)
            .unwrap();
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
        graph.add_module("App.Utils".to_string(), vec![], None).unwrap();
        graph.add_module("App.Models".to_string(), vec![], None).unwrap();
        graph.add_module("App.Models.User".to_string(), vec![], None).unwrap();

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
        graph.add_module("App.Utils".to_string(), vec![], None).unwrap();
        graph.add_module("App.Models".to_string(), vec![], None).unwrap();
        graph.add_module("App.Models.User".to_string(), vec![], None).unwrap();

        let mut desc = graph.descendants("App");
        desc.sort();
        assert_eq!(desc, vec!["App.Models", "App.Models.User", "App.Utils"]);
    }
}
