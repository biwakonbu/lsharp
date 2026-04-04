use std::collections::HashMap;
use std::sync::Arc;

use lsharp_syntax::ast::Program;

use crate::{Module, ModuleTypeSurface, SourceFingerprint};

/// モジュール単位の incremental compile キャッシュ。
#[derive(Debug, Clone)]
pub struct ModuleCacheEntry {
    fingerprint: SourceFingerprint,
    ast: Arc<Program>,
    type_surface: ModuleTypeSurface,
    ir: Module,
    imports: Vec<String>,
}

impl ModuleCacheEntry {
    pub(crate) fn new(
        fingerprint: SourceFingerprint,
        ast: Arc<Program>,
        type_surface: ModuleTypeSurface,
        ir: Module,
        imports: Vec<String>,
    ) -> Self {
        Self {
            fingerprint,
            ast,
            type_surface,
            ir,
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

    pub fn type_result_len(&self) -> usize {
        self.type_surface.results.len()
    }

    pub(crate) fn type_surface_clone(&self) -> ModuleTypeSurface {
        self.type_surface.clone()
    }

    pub(crate) fn set_ir(&mut self, ir: Module) {
        self.ir = ir;
    }
}

/// モジュール名ごとのコンパイル中間成果物キャッシュ。
#[derive(Debug, Clone, Default)]
pub struct CompilationCache {
    entries: HashMap<String, ModuleCacheEntry>,
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

    pub fn get(&self, module: &str) -> Option<&ModuleCacheEntry> {
        self.entries.get(module)
    }

    pub(crate) fn insert_module(&mut self, module: String, entry: ModuleCacheEntry) {
        self.entries.insert(module, entry);
    }
}
