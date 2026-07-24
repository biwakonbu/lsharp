use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lsharp_syntax::ast::Program;

use crate::{Module, ModuleTypeSurface, SourceFingerprint};

fn empty_module() -> Module {
    Module {
        functions: Vec::new(),
        gc_types: Vec::new(),
        imports: Vec::new(),
        globals: Vec::new(),
        string_data: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub struct ModuleIrSegments {
    defns: Module,
    accessors: Module,
    trait_impls: Module,
    constraints: Module,
    ctors: Module,
    defn_lifted: Module,
    trait_impl_lifted: Module,
}

impl ModuleIrSegments {
    pub(crate) fn empty() -> Self {
        Self {
            defns: empty_module(),
            accessors: empty_module(),
            trait_impls: empty_module(),
            constraints: empty_module(),
            ctors: empty_module(),
            defn_lifted: empty_module(),
            trait_impl_lifted: empty_module(),
        }
    }

    pub(crate) fn defns(&self) -> &Module {
        &self.defns
    }

    pub(crate) fn accessors(&self) -> &Module {
        &self.accessors
    }

    pub(crate) fn trait_impls(&self) -> &Module {
        &self.trait_impls
    }

    pub(crate) fn constraints(&self) -> &Module {
        &self.constraints
    }

    pub(crate) fn ctors(&self) -> &Module {
        &self.ctors
    }

    pub(crate) fn defn_lifted(&self) -> &Module {
        &self.defn_lifted
    }

    pub(crate) fn trait_impl_lifted(&self) -> &Module {
        &self.trait_impl_lifted
    }

    pub(crate) fn set_defns(&mut self, module: Module) {
        self.defns = module;
    }

    pub(crate) fn set_accessors(&mut self, module: Module) {
        self.accessors = module;
    }

    pub(crate) fn set_trait_impls(&mut self, module: Module) {
        self.trait_impls = module;
    }

    pub(crate) fn set_constraints(&mut self, module: Module) {
        self.constraints = module;
    }

    pub(crate) fn set_ctors(&mut self, module: Module) {
        self.ctors = module;
    }

    pub(crate) fn set_defn_lifted(&mut self, module: Module) {
        self.defn_lifted = module;
    }

    pub(crate) fn set_trait_impl_lifted(&mut self, module: Module) {
        self.trait_impl_lifted = module;
    }

    pub(crate) fn is_empty(&self) -> bool {
        [
            &self.defns,
            &self.accessors,
            &self.trait_impls,
            &self.constraints,
            &self.ctors,
            &self.defn_lifted,
            &self.trait_impl_lifted,
        ]
        .into_iter()
        .all(|module| {
            module.functions.is_empty()
                && module.gc_types.is_empty()
                && module.string_data.is_empty()
                && module.imports.is_empty()
                && module.globals.is_empty()
        })
    }
}

/// モジュール単位の incremental compile キャッシュ。
#[derive(Debug, Clone)]
pub struct ModuleCacheEntry {
    fingerprint: SourceFingerprint,
    ast: Arc<Program>,
    type_surface: ModuleTypeSurface,
    ir: Module,
    ir_segments: ModuleIrSegments,
    imports: Vec<String>,
}

impl ModuleCacheEntry {
    pub(crate) fn new(
        fingerprint: SourceFingerprint,
        ast: Arc<Program>,
        type_surface: ModuleTypeSurface,
        ir: Module,
        ir_segments: ModuleIrSegments,
        imports: Vec<String>,
    ) -> Self {
        Self {
            fingerprint,
            ast,
            type_surface,
            ir,
            ir_segments,
            imports,
        }
    }

    pub fn fingerprint(&self) -> SourceFingerprint {
        self.fingerprint
    }

    pub fn ast(&self) -> &Program {
        self.ast.as_ref()
    }

    pub(crate) fn ast_arc(&self) -> Arc<Program> {
        Arc::clone(&self.ast)
    }

    pub fn imports(&self) -> &[String] {
        &self.imports
    }

    pub fn ir(&self) -> &Module {
        &self.ir
    }

    pub(crate) fn ir_segments(&self) -> &ModuleIrSegments {
        &self.ir_segments
    }

    pub fn type_result_len(&self) -> usize {
        self.type_surface.results.len()
    }

    pub(crate) fn type_surface_clone(&self) -> ModuleTypeSurface {
        self.type_surface.clone()
    }

    pub(crate) fn set_ir(&mut self, ir: Module) {
        self.ir = ir;
    }

    pub(crate) fn set_ir_segments(&mut self, ir_segments: ModuleIrSegments) {
        self.ir_segments = ir_segments;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LinkedModuleCache {
    module_order: Vec<String>,
    final_module: Module,
}

impl LinkedModuleCache {
    pub(crate) fn new(module_order: Vec<String>, final_module: Module) -> Self {
        Self {
            module_order,
            final_module,
        }
    }

    pub(crate) fn module_order(&self) -> &[String] {
        &self.module_order
    }

    pub(crate) fn final_module(&self) -> &Module {
        &self.final_module
    }
}

/// モジュール名ごとのコンパイル中間成果物キャッシュ。
#[derive(Debug, Clone, Default)]
pub struct CompilationCache {
    entries: HashMap<String, ModuleCacheEntry>,
    linked_module: Option<LinkedModuleCache>,
    entry_root: Option<PathBuf>,
}

impl CompilationCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// エントリの所在が変わったとき、別プロジェクトの module 名を再利用しない。
    ///
    /// キャッシュのキーは既存 caller との互換性のため module 名のままなので、同じ
    /// process で別の entry を compile する前に entry directory を scope として固定する。
    /// scope が変わった場合は module / link cache をまとめて破棄する。
    pub fn prepare_for_entry(&mut self, entry_file: &Path) {
        let entry_root = std::fs::canonicalize(entry_file)
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| entry_file.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        if self.entry_root.as_ref() == Some(&entry_root) {
            return;
        }
        self.entries.clear();
        self.linked_module = None;
        self.entry_root = Some(entry_root);
    }

    pub fn get(&self, module: &str) -> Option<&ModuleCacheEntry> {
        self.entries.get(module)
    }

    pub(crate) fn insert_module(&mut self, module: String, entry: ModuleCacheEntry) {
        self.entries.insert(module, entry);
    }

    pub fn remove_module(&mut self, module: &str) -> bool {
        let removed = self.entries.remove(module).is_some();
        if removed {
            self.linked_module = None;
        }
        removed
    }

    pub(crate) fn linked_module(&self) -> Option<&LinkedModuleCache> {
        self.linked_module.as_ref()
    }

    pub(crate) fn set_linked_module(&mut self, module_order: Vec<String>, final_module: Module) {
        self.linked_module = Some(LinkedModuleCache::new(module_order, final_module));
    }
}
