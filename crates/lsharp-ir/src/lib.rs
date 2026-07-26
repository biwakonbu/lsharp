//! L# 中間表現 (IR)
//!
//! MVP ではフラット化された命令列を使用。
//! 将来的に SSA 形式の BasicBlock ベースに拡張する。

pub mod cache;
pub mod closure;
mod instruction;
pub mod lower;
pub mod module_graph;
pub mod root_lifetime;

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use lsharp_types::infer::ExprTypeKey;
use sha2::{Digest, Sha256};

pub use cache::{CompilationCache, ModuleCacheEntry, ModuleIrSegments};
pub use instruction::{Instruction, IrType};

#[cfg(test)]
include!("incremental_trackers.rs");

/// SHA-256 ベースのソース fingerprint。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceFingerprint([u8; 32]);

impl SourceFingerprint {
    pub fn from_source(source: &str) -> Self {
        Self::from_bytes(source.as_bytes())
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let digest = hasher.finalize();
        Self(digest.into())
    }

    pub fn from_file(path: &std::path::Path) -> std::io::Result<Self> {
        let source = std::fs::read_to_string(path)?;
        Ok(Self::from_source(&source))
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for SourceFingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// IR モジュール（コンパイル単位）
#[derive(Debug, Clone)]
pub struct Module {
    pub functions: Vec<Function>,
    /// GC 型定義（WasmGC struct 用）
    pub gc_types: Vec<GcTypeDef>,
    /// import 関数定義
    pub imports: Vec<ImportFunc>,
    /// グローバル変数定義（辞書インスタンス等）
    pub globals: Vec<GlobalDef>,
    /// 文字列定数データ（data section 用）
    pub string_data: Vec<(String, Vec<u8>)>,
}

/// import 関数定義
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportFunc {
    /// import 元モジュール名 (例: "wasi_snapshot_preview1")
    pub module: String,
    /// import 関数名 (例: "fd_write")
    pub name: String,
    /// パラメータ型
    pub params: Vec<IrType>,
    /// 戻り値型
    pub result: IrType,
}

/// グローバル変数定義（辞書インスタンス等）
#[derive(Debug, Clone)]
pub struct GlobalDef {
    pub name: String,
    pub ty: IrType,
    pub mutable: bool,
    /// 初期値命令列（const 式）
    pub init: Vec<Instruction>,
}

/// GC 型定義（WasmGC struct/array 用）
#[derive(Debug, Clone)]
pub struct GcTypeDef {
    pub name: String,
    pub kind: GcTypeKind,
}

/// GC 型の種別
#[derive(Debug, Clone)]
pub enum GcTypeKind {
    /// struct 型 (レコード, ADT バリアント)
    Struct(Vec<GcField>),
    /// array 型 (文字列等)
    Array(IrType),
    /// packed `i8` array 型 (UTF-8 byte string 等)
    PackedByteArray,
}

/// GC struct のフィールド
#[derive(Debug, Clone)]
pub struct GcField {
    pub name: String,
    pub ty: IrType,
    pub mutable: bool,
}

/// 関数定義
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<IrType>,
    pub result: IrType,
    pub locals: Vec<IrType>,
    pub body: Vec<Instruction>,
    pub is_export: bool,
}

impl Module {
    /// IR のテキスト表示
    pub fn dump(&self) -> String {
        let mut out = String::new();

        // GC 型定義を出力
        for gc_type in &self.gc_types {
            out.push_str(&format!("gc_type {} = ", gc_type.name));
            match &gc_type.kind {
                GcTypeKind::Struct(fields) => {
                    out.push_str("struct {");
                    for (i, field) in fields.iter().enumerate() {
                        if i > 0 {
                            out.push_str(", ");
                        }
                        let mut_str = if field.mutable { "mut " } else { "" };
                        out.push_str(&format!("{}{}: {}", mut_str, field.name, field.ty));
                    }
                    out.push_str("}\n");
                }
                GcTypeKind::Array(elem_ty) => {
                    out.push_str(&format!("array({elem_ty})\n"));
                }
                GcTypeKind::PackedByteArray => {
                    out.push_str("array(i8)\n");
                }
            }
        }

        if !self.gc_types.is_empty() {
            out.push('\n');
        }

        for func in &self.functions {
            out.push_str(&format!("fn {}(", func.name));
            for (i, p) in func.params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(&format!("{p}"));
            }
            out.push_str(&format!(") -> {}:\n", func.result));

            if !func.locals.is_empty() {
                out.push_str("  locals: ");
                for (i, l) in func.locals.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("{l}"));
                }
                out.push('\n');
            }

            for instr in &func.body {
                out.push_str(&format!("  {instr}\n"));
            }
            out.push('\n');
        }
        out
    }
}

/// 複数の IR モジュールを単一モジュールにリンク
///
/// 関数インデックスとGC型インデックスをリベースして結合する。
pub fn link_modules(modules: &[Module]) -> Module {
    use std::collections::HashMap;

    let mut linked_functions = Vec::new();
    let mut linked_gc_types = Vec::new();
    let mut linked_imports = Vec::new();

    // import 関数の重複除去
    // (module, name) -> 新 import index
    let mut import_dedup: HashMap<(String, String), u32> = HashMap::new();
    // (モジュールindex, 旧import_index) -> 新import_index
    let mut import_remap: HashMap<(usize, u32), u32> = HashMap::new();
    // linked import が採用した module-local signature の出所。
    let mut import_sources: Vec<(usize, u32)> = Vec::new();

    for (mod_idx, module) in modules.iter().enumerate() {
        for (old_idx, imp) in module.imports.iter().enumerate() {
            let key = (imp.module.clone(), imp.name.clone());
            if let Some(&existing_idx) = import_dedup.get(&key) {
                import_remap.insert((mod_idx, old_idx as u32), existing_idx);
            } else {
                let new_idx = linked_imports.len() as u32;
                import_dedup.insert(key, new_idx);
                import_remap.insert((mod_idx, old_idx as u32), new_idx);
                linked_imports.push(imp.clone());
                import_sources.push((mod_idx, old_idx as u32));
            }
        }
    }

    // GC 型インデックスのリベースマップ
    // (モジュールindex, 旧型index) -> 新型index
    let mut gc_type_remap: HashMap<(usize, u32), u32> = HashMap::new();

    // まず全 GC 型を集約
    for (mod_idx, module) in modules.iter().enumerate() {
        for (old_idx, gc_type) in module.gc_types.iter().enumerate() {
            let new_idx = linked_gc_types.len() as u32;
            gc_type_remap.insert((mod_idx, old_idx as u32), new_idx);
            linked_gc_types.push(gc_type.clone());
        }
    }

    // 関数インデックスのリベースマップ
    // (モジュールindex, 旧関数index) -> 新関数index
    let mut func_remap: HashMap<(usize, u32), u32> = HashMap::new();
    let mut func_idx = 0u32;

    // import 関数数分オフセット（各モジュールの import 数を考慮）
    let total_imports = linked_imports.len() as u32;

    for (mod_idx, module) in modules.iter().enumerate() {
        let module_import_count = module.imports.len() as u32;
        for (old_idx, _func) in module.functions.iter().enumerate() {
            // ユーザー関数は import 数分オフセット
            func_remap.insert(
                (mod_idx, old_idx as u32 + module_import_count),
                func_idx + total_imports,
            );
            func_idx += 1;
        }
    }

    // `CallRef` が参照する function type index のリベースマップ。
    // IR の type section は GC 型 → import 関数型 → user 関数型の順で構成する。
    let mut function_type_remap: HashMap<(usize, u32), u32> = HashMap::new();
    let linked_function_type_start = linked_gc_types.len() as u32 + total_imports;
    let mut linked_function_offset = 0u32;
    for (mod_idx, module) in modules.iter().enumerate() {
        let old_import_type_start = module.gc_types.len() as u32;
        for (old_import_idx, _) in module.imports.iter().enumerate() {
            if let Some(&new_import_idx) = import_remap.get(&(mod_idx, old_import_idx as u32)) {
                function_type_remap.insert(
                    (mod_idx, old_import_type_start + old_import_idx as u32),
                    linked_gc_types.len() as u32 + new_import_idx,
                );
            }
        }

        let old_function_type_start = old_import_type_start + module.imports.len() as u32;
        for old_function_idx in 0..module.functions.len() as u32 {
            function_type_remap.insert(
                (mod_idx, old_function_type_start + old_function_idx),
                linked_function_type_start + linked_function_offset + old_function_idx,
            );
        }
        linked_function_offset += module.functions.len() as u32;
    }

    // 関数シグネチャと GC 型定義にも module-local な型 index が残るため、
    // 命令列と同じ remap を適用する。WasmGC の env struct は field に
    // `Ref(gc_type)` と `TypedFuncRef(function_type)` の両方を持つため、
    // 命令だけを直しても linked module の型境界が壊れる。
    for (mod_idx, module) in modules.iter().enumerate() {
        for (old_idx, gc_type) in module.gc_types.iter().enumerate() {
            let Some(&new_idx) = gc_type_remap.get(&(mod_idx, old_idx as u32)) else {
                continue;
            };
            let mut remapped = gc_type.clone();
            remap_gc_type_definition(&mut remapped, mod_idx, &gc_type_remap, &function_type_remap);
            linked_gc_types[new_idx as usize] = remapped;
        }
    }

    // import の params/result も linked type index の境界に含める。重複 import は最初に
    // 採用した module-local signature を正本として remap する。
    for (linked_idx, (source_mod_idx, source_import_idx)) in import_sources.iter().enumerate() {
        let source_import = &modules[*source_mod_idx].imports[*source_import_idx as usize];
        let linked_import = &mut linked_imports[linked_idx];
        for ty in &mut linked_import.params {
            *ty = remap_ir_type(*ty, *source_mod_idx, &gc_type_remap, &function_type_remap);
        }
        linked_import.result = remap_ir_type(
            source_import.result,
            *source_mod_idx,
            &gc_type_remap,
            &function_type_remap,
        );
    }

    // 全関数を集約（命令のインデックスをリベース）
    for (mod_idx, module) in modules.iter().enumerate() {
        let module_import_count = module.imports.len() as u32;
        for func in &module.functions {
            let mut new_func = func.clone();

            for ty in &mut new_func.params {
                *ty = remap_ir_type(*ty, mod_idx, &gc_type_remap, &function_type_remap);
            }
            new_func.result = remap_ir_type(
                new_func.result,
                mod_idx,
                &gc_type_remap,
                &function_type_remap,
            );
            for ty in &mut new_func.locals {
                *ty = remap_ir_type(*ty, mod_idx, &gc_type_remap, &function_type_remap);
            }

            // 命令内のインデックスをリベース
            for instr in &mut new_func.body {
                remap_instruction_with_imports(
                    instr,
                    mod_idx,
                    module_import_count,
                    &func_remap,
                    &import_remap,
                    &gc_type_remap,
                    &function_type_remap,
                );
            }

            linked_functions.push(new_func);
        }
    }

    Module {
        functions: linked_functions,
        gc_types: linked_gc_types,
        imports: linked_imports,
        globals: Vec::new(),
        string_data: Vec::new(),
    }
}

