use std::collections::{HashMap, HashSet};

use super::ModuleNode;

#[derive(Default)]
struct SccState {
    next_index: usize,
    indices: HashMap<String, usize>,
    lowlinks: HashMap<String, usize>,
    stack: Vec<String>,
    on_stack: HashSet<String>,
}

/// Tarjan の強連結成分を、依存先が先に来る安定した順序で返す。
pub(super) fn compute_groups(modules: &HashMap<String, ModuleNode>) -> Vec<Vec<String>> {
    let mut module_names: Vec<String> = modules.keys().cloned().collect();
    module_names.sort();

    let mut state = SccState::default();
    let mut groups = Vec::new();

    for name in module_names {
        if !state.indices.contains_key(&name) {
            visit(modules, &name, &mut state, &mut groups);
        }
    }

    groups
}

fn visit(
    modules: &HashMap<String, ModuleNode>,
    node: &str,
    state: &mut SccState,
    groups: &mut Vec<Vec<String>>,
) {
    let index = state.next_index;
    state.next_index += 1;
    state.indices.insert(node.to_string(), index);
    state.lowlinks.insert(node.to_string(), index);
    state.stack.push(node.to_string());
    state.on_stack.insert(node.to_string());

    let mut imports = modules
        .get(node)
        .map(|module| module.imports.clone())
        .unwrap_or_default();
    imports.sort();

    for import in imports {
        if !modules.contains_key(&import) {
            continue;
        }
        if !state.indices.contains_key(&import) {
            visit(modules, &import, state, groups);
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
