//! コンパイル時の import 可視性と型 surface の集約。
//!
//! モジュール解析・incremental cache が共有する surface のキーと、
//! import の宣言を収集する責務を `lib.rs` のパイプライン本体から分離する。

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use lsharp_types::infer::ExprTypeKey;

use super::CompilationCache;

#[derive(Debug, Clone, Default)]
pub(super) struct ImportVisibilitySpec {
    pub(super) only: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModuleTypeSurface {
    pub(super) results: Vec<(String, lsharp_types::types::TypeScheme)>,
    pub(super) hidden: HashSet<String>,
    pub(super) expr_types: HashMap<ExprTypeKey, lsharp_types::types::Type>,
}

impl ModuleTypeSurface {
    pub(super) fn export_surface_eq(&self, other: &Self) -> bool {
        self.results == other.results && self.hidden == other.hidden
    }
}

pub(super) fn type_surface_key(surface: &ModuleTypeSurface) -> u64 {
    let mut results = surface.results.clone();
    results.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hidden = surface.hidden.iter().cloned().collect::<Vec<_>>();
    hidden.sort();

    let mut hasher = DefaultHasher::new();
    results.hash(&mut hasher);
    hidden.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn dependency_surface_key(
    direct_imports: &HashMap<String, ImportVisibilitySpec>,
    current_surfaces: &HashMap<String, ModuleTypeSurface>,
    cache: &CompilationCache,
) -> u64 {
    let mut dependencies = direct_imports.keys().cloned().collect::<Vec<_>>();
    dependencies.sort();

    let mut hasher = DefaultHasher::new();
    for dependency in dependencies {
        dependency.hash(&mut hasher);
        if let Some(surface) = current_surfaces.get(&dependency) {
            type_surface_key(surface).hash(&mut hasher);
        } else if let Some(entry) = cache.get(&dependency) {
            type_surface_key(&entry.type_surface_clone()).hash(&mut hasher);
        } else {
            0u8.hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub(super) fn push_defn_origins_infer_order(
    decls: &[lsharp_syntax::ast::Decl],
    file_module: &str,
    module_prefix: Option<&str>,
    out: &mut Vec<String>,
) {
    use lsharp_syntax::ast::Decl;
    for decl in decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        match actual_decl {
            Decl::Defn { .. } => out.push(file_module.to_string()),
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let prefix = if let Some(outer) = module_prefix {
                    format!("{outer}.{name}")
                } else {
                    name.clone()
                };
                push_defn_origins_infer_order(body, file_module, Some(prefix.as_str()), out);
            }
            _ => {}
        }
    }
}

pub(super) fn collect_import_visibility(
    program: &lsharp_syntax::ast::Program,
) -> HashMap<String, ImportVisibilitySpec> {
    let mut imports = HashMap::new();
    for decl in &program.decls {
        if let lsharp_syntax::ast::Decl::ImportDecl { module, only, .. } = decl {
            let entry = imports
                .entry(module.clone())
                .or_insert_with(ImportVisibilitySpec::default);
            match (&mut entry.only, only.as_ref()) {
                (None, None) => {}
                (slot @ None, Some(next)) => {
                    *slot = Some(next.clone());
                }
                (Some(existing), Some(next)) => {
                    for symbol in next {
                        if !existing.contains(symbol) {
                            existing.push(symbol.clone());
                        }
                    }
                }
                (Some(_), None) => {
                    entry.only = None;
                }
            }
        }
    }
    imports
}

pub(super) fn collect_import_modules(program: &lsharp_syntax::ast::Program) -> Vec<String> {
    let mut imports = Vec::new();
    let mut seen = HashSet::new();
    for decl in &program.decls {
        if let lsharp_syntax::ast::Decl::ImportDecl { module, .. } = decl
            && seen.insert(module.clone())
        {
            imports.push(module.clone());
        }
    }
    imports
}
