use std::collections::{HashMap, HashSet, VecDeque};

use super::{FORMATTER_TRIO_MODULES, ImportDiff, ModuleGraph, ModuleGraphError};

impl ModuleGraph {
    /// 指定モジュールの依存 closure を依存先優先の安定順で返す。
    pub fn dependency_closure(&self, module: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut deps = Vec::new();
        self.collect_dependencies(module, &mut visited, &mut deps);
        deps
    }

    /// 指定モジュールを参照しているモジュール closure を近い依存元から安定順で返す。
    pub fn reverse_dependency_closure(&self, module: &str) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut out = Vec::new();
        let mut queue = VecDeque::new();

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

    /// 変更モジュールと、その逆依存 closure を dirty set として返す。
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

    pub(super) fn rebuild_reverse_deps(&mut self) {
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