/// module-local な GC/function type index を linked module の index へ変換する。
fn remap_ir_type(
    ty: IrType,
    mod_idx: usize,
    gc_type_remap: &std::collections::HashMap<(usize, u32), u32>,
    function_type_remap: &std::collections::HashMap<(usize, u32), u32>,
) -> IrType {
    match ty {
        IrType::Ref(index) => gc_type_remap
            .get(&(mod_idx, index))
            .copied()
            .map(IrType::Ref)
            .unwrap_or(IrType::Ref(index)),
        IrType::TypedFuncRef(index) => function_type_remap
            .get(&(mod_idx, index))
            .copied()
            .map(IrType::TypedFuncRef)
            .unwrap_or(IrType::TypedFuncRef(index)),
        other => other,
    }
}

/// GC struct/array の field type に linked module の型 index を適用する。
fn remap_gc_type_definition(
    gc_type: &mut GcTypeDef,
    mod_idx: usize,
    gc_type_remap: &std::collections::HashMap<(usize, u32), u32>,
    function_type_remap: &std::collections::HashMap<(usize, u32), u32>,
) {
    match &mut gc_type.kind {
        GcTypeKind::Struct(fields) => {
            for field in fields {
                field.ty = remap_ir_type(field.ty, mod_idx, gc_type_remap, function_type_remap);
            }
        }
        GcTypeKind::Array(element_type) => {
            *element_type =
                remap_ir_type(*element_type, mod_idx, gc_type_remap, function_type_remap);
        }
        GcTypeKind::PackedByteArray => {}
    }
}

/// 命令内のインデックスをリベース（import 対応版）
fn remap_instruction_with_imports(
    instr: &mut Instruction,
    mod_idx: usize,
    module_import_count: u32,
    func_remap: &std::collections::HashMap<(usize, u32), u32>,
    import_remap: &std::collections::HashMap<(usize, u32), u32>,
    gc_type_remap: &std::collections::HashMap<(usize, u32), u32>,
    function_type_remap: &std::collections::HashMap<(usize, u32), u32>,
) {
    match instr {
        Instruction::Call(idx) | Instruction::RefFunc(idx) => {
            if *idx < module_import_count {
                // import 関数の呼び出し
                if let Some(&new_idx) = import_remap.get(&(mod_idx, *idx)) {
                    *idx = new_idx;
                }
            } else {
                // ユーザー関数の呼び出し
                if let Some(&new_idx) = func_remap.get(&(mod_idx, *idx)) {
                    *idx = new_idx;
                }
            }
        }
        Instruction::StructNew(idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        Instruction::StructGet(type_idx, _) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::StructSet(type_idx, _) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::RefCast(idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        Instruction::RefNull(idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        Instruction::ArrayNewFixed(type_idx, _)
        | Instruction::ArrayNewDefault(type_idx)
        | Instruction::ArrayGet(type_idx)
        | Instruction::ArraySet(type_idx)
        | Instruction::ArrayLen(type_idx) => {
            if let Some(&new_idx) = gc_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::CallRef(type_idx) => {
            if let Some(&new_idx) = function_type_remap.get(&(mod_idx, *type_idx)) {
                *type_idx = new_idx;
            }
        }
        Instruction::CallIndirect(_) => {
            // CallIndirect の型インデックスはリマップ不要
        }
        Instruction::FuncIdx(idx) => {
            // FuncIdx は Call と同じインデックス空間
            if *idx < module_import_count {
                if let Some(&new_idx) = import_remap.get(&(mod_idx, *idx)) {
                    *idx = new_idx;
                }
            } else if let Some(&new_idx) = func_remap.get(&(mod_idx, *idx)) {
                *idx = new_idx;
            }
        }
        _ => {}
    }
}

/// マルチファイルコンパイルのパイプライン
///
/// エントリファイルから依存関係を解決し、トポロジカルソート順に
/// パース → 型チェックを行い、全モジュールの AST を結合してから
/// IR 変換することで、関数インデックスの一貫性を保つ。
#[derive(Debug, Clone, Default)]
struct ImportVisibilitySpec {
    only: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ModuleTypeSurface {
    results: Vec<(String, lsharp_types::types::TypeScheme)>,
    hidden: HashSet<String>,
    expr_types: HashMap<ExprTypeKey, lsharp_types::types::Type>,
}

impl ModuleTypeSurface {
    fn export_surface_eq(&self, other: &Self) -> bool {
        self.results == other.results && self.hidden == other.hidden
    }
}

fn type_surface_key(surface: &ModuleTypeSurface) -> u64 {
    let mut results = surface.results.clone();
    results.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hidden = surface.hidden.iter().cloned().collect::<Vec<_>>();
    hidden.sort();

    let mut hasher = DefaultHasher::new();
    results.hash(&mut hasher);
    hidden.hash(&mut hasher);
    hasher.finish()
}

fn dependency_surface_key(
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

fn push_defn_origins_infer_order(
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

fn collect_import_visibility(
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

fn collect_import_modules(program: &lsharp_syntax::ast::Program) -> Vec<String> {
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

fn parse_program_for_incremental(
    source: &str,
) -> Result<lsharp_syntax::ast::Program, lsharp_syntax::ParseAllError> {
    #[cfg(test)]
    {
        INCREMENTAL_PARSE_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_PARSE_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
    lsharp_syntax::parse(source)
}

fn cached_program_or_parse(
    mod_name: &str,
    source: &str,
    fingerprint: SourceFingerprint,
    cache: &CompilationCache,
) -> Result<Arc<lsharp_syntax::ast::Program>, lsharp_syntax::ParseAllError> {
    if let Some(entry) = cache.get(mod_name)
        && entry.fingerprint() == fingerprint
    {
        return Ok(entry.ast_arc());
    }
    parse_program_for_incremental(source).map(Arc::new)
}

fn note_incremental_type_infer() {
    #[cfg(test)]
    {
        INCREMENTAL_TYPE_INFER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_TYPE_INFER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_scc_infer() {
    #[cfg(test)]
    {
        INCREMENTAL_SCC_INFER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_SCC_INFER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_scc_merged_fast_path() {
    #[cfg(test)]
    {
        INCREMENTAL_SCC_MERGED_FAST_PATH_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_SCC_MERGED_FAST_PATH_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_lower() {
    #[cfg(test)]
    {
        INCREMENTAL_LOWER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LOWER_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_module_segment_lower_by(_count: usize) {
    #[cfg(test)]
    {
        INCREMENTAL_MODULE_SEGMENT_LOWER_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_MODULE_SEGMENT_LOWER_COUNT.with(|slot| {
                    slot.set(slot.get() + _count);
                });
            }
        });
    }
}

fn note_incremental_link_full() {
    #[cfg(test)]
    {
        INCREMENTAL_LINK_FULL_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LINK_FULL_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn note_incremental_link_cache_hit() {
    #[cfg(test)]
    {
        INCREMENTAL_LINK_CACHE_HIT_TRACKING_ENABLED.with(|enabled| {
            if enabled.get() {
                INCREMENTAL_LINK_CACHE_HIT_COUNT.with(|count| count.set(count.get() + 1));
            }
        });
    }
}

fn build_module_cache_entry(
    fingerprint: SourceFingerprint,
    deps_key: u64,
    program: &Arc<lsharp_syntax::ast::Program>,
    type_surface: ModuleTypeSurface,
) -> ModuleCacheEntry {
    ModuleCacheEntry::new(
        fingerprint,
        deps_key,
        Arc::clone(program),
        type_surface,
        Module {
            functions: Vec::new(),
            gc_types: Vec::new(),
            imports: Vec::new(),
            globals: Vec::new(),
            string_data: Vec::new(),
        },
        ModuleIrSegments::empty(),
        collect_import_modules(program.as_ref()),
    )
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum MultiFileLoweringMode {
    Merged,
    Modular,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SegmentRange {
    start: usize,
    len: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleLinkRanges {
    defns_functions: SegmentRange,
    accessors_functions: SegmentRange,
    trait_impls_functions: SegmentRange,
    constraints_functions: SegmentRange,
    ctors_functions: SegmentRange,
    defn_lifted_functions: SegmentRange,
    trait_impl_lifted_functions: SegmentRange,
    defns_gc_types: SegmentRange,
    defns_string_data: SegmentRange,
    trait_impls_string_data: SegmentRange,
}

fn module_has_content(module: &Module) -> bool {
    !module.functions.is_empty()
        || !module.gc_types.is_empty()
        || !module.imports.is_empty()
        || !module.globals.is_empty()
        || !module.string_data.is_empty()
}

fn build_segment_module(
    functions: Vec<Function>,
    gc_types: Vec<GcTypeDef>,
    string_data: Vec<(String, Vec<u8>)>,
) -> Module {
    Module {
        functions,
        gc_types,
        imports: Vec::new(),
        globals: Vec::new(),
        string_data,
    }
}

fn link_modules_preserving_indices(modules: &[Module]) -> Module {
    let mut linked = Module {
        functions: Vec::new(),
        gc_types: Vec::new(),
        imports: Vec::new(),
        globals: Vec::new(),
        string_data: Vec::new(),
    };

    for module in modules {
        linked.functions.extend(module.functions.clone());
        linked.gc_types.extend(module.gc_types.clone());
        linked.imports.extend(module.imports.clone());
        linked.globals.extend(module.globals.clone());
        linked.string_data.extend(module.string_data.clone());
    }

    linked
}

fn lower_multi_file_merged(
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
) -> Result<Module, lower::LowerError> {
    let merged_program = lsharp_syntax::ast::Program {
        decls: all_decls.to_vec(),
    };
    let mut lower_ctx = lower::Lower::new();
    lower_ctx.lower_program_with_expr_types(
        &merged_program,
        all_type_results,
        all_expr_type_results,
    )
}

fn prime_cached_string_data(lower_ctx: &mut lower::Lower, string_data: &[(String, Vec<u8>)]) {
    for (label, bytes) in string_data {
        lower_ctx.string_data.push((label.clone(), bytes.clone()));
        lower_ctx.string_offset += bytes.len() as u32;
    }
}

fn prime_cached_lifted(lower_ctx: &mut lower::Lower, module: &Module) {
    lower_ctx.lambda_counter += module.functions.len() as u32;
    lower_ctx.lifted_functions.extend(module.functions.clone());
}

fn link_module_ir_segments(segments: &[ModuleIrSegments]) -> Module {
    let mut modules = Vec::new();

    for segment in segments {
        if module_has_content(segment.defns()) {
            modules.push(segment.defns().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.accessors()) {
            modules.push(segment.accessors().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.trait_impls()) {
            modules.push(segment.trait_impls().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.constraints()) {
            modules.push(segment.constraints().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.ctors()) {
            modules.push(segment.ctors().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.defn_lifted()) {
            modules.push(segment.defn_lifted().clone());
        }
    }
    for segment in segments {
        if module_has_content(segment.trait_impl_lifted()) {
            modules.push(segment.trait_impl_lifted().clone());
        }
    }

    link_modules_preserving_indices(&modules)
}

fn next_segment_range(cursor: &mut usize, len: usize) -> SegmentRange {
    let range = SegmentRange {
        start: *cursor,
        len,
    };
    *cursor += len;
    range
}

fn compute_module_link_ranges(segments: &[ModuleIrSegments]) -> Vec<ModuleLinkRanges> {
    let mut ranges = vec![ModuleLinkRanges::default(); segments.len()];

    let mut function_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_functions =
            next_segment_range(&mut function_cursor, segment.defns().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].accessors_functions =
            next_segment_range(&mut function_cursor, segment.accessors().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impls_functions =
            next_segment_range(&mut function_cursor, segment.trait_impls().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].constraints_functions =
            next_segment_range(&mut function_cursor, segment.constraints().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].ctors_functions =
            next_segment_range(&mut function_cursor, segment.ctors().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defn_lifted_functions =
            next_segment_range(&mut function_cursor, segment.defn_lifted().functions.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impl_lifted_functions = next_segment_range(
            &mut function_cursor,
            segment.trait_impl_lifted().functions.len(),
        );
    }

    let mut gc_type_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_gc_types =
            next_segment_range(&mut gc_type_cursor, segment.defns().gc_types.len());
    }

    let mut string_cursor = 0;
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].defns_string_data =
            next_segment_range(&mut string_cursor, segment.defns().string_data.len());
    }
    for (idx, segment) in segments.iter().enumerate() {
        ranges[idx].trait_impls_string_data =
            next_segment_range(&mut string_cursor, segment.trait_impls().string_data.len());
    }

    ranges
}

fn segment_layout_matches(old: &ModuleIrSegments, new: &ModuleIrSegments) -> bool {
    old.defns().functions.len() == new.defns().functions.len()
        && old.accessors().functions.len() == new.accessors().functions.len()
        && old.trait_impls().functions.len() == new.trait_impls().functions.len()
        && old.constraints().functions.len() == new.constraints().functions.len()
        && old.ctors().functions.len() == new.ctors().functions.len()
        && old.defn_lifted().functions.len() == new.defn_lifted().functions.len()
        && old.trait_impl_lifted().functions.len() == new.trait_impl_lifted().functions.len()
        && old.defns().gc_types.len() == new.defns().gc_types.len()
        && old.defns().string_data.len() == new.defns().string_data.len()
        && old.trait_impls().string_data.len() == new.trait_impls().string_data.len()
}

fn can_patch_linked_module(
    cache: &CompilationCache,
    module_order: &[String],
    old_segments: &[ModuleIrSegments],
    new_segments: &[ModuleIrSegments],
) -> bool {
    cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
        && old_segments.len() == new_segments.len()
        && old_segments
            .iter()
            .zip(new_segments.iter())
            .all(|(old, new)| segment_layout_matches(old, new))
}

fn overwrite_range<T: Clone>(target: &mut [T], range: SegmentRange, replacement: &[T]) {
    debug_assert_eq!(range.len, replacement.len());
    target[range.start..range.start + range.len].clone_from_slice(replacement);
}

fn patch_linked_module(
    base: &Module,
    old_segments: &[ModuleIrSegments],
    new_segments: &[ModuleIrSegments],
) -> Module {
    let ranges = compute_module_link_ranges(old_segments);
    let mut patched = base.clone();

    for (range, segment) in ranges.iter().zip(new_segments.iter()) {
        overwrite_range(
            &mut patched.functions,
            range.defns_functions,
            &segment.defns().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.accessors_functions,
            &segment.accessors().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.trait_impls_functions,
            &segment.trait_impls().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.constraints_functions,
            &segment.constraints().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.ctors_functions,
            &segment.ctors().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.defn_lifted_functions,
            &segment.defn_lifted().functions,
        );
        overwrite_range(
            &mut patched.functions,
            range.trait_impl_lifted_functions,
            &segment.trait_impl_lifted().functions,
        );
        overwrite_range(
            &mut patched.gc_types,
            range.defns_gc_types,
            &segment.defns().gc_types,
        );
        overwrite_range(
            &mut patched.string_data,
            range.defns_string_data,
            &segment.defns().string_data,
        );
        overwrite_range(
            &mut patched.string_data,
            range.trait_impls_string_data,
            &segment.trait_impls().string_data,
        );
    }

    patched
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModulePrecomputedShape {
    defn_count: usize,
    accessor_count: usize,
    trait_impl_count: usize,
    constraint_count: usize,
    ctor_count: usize,
    gc_type_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleDefnStateShape {
    string_bytes: usize,
    lifted_count: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ModuleTraitImplStateShape {
    string_bytes: usize,
    lifted_count: usize,
}

fn module_precomputed_shape_from_program(
    program: &lsharp_syntax::ast::Program,
) -> ModulePrecomputedShape {
    use lsharp_syntax::ast::Decl;

    let mut shape = ModulePrecomputedShape::default();
    for decl in &program.decls {
        match decl {
            Decl::Defn { .. } => shape.defn_count += 1,
            Decl::RecordDef { fields, .. } => {
                shape.accessor_count += fields.len();
                shape.gc_type_count += 1;
            }
            Decl::ImplDef { methods, .. } => {
                shape.trait_impl_count += methods.len();
            }
            Decl::TypeConstrained { .. } => {
                shape.constraint_count += 2;
            }
            Decl::TypeDef { variants, .. } => {
                shape.ctor_count += variants.len();
            }
            Decl::ModuleDecl { .. } | Decl::ImportDecl { .. } => {}
            _ => {}
        }
    }
    shape
}

fn module_precomputed_shape_from_segments(segments: &ModuleIrSegments) -> ModulePrecomputedShape {
    ModulePrecomputedShape {
        defn_count: segments.defns().functions.len(),
        accessor_count: segments.accessors().functions.len(),
        trait_impl_count: segments.trait_impls().functions.len(),
        constraint_count: segments.constraints().functions.len(),
        ctor_count: segments.ctors().functions.len(),
        gc_type_count: segments.defns().gc_types.len(),
    }
}

fn module_defn_state_shape(module: &ModuleIrSegments) -> ModuleDefnStateShape {
    ModuleDefnStateShape {
        string_bytes: module
            .defns()
            .string_data
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum(),
        lifted_count: module.defn_lifted().functions.len(),
    }
}

fn module_trait_impl_state_shape(module: &ModuleIrSegments) -> ModuleTraitImplStateShape {
    ModuleTraitImplStateShape {
        string_bytes: module
            .trait_impls()
            .string_data
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum(),
        lifted_count: module.trait_impl_lifted().functions.len(),
    }
}

fn defn_state_depends_on_prefix(shape: ModuleDefnStateShape) -> bool {
    shape.string_bytes > 0 || shape.lifted_count > 0
}

struct ModularLoweringResult {
    segments: Vec<ModuleIrSegments>,
    fresh_defn_lower_count: usize,
}

fn lower_multi_file_modular_with_segments(
    module_programs: &[lsharp_syntax::ast::Program],
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
    reusable_segments: &[Option<ModuleIrSegments>],
    segment_reuse_candidates: &[bool],
) -> Result<ModularLoweringResult, lower::LowerError> {
    let merged_program = lsharp_syntax::ast::Program {
        decls: all_decls.to_vec(),
    };
    let mut lower_ctx = lower::Lower::new();
    lower_ctx.prepare_program_state(&merged_program, all_type_results);
    lower_ctx.expr_type_results = all_expr_type_results.clone();

    let mut segments = vec![ModuleIrSegments::empty(); module_programs.len()];
    let cached_precomputed_shapes: Vec<Option<ModulePrecomputedShape>> = reusable_segments
        .iter()
        .map(|segments| {
            segments
                .as_ref()
                .map(module_precomputed_shape_from_segments)
        })
        .collect();
    let cached_defn_shapes: Vec<Option<ModuleDefnStateShape>> = reusable_segments
        .iter()
        .map(|segments| segments.as_ref().map(module_defn_state_shape))
        .collect();
    let cached_trait_shapes: Vec<Option<ModuleTraitImplStateShape>> = reusable_segments
        .iter()
        .map(|segments| segments.as_ref().map(module_trait_impl_state_shape))
        .collect();
    let precomputed_shape_matches: Vec<bool> = module_programs
        .iter()
        .zip(cached_precomputed_shapes.iter())
        .map(|(program, cached)| {
            cached
                .map(|cached_shape| module_precomputed_shape_from_program(program) == cached_shape)
                .unwrap_or(false)
        })
        .collect();
    let mut defn_shape_matches = vec![false; module_programs.len()];
    let mut fresh_defn_lower_count = 0usize;
    let mut precomputed_prefix_stable = true;
    let mut defn_prefix_stable = true;

    for (idx, program) in module_programs.iter().enumerate() {
        let current_defn_needs_prefix_state =
            cached_defn_shapes[idx].is_some_and(defn_state_depends_on_prefix);
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && (!current_defn_needs_prefix_state || defn_prefix_stable)
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            prime_cached_string_data(&mut lower_ctx, &cached.defns().string_data);
            prime_cached_lifted(&mut lower_ctx, cached.defn_lifted());
            segments[idx].set_defns(cached.defns().clone());
            segments[idx].set_defn_lifted(cached.defn_lifted().clone());
            defn_shape_matches[idx] = true;
        } else {
            fresh_defn_lower_count += 1;
            let gc_types = lower_ctx.gc_types_for_program(program);
            let string_start = lower_ctx.string_data.len();
            let lifted_start = lower_ctx.lifted_functions.len();
            let functions = lower_ctx.lower_defn_functions(program)?;
            let string_data = lower_ctx.clone_string_data_from(string_start);
            let lifted = lower_ctx.lifted_functions[lifted_start..].to_vec();
            segments[idx].set_defns(build_segment_module(functions, gc_types, string_data));
            segments[idx].set_defn_lifted(build_segment_module(lifted, Vec::new(), Vec::new()));
            defn_shape_matches[idx] = cached_defn_shapes[idx].is_some_and(|cached_shape| {
                module_defn_state_shape(&segments[idx]) == cached_shape
            });
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        defn_prefix_stable &= defn_shape_matches[idx];
    }

    let defn_global_stable = defn_shape_matches.iter().all(|stable| *stable);

    precomputed_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            segments[idx].set_accessors(cached.accessors().clone());
        } else {
            segments[idx].set_accessors(build_segment_module(
                lower_ctx.lower_field_accessors(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
    }

    precomputed_prefix_stable = true;
    let mut trait_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && defn_global_stable
            && precomputed_prefix_stable
            && trait_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            prime_cached_string_data(&mut lower_ctx, &cached.trait_impls().string_data);
            prime_cached_lifted(&mut lower_ctx, cached.trait_impl_lifted());
            segments[idx].set_trait_impls(cached.trait_impls().clone());
            segments[idx].set_trait_impl_lifted(cached.trait_impl_lifted().clone());
        } else {
            let string_start = lower_ctx.string_data.len();
            let lifted_start = lower_ctx.lifted_functions.len();
            let functions = lower_ctx.lower_trait_impl_functions(program)?;
            let string_data = lower_ctx.clone_string_data_from(string_start);
            let lifted = lower_ctx.lifted_functions[lifted_start..].to_vec();
            segments[idx].set_trait_impls(build_segment_module(functions, Vec::new(), string_data));
            segments[idx].set_trait_impl_lifted(build_segment_module(
                lifted,
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        trait_prefix_stable &= cached_trait_shapes[idx].is_some_and(|cached_shape| {
            module_trait_impl_state_shape(&segments[idx]) == cached_shape
        });
    }

    precomputed_prefix_stable = true;
    let mut constraint_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && constraint_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            lower_ctx.late_func_idx += cached.constraints().functions.len() as u32;
            segments[idx].set_constraints(cached.constraints().clone());
        } else {
            segments[idx].set_constraints(build_segment_module(
                lower_ctx.lower_constraint_functions(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
        constraint_prefix_stable &= cached_precomputed_shapes[idx].is_some_and(|cached_shape| {
            module_precomputed_shape_from_segments(&segments[idx]).constraint_count
                == cached_shape.constraint_count
        });
    }

    precomputed_prefix_stable = true;
    for (idx, program) in module_programs.iter().enumerate() {
        if segment_reuse_candidates.get(idx).copied().unwrap_or(false)
            && precomputed_prefix_stable
            && let Some(cached) = reusable_segments
                .get(idx)
                .and_then(|segment| segment.clone())
        {
            segments[idx].set_ctors(cached.ctors().clone());
        } else {
            segments[idx].set_ctors(build_segment_module(
                lower_ctx.lower_adt_constructors(program),
                Vec::new(),
                Vec::new(),
            ));
        }

        precomputed_prefix_stable &= precomputed_shape_matches[idx];
    }

    Ok(ModularLoweringResult {
        segments,
        fresh_defn_lower_count,
    })
}

fn lower_multi_file_modular(
    module_programs: &[lsharp_syntax::ast::Program],
    all_decls: &[lsharp_syntax::ast::Decl],
    all_type_results: &[(String, lsharp_types::types::TypeScheme)],
    all_expr_type_results: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
) -> Result<Module, lower::LowerError> {
    let reusable_segments = vec![None; module_programs.len()];
    let lowering = lower_multi_file_modular_with_segments(
        module_programs,
        all_decls,
        all_type_results,
        all_expr_type_results,
        &reusable_segments,
        &vec![false; module_programs.len()],
    )?;
    Ok(link_module_ir_segments(&lowering.segments))
}

fn collect_private_surface_names(
    decls: &[lsharp_syntax::ast::Decl],
    module_prefix: Option<&str>,
    out: &mut HashSet<String>,
) {
    use lsharp_syntax::ast::Decl;

    for decl in decls {
        match decl {
            Decl::Private { inner, .. } => match inner.as_ref() {
                Decl::Defn { name, .. }
                | Decl::TypeDef { name, .. }
                | Decl::RecordDef { name, .. }
                | Decl::TypeAlias { name, .. }
                | Decl::TypeConstrained { name, .. } => {
                    let qualified = module_prefix
                        .map(|prefix| format!("{prefix}.{name}"))
                        .unwrap_or_else(|| name.clone());
                    out.insert(qualified);
                }
                Decl::ModuleDecl { name, body, .. } => {
                    let qualified = module_prefix
                        .map(|prefix| format!("{prefix}.{name}"))
                        .unwrap_or_else(|| name.clone());
                    collect_private_surface_names(body, Some(&qualified), out);
                }
                _ => {}
            },
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let qualified = module_prefix
                    .map(|prefix| format!("{prefix}.{name}"))
                    .unwrap_or_else(|| name.clone());
                collect_private_surface_names(body, Some(&qualified), out);
            }
            _ => {}
        }
    }
}

fn register_expr_scope_owner(
    owners: &mut HashMap<String, Option<String>>,
    scope: String,
    module_name: &str,
) {
    if let Some(existing) = owners.get_mut(&scope) {
        if existing.as_deref() != Some(module_name) {
            *existing = None;
        }
    } else {
        owners.insert(scope, Some(module_name.to_string()));
    }
}

fn collect_expr_scope_owners(
    decls: &[lsharp_syntax::ast::Decl],
    module_prefix: Option<&str>,
    module_name: &str,
    owners: &mut HashMap<String, Option<String>>,
) {
    use lsharp_syntax::ast::Decl;

    for decl in decls {
        let actual_decl = match decl {
            Decl::Private { inner, .. } => inner.as_ref(),
            other => other,
        };
        match actual_decl {
            Decl::Defn { name, .. } => {
                let scope = module_prefix
                    .map(|prefix| format!("{prefix}.{name}"))
                    .unwrap_or_else(|| name.clone());
                register_expr_scope_owner(owners, scope, module_name);
            }
            Decl::ModuleDecl { name, body, .. } if !body.is_empty() => {
                let prefix = module_prefix
                    .map(|outer| format!("{outer}.{name}"))
                    .unwrap_or_else(|| name.clone());
                collect_expr_scope_owners(body, Some(&prefix), module_name, owners);
            }
            Decl::ImplDef {
                trait_name,
                type_name,
                methods,
                ..
            } => {
                for method in methods {
                    let method = match method {
                        Decl::Private { inner, .. } => inner.as_ref(),
                        other => other,
                    };
                    if let Decl::Defn { name, .. } = method {
                        register_expr_scope_owner(
                            owners,
                            format!("{}::{}{}{}", trait_name, name, '$', type_name),
                            module_name,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

fn try_build_unrestricted_merged_scc_surfaces(
    group: &[String],
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
    direct_imports: &HashMap<String, HashMap<String, ImportVisibilitySpec>>,
    inferred_private_names: &[String],
    merged_expr_types: &HashMap<ExprTypeKey, lsharp_types::types::Type>,
    results_by_module: &mut HashMap<String, Vec<(String, lsharp_types::types::TypeScheme)>>,
) -> Option<HashMap<String, ModuleTypeSurface>> {
    if !inferred_private_names.is_empty()
        || group.iter().any(|module_name| {
            direct_imports
                .get(module_name)
                .into_iter()
                .flat_map(HashMap::values)
                .any(|import| import.only.is_some())
        })
    {
        return None;
    }

    let mut owners = HashMap::new();
    for module_name in group {
        let program = parsed_modules.get(module_name)?;
        collect_expr_scope_owners(&program.decls, None, module_name, &mut owners);
        results_by_module.get(module_name)?;
    }

    let mut expr_types_by_module: HashMap<String, HashMap<ExprTypeKey, lsharp_types::types::Type>> =
        HashMap::new();
    for (key, ty) in merged_expr_types {
        let Some(Some(module_name)) = owners.get(&key.scope) else {
            return None;
        };
        expr_types_by_module
            .entry(module_name.clone())
            .or_default()
            .insert(key.clone(), ty.clone());
    }

    let mut surfaces = HashMap::new();
    for module_name in group {
        surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results: results_by_module.remove(module_name)?,
                hidden: HashSet::new(),
                expr_types: expr_types_by_module.remove(module_name).unwrap_or_default(),
            },
        );
    }
    Some(surfaces)
}

/// SCC の merged inference 用に宣言を連結する。
///
/// 同じ import 宣言が SCC 内の複数 module に現れても、型環境への注入は一度で足りる。
/// ただし `:only`、alias、`open` が異なる import は意味が異なるため、完全一致する宣言
/// だけを重複除去する。宣言の順序と defn の所属 module は維持する。
type SccImportKey = (String, Option<String>, Option<Vec<String>>, bool);

fn merge_scc_declarations(
    group: &[String],
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
) -> Result<(Vec<lsharp_syntax::ast::Decl>, Vec<String>), String> {
    use lsharp_syntax::ast::Decl;

    let mut merged_decls = Vec::new();
    let mut defn_origins = Vec::new();
    let mut seen_imports: HashSet<SccImportKey> = HashSet::new();

    for module_name in group {
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        for decl in &program.decls {
            match decl {
                Decl::ModuleDecl { body, .. } if body.is_empty() => {}
                Decl::ImportDecl {
                    module,
                    alias,
                    only,
                    open,
                    ..
                } => {
                    let key = (module.clone(), alias.clone(), only.clone(), *open);
                    if seen_imports.insert(key) {
                        merged_decls.push(decl.clone());
                    }
                }
                _ => {
                    push_defn_origins_infer_order(
                        std::slice::from_ref(decl),
                        module_name,
                        None,
                        &mut defn_origins,
                    );
                    merged_decls.push(decl.clone());
                }
            }
        }
    }

    Ok((merged_decls, defn_origins))
}

fn infer_scc_type_surfaces(
    group: &[String],
    graph: &module_graph::ModuleGraph,
    parsed_modules: &HashMap<String, lsharp_syntax::ast::Program>,
    module_paths: &HashMap<String, std::path::PathBuf>,
    direct_imports: &HashMap<String, HashMap<String, ImportVisibilitySpec>>,
    known_surfaces: &HashMap<String, ModuleTypeSurface>,
) -> Result<HashMap<String, ModuleTypeSurface>, String> {
    use lsharp_syntax::ast::Program;

    let group_set: HashSet<&str> = group.iter().map(String::as_str).collect();
    if group.len() == 1 {
        let module_name = &group[0];
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        let mut infer = lsharp_types::infer::Infer::new();
        for dependency in graph.dependency_closure(module_name) {
            if group_set.contains(dependency.as_str()) {
                continue;
            }
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = known_surfaces.get(&dependency)
            {
                infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        note_incremental_type_infer();
        let results = infer.infer_program(program).map_err(|error| {
            let path = module_paths
                .get(module_name)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| module_name.clone());
            format!("{path}: [{}] {error}", error.code())
        })?;
        return Ok(HashMap::from([(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            },
        )]));
    }

    let (merged_decls, defn_origins) = merge_scc_declarations(group, parsed_modules)?;

    let mut infer = lsharp_types::infer::Infer::new();
    for module_name in group {
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        for dependency in graph.dependency_closure(module_name) {
            if group_set.contains(dependency.as_str()) {
                continue;
            }
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = known_surfaces.get(&dependency)
            {
                infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
    }

    let merged = Program {
        decls: merged_decls,
    };
    let type_results = infer.infer_program(&merged).map_err(|error| {
        let path = group
            .first()
            .and_then(|module| module_paths.get(module))
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| group.join(", "));
        format!("{path}: [{}] {error}", error.code())
    })?;
    if type_results.len() != defn_origins.len() {
        return Err(format!(
            "SCC の型結果数が宣言数と一致しません: modules={}, results={}, origins={}",
            group.join(", "),
            type_results.len(),
            defn_origins.len()
        ));
    }

    let mut results_by_module: HashMap<String, Vec<(String, lsharp_types::types::TypeScheme)>> =
        HashMap::new();
    for ((name, scheme), origin) in type_results.into_iter().zip(defn_origins) {
        results_by_module
            .entry(origin)
            .or_default()
            .push((name, scheme));
    }

    let inferred_private_names = infer.module_env.privates.clone();
    let merged_expr_types = infer.expr_type_results_snapshot();
    if let Some(surfaces) = try_build_unrestricted_merged_scc_surfaces(
        group,
        parsed_modules,
        direct_imports,
        &inferred_private_names,
        &merged_expr_types,
        &mut results_by_module,
    ) {
        note_incremental_scc_merged_fast_path();
        return Ok(surfaces);
    }

    let mut provisional_surfaces = HashMap::new();
    for module_name in group {
        let results = results_by_module.remove(module_name).unwrap_or_default();
        let mut private_names = HashSet::new();
        if let Some(program) = parsed_modules.get(module_name) {
            collect_private_surface_names(&program.decls, None, &mut private_names);
        }
        let hidden = inferred_private_names
            .iter()
            .filter(|name| private_names.contains(*name))
            .cloned()
            .collect();
        provisional_surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden,
                expr_types: HashMap::new(),
            },
        );
    }

    // merged prepass で相互再帰の型を確定した後、各 module を元の import visibility
    // で再検証する。これにより SCC 内でも `:only` / private の境界を失わない。
    let mut surfaces = HashMap::new();
    for module_name in group {
        let mut module_infer = lsharp_types::infer::Infer::new();
        let imports = direct_imports.get(module_name).cloned().unwrap_or_default();
        for dependency in graph.dependency_closure(module_name) {
            if let Some(import_spec) = imports.get(&dependency)
                && let Some(surface) = provisional_surfaces
                    .get(&dependency)
                    .or_else(|| known_surfaces.get(&dependency))
            {
                module_infer.inject_external_types_for_import(
                    &dependency,
                    import_spec.only.as_deref(),
                    &surface.hidden,
                    &surface.results,
                );
            }
        }
        let program = parsed_modules
            .get(module_name)
            .ok_or_else(|| format!("SCC 内のモジュールが parse 結果にありません: {module_name}"))?;
        let results = module_infer.infer_program(program).map_err(|error| {
            let path = module_paths
                .get(module_name)
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| module_name.clone());
            format!("{path}: [{}] {error}", error.code())
        })?;
        surfaces.insert(
            module_name.clone(),
            ModuleTypeSurface {
                results,
                hidden: module_infer.module_env.privates.iter().cloned().collect(),
                expr_types: module_infer.expr_type_results_snapshot(),
            },
        );
    }

    Ok(surfaces)
}

fn compile_multi_file_with_mode(
    entry_file: &std::path::Path,
    lowering_mode: MultiFileLoweringMode,
) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    // 1. モジュールグラフの構築とファイル探索
    let (graph, sorted_files) = ModuleGraph::build_from_entry_with_scc(entry_file)
        .map_err(|e| format!("[{}] モジュールグラフ構築エラー: {e}", e.code()))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    // 単一ファイルの場合は通常のパイプライン
    if sorted_files.len() == 1 {
        let (_, mod_path) = &sorted_files[0];
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let program = lsharp_syntax::parse(&source)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let mut infer = lsharp_types::infer::Infer::new();
        let type_results = infer
            .infer_program(&program)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let expr_type_results = infer.expr_type_results_snapshot();
        let mut lower_ctx = lower::Lower::new();
        return lower_ctx
            .lower_program_with_expr_types(&program, &type_results, &expr_type_results)
            .map_err(|e| format!("{}: {e}", mod_path.display()));
    }

    // 2. 全モジュールを依存順にパースし、SCC ごとに型チェックする。
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
    let mut all_expr_type_results: HashMap<ExprTypeKey, lsharp_types::types::Type> = HashMap::new();
    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut module_programs: Vec<lsharp_syntax::ast::Program> = Vec::new();
    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();

    for (mod_name, mod_path) in &sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;

        let program = lsharp_syntax::parse(&source)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        direct_imports.insert(mod_name.clone(), collect_import_visibility(&program));
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program);
    }

    for group in graph.scc_groups() {
        let surfaces = infer_scc_type_surfaces(
            &group,
            &graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    for (mod_name, _) in &sorted_files {
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        all_type_results.extend(surface.results.clone());
        all_expr_type_results.extend(surface.expr_types.clone());

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        // 宣言を収集（module 宣言と import 宣言は除外）
        let mut module_decls = Vec::new();
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => {
                    all_decls.push(decl.clone());
                    module_decls.push(decl.clone());
                }
            }
        }
        module_programs.push(lsharp_syntax::ast::Program {
            decls: module_decls,
        });
    }

    let lowered = match lowering_mode {
        MultiFileLoweringMode::Merged => {
            lower_multi_file_merged(&all_decls, &all_type_results, &all_expr_type_results)
        }
        MultiFileLoweringMode::Modular => lower_multi_file_modular(
            &module_programs,
            &all_decls,
            &all_type_results,
            &all_expr_type_results,
        ),
    };

    lowered.map_err(|e| format!("IR 変換エラー: {e}"))
}

pub fn compile_multi_file(entry_file: &std::path::Path) -> Result<Module, String> {
    compile_multi_file_with_mode(entry_file, MultiFileLoweringMode::Modular)
}

/// CLI compile で再利用できる解析/IR cache 付きの multi-file compile 入口。
///
/// 既存の `compile_multi_file_incremental` は互換 API として残し、公開 surface には
/// cache の意図が名前に現れるこちらを推奨する。
pub fn compile_multi_file_with_cache(
    entry_file: &std::path::Path,
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    cache.prepare_for_entry(entry_file);
    compile_multi_file_incremental(entry_file, cache)
}

/// 循環を含む multi-file compile の incremental cache 更新を行う。
///
/// SCC は module 単位のトポロジカル推論では扱えないため、SCC ごとに一括推論してから
/// modular lowering を行う。型推論は SCC 単位で行うが、lowering は clean module の
/// segment を再利用し、dirty module の segment だけを生成し直す。
fn compile_multi_file_incremental_scc(
    graph: &module_graph::ModuleGraph,
    sorted_files: &[(String, std::path::PathBuf)],
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    let module_order = sorted_files
        .iter()
        .map(|(mod_name, _)| mod_name.clone())
        .collect::<Vec<_>>();
    let mut current_fingerprints = HashMap::new();
    let mut all_clean = true;
    for (mod_name, mod_path) in sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        all_clean &= cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        current_fingerprints.insert(mod_name.clone(), fingerprint);
    }
    if all_clean
        && let Some(linked) = cache
            .linked_module()
            .filter(|linked| linked.module_order() == module_order)
    {
        return Ok(linked.final_module().clone());
    }

    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();
    let mut fingerprints = HashMap::new();

    for (mod_name, mod_path) in sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = current_fingerprints
            .get(mod_name)
            .copied()
            .unwrap_or_else(|| SourceFingerprint::from_source(&source));
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        direct_imports.insert(
            mod_name.clone(),
            collect_import_visibility(program.as_ref()),
        );
        fingerprints.insert(mod_name.clone(), fingerprint);
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program.as_ref().clone());
    }

    let mut per_module_type_results = HashMap::new();
    for group in graph.scc_groups() {
        let group_cache_hit = group.iter().all(|module_name| {
            let Some(fingerprint) = current_fingerprints.get(module_name) else {
                return false;
            };
            let Some(imports) = direct_imports.get(module_name) else {
                return false;
            };
            let deps_key = dependency_surface_key(imports, &per_module_type_results, cache);
            cache.get(module_name).is_some_and(|entry| {
                entry.fingerprint() == *fingerprint && entry.deps_key() == deps_key
            })
        });
        if group_cache_hit {
            for module_name in &group {
                let surface = cache
                    .get(module_name)
                    .map(|entry| entry.type_surface_clone())
                    .ok_or_else(|| format!("型 surface cache がありません: {module_name}"))?;
                per_module_type_results.insert(module_name.clone(), surface);
            }
            continue;
        }

        note_incremental_scc_infer();
        let surfaces = infer_scc_type_surfaces(
            &group,
            graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    let mut all_decls = Vec::new();
    let mut all_type_results = Vec::new();
    let mut all_expr_type_results = HashMap::new();
    let mut module_programs = Vec::new();
    let mut cache_entries = Vec::new();
    let mut surface_changed_modules = HashSet::new();
    let mut segment_reuse_candidates = Vec::new();
    for (mod_name, _) in sorted_files {
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        all_type_results.extend(surface.results.clone());
        all_expr_type_results.extend(surface.expr_types.clone());

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let mut module_decls = Vec::new();
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => {
                    all_decls.push(decl.clone());
                    module_decls.push(decl.clone());
                }
            }
        }
        module_programs.push(lsharp_syntax::ast::Program {
            decls: module_decls,
        });

        let direct_imports = direct_imports
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの import 情報がありません: {mod_name}"))?;
        let fingerprint = fingerprints
            .get(mod_name)
            .copied()
            .ok_or_else(|| format!("モジュールの fingerprint がありません: {mod_name}"))?;
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let deps_key = dependency_surface_key(direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);
        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));
        let segment_reuse_candidate = clean_hit && deps_hit && !direct_dep_surface_changed;
        let surface_changed = cache
            .get(mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let program = std::sync::Arc::new(program.clone());
        let entry = build_module_cache_entry(fingerprint, deps_key, &program, surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        segment_reuse_candidates.push(segment_reuse_candidate);
    }

    let mut reusable_segments = vec![None; module_programs.len()];
    for (idx, (mod_name, _)) in cache_entries.iter().enumerate() {
        if let Some(cached_entry) = cache.get(mod_name)
            && !cached_entry.ir_segments().is_empty()
        {
            reusable_segments[idx] = Some(cached_entry.ir_segments().clone());
        }
    }

    note_incremental_lower();
    let lowering = lower_multi_file_modular_with_segments(
        &module_programs,
        &all_decls,
        &all_type_results,
        &all_expr_type_results,
        &reusable_segments,
        &segment_reuse_candidates,
    )
    .map_err(|e| format!("IR 変換エラー: {e}"))?;
    note_incremental_module_segment_lower_by(lowering.fresh_defn_lower_count);
    let new_segments = lowering.segments;
    let old_segments: Option<Vec<ModuleIrSegments>> = if cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
    {
        cache_entries
            .iter()
            .map(|(mod_name, _)| cache.get(mod_name).map(|entry| entry.ir_segments().clone()))
            .collect()
    } else {
        None
    };
    let final_module =
        if let (Some(old_segments), Some(linked)) = (old_segments, cache.linked_module()) {
            if can_patch_linked_module(cache, &module_order, &old_segments, &new_segments) {
                note_incremental_link_cache_hit();
                patch_linked_module(linked.final_module(), &old_segments, &new_segments)
            } else {
                note_incremental_link_full();
                link_module_ir_segments(&new_segments)
            }
        } else {
            note_incremental_link_full();
            link_module_ir_segments(&new_segments)
        };

    for ((mod_name, mut entry), segments) in cache_entries.into_iter().zip(new_segments) {
        entry.set_ir(final_module.clone());
        entry.set_ir_segments(segments);
        cache.insert_module(mod_name.clone(), entry);
    }
    cache.set_linked_module(module_order, final_module.clone());

    Ok(final_module)
}

/// source override を含む循環 module の incremental analysis を行う。
///
/// LSP の未保存 source でも compile と同じ SCC 推論境界を使い、解析結果だけを cache に保存する。
/// lowering はこの入口の責務ではないため、IR は空のまま保持する。
fn analyze_multi_file_incremental_scc_with_overrides(
    graph: &module_graph::ModuleGraph,
    sorted_files: &[(String, std::path::PathBuf)],
    source_overrides: &HashMap<std::path::PathBuf, String>,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    let mut parsed_modules = HashMap::new();
    let mut module_paths = HashMap::new();
    let mut direct_imports = HashMap::new();
    let mut fingerprints = HashMap::new();

    for (mod_name, mod_path) in sorted_files {
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        direct_imports.insert(
            mod_name.clone(),
            collect_import_visibility(program.as_ref()),
        );
        fingerprints.insert(mod_name.clone(), fingerprint);
        module_paths.insert(mod_name.clone(), mod_path.clone());
        parsed_modules.insert(mod_name.clone(), program.as_ref().clone());
    }

    let mut per_module_type_results = HashMap::new();
    for group in graph.scc_groups() {
        let group_cache_hit = group.iter().all(|module_name| {
            let Some(fingerprint) = fingerprints.get(module_name) else {
                return false;
            };
            let Some(imports) = direct_imports.get(module_name) else {
                return false;
            };
            let deps_key = dependency_surface_key(imports, &per_module_type_results, cache);
            cache.get(module_name).is_some_and(|entry| {
                entry.fingerprint() == *fingerprint && entry.deps_key() == deps_key
            })
        });
        if group_cache_hit {
            for module_name in &group {
                let surface = cache
                    .get(module_name)
                    .map(|entry| entry.type_surface_clone())
                    .ok_or_else(|| format!("型 surface cache がありません: {module_name}"))?;
                per_module_type_results.insert(module_name.clone(), surface);
            }
            continue;
        }

        note_incremental_scc_infer();
        let surfaces = infer_scc_type_surfaces(
            &group,
            graph,
            &parsed_modules,
            &module_paths,
            &direct_imports,
            &per_module_type_results,
        )?;
        per_module_type_results.extend(surfaces);
    }

    for (mod_name, _) in sorted_files {
        let program = parsed_modules
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの parse 結果がありません: {mod_name}"))?;
        let surface = per_module_type_results
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの SCC 型結果がありません: {mod_name}"))?;
        let direct_imports = direct_imports
            .get(mod_name)
            .ok_or_else(|| format!("モジュールの import 情報がありません: {mod_name}"))?;
        let fingerprint = fingerprints
            .get(mod_name)
            .copied()
            .ok_or_else(|| format!("モジュールの fingerprint がありません: {mod_name}"))?;
        let program = std::sync::Arc::new(program.clone());
        let deps_key = dependency_surface_key(direct_imports, &per_module_type_results, cache);
        let entry = build_module_cache_entry(fingerprint, deps_key, &program, surface.clone());
        cache.insert_module(mod_name.clone(), entry);
    }

    Ok(())
}

fn read_source_with_overrides(
    path: &std::path::Path,
    source_overrides: &HashMap<std::path::PathBuf, String>,
) -> Result<String, String> {
    if let Some(source) = source_overrides.get(path) {
        return Ok(source.clone());
    }

    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

pub fn analyze_single_file_incremental(
    module_name: &str,
    source: &str,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    let fingerprint = SourceFingerprint::from_source(source);
    let clean_hit = cache
        .get(module_name)
        .is_some_and(|entry| entry.fingerprint() == fingerprint);
    if clean_hit {
        return Ok(());
    }

    let program = cached_program_or_parse(module_name, source, fingerprint, cache)
        .map_err(|e| format!("[{}] {e}", e.code()))?;
    let mut infer = lsharp_types::infer::Infer::new();
    note_incremental_type_infer();
    let type_results = infer
        .infer_program(program.as_ref())
        .map_err(|e| format!("[{}] {e}", e.code()))?;
    let type_surface = ModuleTypeSurface {
        results: type_results,
        hidden: infer.module_env.privates.iter().cloned().collect(),
        expr_types: infer.expr_type_results_snapshot(),
    };
    let entry = build_module_cache_entry(fingerprint, 0, &program, type_surface);
    cache.insert_module(module_name.to_string(), entry);
    Ok(())
}

pub fn analyze_multi_file_incremental_with_overrides(
    entry_file: &std::path::Path,
    source_overrides: &HashMap<std::path::PathBuf, String>,
    cache: &mut CompilationCache,
) -> Result<(), String> {
    use module_graph::ModuleGraph;

    cache.prepare_for_entry(entry_file);
    let (graph, sorted_files) =
        ModuleGraph::build_from_entry_with_overrides_scc(entry_file, source_overrides)
            .map_err(|e| format!("[{}] モジュールグラフ構築エラー: {e}", e.code()))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    if sorted_files.len() == 1 {
        let (mod_name, mod_path) = &sorted_files[0];
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        return analyze_single_file_incremental(mod_name, &source, cache)
            .map_err(|e| format!("{}: {e}", mod_path.display()));
    }

    if graph.scc_groups().iter().any(|group| group.len() > 1) {
        return analyze_multi_file_incremental_scc_with_overrides(
            &graph,
            &sorted_files,
            source_overrides,
            cache,
        );
    }

    let mut module_inputs = Vec::new();
    let mut changed_modules = Vec::new();
    for (mod_name, mod_path) in &sorted_files {
        let source = read_source_with_overrides(mod_path, source_overrides)?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if !clean_hit {
            changed_modules.push(mod_name.clone());
        }
        module_inputs.push((mod_name.clone(), mod_path.clone(), source, fingerprint));
    }

    if changed_modules.is_empty() {
        return Ok(());
    }

    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut cache_entries: Vec<(String, ModuleCacheEntry)> = Vec::new();
    let mut surface_changed_modules: HashSet<String> = HashSet::new();

    for (mod_name, mod_path, source, fingerprint) in module_inputs {
        let clean_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let program = cached_program_or_parse(&mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let direct_imports = collect_import_visibility(program.as_ref());
        let deps_key = dependency_surface_key(&direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);

        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));

        let type_surface = if clean_hit && deps_hit && !direct_dep_surface_changed {
            cache
                .get(&mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            for dep_name in graph.dependency_closure(&mod_name) {
                if let Some(import_spec) = direct_imports.get(&dep_name)
                    && let Some(dep_surface) = per_module_type_results.get(&dep_name)
                {
                    infer.inject_external_types_for_import(
                        &dep_name,
                        import_spec.only.as_deref(),
                        &dep_surface.hidden,
                        &dep_surface.results,
                    );
                }
            }
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let surface_changed = cache
            .get(&mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(&type_surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        let entry = build_module_cache_entry(fingerprint, deps_key, &program, type_surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        per_module_type_results.insert(mod_name, type_surface);
    }

    for (mod_name, entry) in cache_entries {
        cache.insert_module(mod_name, entry);
    }

    Ok(())
}

pub fn compile_multi_file_incremental(
    entry_file: &std::path::Path,
    cache: &mut CompilationCache,
) -> Result<Module, String> {
    use module_graph::ModuleGraph;

    let (graph, sorted_files) = ModuleGraph::build_from_entry_with_scc(entry_file)
        .map_err(|e| format!("[{}] モジュールグラフ構築エラー: {e}", e.code()))?;

    if sorted_files.is_empty() {
        return Err("コンパイル対象のファイルがありません".to_string());
    }

    if sorted_files.len() == 1 {
        let (mod_name, mod_path) = &sorted_files[0];
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if clean_hit {
            return Ok(cache
                .get(mod_name)
                .expect("clean hit should have cache entry")
                .ir()
                .clone());
        }
        let program = cached_program_or_parse(mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let type_surface = if clean_hit {
            cache
                .get(mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let mut lower_ctx = lower::Lower::new();
        note_incremental_lower();
        let module = lower_ctx
            .lower_program_with_expr_types(
                program.as_ref(),
                &type_surface.results,
                &type_surface.expr_types,
            )
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let mut entry = build_module_cache_entry(fingerprint, 0, &program, type_surface);
        entry.set_ir(module.clone());
        cache.insert_module(mod_name.clone(), entry);
        return Ok(module);
    }

    if graph.scc_groups().iter().any(|group| group.len() > 1) {
        return compile_multi_file_incremental_scc(&graph, &sorted_files, cache);
    }

    let mut module_inputs = Vec::new();
    let mut changed_modules = Vec::new();
    for (mod_name, mod_path) in &sorted_files {
        let source = std::fs::read_to_string(mod_path)
            .map_err(|e| format!("{}: {e}", mod_path.display()))?;
        let fingerprint = SourceFingerprint::from_source(&source);
        let clean_hit = cache
            .get(mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        if !clean_hit {
            changed_modules.push(mod_name.clone());
        }
        module_inputs.push((mod_name.clone(), mod_path.clone(), source, fingerprint));
    }
    if changed_modules.is_empty() {
        let first_clean_entry = module_inputs
            .first()
            .and_then(|(mod_name, _, _, _)| cache.get(mod_name).map(|entry| entry.ir().clone()))
            .expect("all clean hits should have cache entries");
        return Ok(first_clean_entry);
    }
    let mut all_decls: Vec<lsharp_syntax::ast::Decl> = Vec::new();
    let mut all_type_results: Vec<(String, lsharp_types::types::TypeScheme)> = Vec::new();
    let mut all_expr_type_results: HashMap<ExprTypeKey, lsharp_types::types::Type> = HashMap::new();
    let mut per_module_type_results: HashMap<String, ModuleTypeSurface> = HashMap::new();
    let mut cache_entries: Vec<(String, ModuleCacheEntry)> = Vec::new();
    let mut surface_changed_modules: HashSet<String> = HashSet::new();
    let mut module_programs: Vec<lsharp_syntax::ast::Program> = Vec::new();
    let mut segment_reuse_candidates: Vec<bool> = Vec::new();

    for (mod_name, mod_path, source, fingerprint) in module_inputs {
        let clean_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.fingerprint() == fingerprint);
        let program = cached_program_or_parse(&mod_name, &source, fingerprint, cache)
            .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
        let direct_imports = collect_import_visibility(program.as_ref());
        let deps_key = dependency_surface_key(&direct_imports, &per_module_type_results, cache);
        let deps_hit = cache
            .get(&mod_name)
            .is_some_and(|entry| entry.deps_key() == deps_key);

        let direct_dep_surface_changed = direct_imports
            .keys()
            .any(|dep_name| surface_changed_modules.contains(dep_name));
        let segment_reuse_candidate = clean_hit && deps_hit && !direct_dep_surface_changed;

        let type_surface = if clean_hit && deps_hit && !direct_dep_surface_changed {
            cache
                .get(&mod_name)
                .expect("clean hit should have cache entry")
                .type_surface_clone()
        } else {
            let mut infer = lsharp_types::infer::Infer::new();
            for dep_name in graph.dependency_closure(&mod_name) {
                if let Some(import_spec) = direct_imports.get(&dep_name)
                    && let Some(dep_surface) = per_module_type_results.get(&dep_name)
                {
                    infer.inject_external_types_for_import(
                        &dep_name,
                        import_spec.only.as_deref(),
                        &dep_surface.hidden,
                        &dep_surface.results,
                    );
                }
            }
            note_incremental_type_infer();
            let type_results = infer
                .infer_program(program.as_ref())
                .map_err(|e| format!("{}: [{}] {e}", mod_path.display(), e.code()))?;
            ModuleTypeSurface {
                results: type_results,
                hidden: infer.module_env.privates.iter().cloned().collect(),
                expr_types: infer.expr_type_results_snapshot(),
            }
        };
        let surface_changed = cache
            .get(&mod_name)
            .map(|entry| !entry.type_surface_clone().export_surface_eq(&type_surface))
            .unwrap_or(true);
        if surface_changed {
            surface_changed_modules.insert(mod_name.clone());
        }

        all_type_results.extend(type_surface.results.clone());
        all_expr_type_results.extend(type_surface.expr_types.clone());
        let mut module_decls = Vec::new();
        for decl in &program.decls {
            match decl {
                lsharp_syntax::ast::Decl::ModuleDecl { .. }
                | lsharp_syntax::ast::Decl::ImportDecl { .. } => {}
                _ => {
                    all_decls.push(decl.clone());
                    module_decls.push(decl.clone());
                }
            }
        }
        module_programs.push(lsharp_syntax::ast::Program {
            decls: module_decls,
        });

        let entry = build_module_cache_entry(fingerprint, deps_key, &program, type_surface.clone());
        cache_entries.push((mod_name.clone(), entry));
        per_module_type_results.insert(mod_name.clone(), type_surface);
        segment_reuse_candidates.push(segment_reuse_candidate);
    }

    let mut reusable_segments = vec![None; module_programs.len()];
    for (idx, (mod_name, _)) in cache_entries.iter().enumerate() {
        if let Some(cached_entry) = cache.get(mod_name)
            && !cached_entry.ir_segments().is_empty()
        {
            reusable_segments[idx] = Some(cached_entry.ir_segments().clone());
        }
    }

    note_incremental_lower();
    let lowering = lower_multi_file_modular_with_segments(
        &module_programs,
        &all_decls,
        &all_type_results,
        &all_expr_type_results,
        &reusable_segments,
        &segment_reuse_candidates,
    )
    .map_err(|e| format!("IR 変換エラー: {e}"))?;
    note_incremental_module_segment_lower_by(lowering.fresh_defn_lower_count);
    let new_segments = lowering.segments;
    let module_order: Vec<String> = cache_entries
        .iter()
        .map(|(mod_name, _)| mod_name.clone())
        .collect();
    let old_segments: Option<Vec<ModuleIrSegments>> = if cache
        .linked_module()
        .is_some_and(|linked| linked.module_order() == module_order)
    {
        cache_entries
            .iter()
            .map(|(mod_name, _)| cache.get(mod_name).map(|entry| entry.ir_segments().clone()))
            .collect()
    } else {
        None
    };
    let final_module =
        if let (Some(old_segments), Some(linked)) = (old_segments, cache.linked_module()) {
            if can_patch_linked_module(cache, &module_order, &old_segments, &new_segments) {
                note_incremental_link_cache_hit();
                patch_linked_module(linked.final_module(), &old_segments, &new_segments)
            } else {
                note_incremental_link_full();
                link_module_ir_segments(&new_segments)
            }
        } else {
            note_incremental_link_full();
            link_module_ir_segments(&new_segments)
        };

    for ((mod_name, mut entry), segments) in cache_entries.into_iter().zip(new_segments) {
        entry.set_ir(final_module.clone());
        entry.set_ir_segments(segments);
        cache.insert_module(mod_name, entry);
    }
    cache.set_linked_module(module_order, final_module.clone());

    Ok(final_module)
}

#[cfg(test)]
include!("lib_tests.rs");
